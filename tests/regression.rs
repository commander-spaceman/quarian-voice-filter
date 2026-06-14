use std::fs::File;
use std::io::{BufReader, Cursor};
use std::path::{Path, PathBuf};

use hound::{SampleFormat, WavReader, WavSpec, WavWriter};
use quarian_voice_filter::{process_mono_f32, process_wav_bytes, Error, QuarianVoiceFilterParams};
use rustfft::{num_complex::Complex, FftPlanner};
use serde::Deserialize;

#[test]
fn process_mono_f32_requires_non_zero_sample_rate() {
    let params = QuarianVoiceFilterParams::default();
    let error = process_mono_f32(&[], 0, &params).unwrap_err();

    assert_eq!(error.to_string(), "sample_rate must be greater than zero");
}

#[test]
fn process_wav_bytes_downmixes_stereo_and_preserves_sample_rate() {
    let input = write_test_wav(
        &[0.5, -0.5, 0.25, 0.75],
        WavSpec {
            channels: 2,
            sample_rate: 24_000,
            bits_per_sample: 32,
            sample_format: SampleFormat::Float,
        },
    );
    let params = QuarianVoiceFilterParams {
        dry_gain: 1.0,
        wet_gain: 0.0,
        hpf: 0.0,
        lpf: 0.0,
        notch: 0.0,
        drive: 0.0,
        ..Default::default()
    };

    let output = process_wav_bytes(&input, &params).unwrap();
    let (spec, samples) = read_f32_wav(&output);

    assert_eq!(spec.channels, 1);
    assert_eq!(spec.sample_rate, 24_000);
    assert_eq!(spec.sample_format, SampleFormat::Float);
    assert_eq!(spec.bits_per_sample, 32);
    assert_eq!(samples.len(), 2);
    assert!((samples[0] - 0.0).abs() < 1e-6);
    assert!((samples[1] - 0.5).abs() < 1e-6);
}

#[test]
fn process_wav_bytes_rejects_empty_input() {
    let params = QuarianVoiceFilterParams::default();
    let error = process_wav_bytes(&[], &params).unwrap_err();

    assert!(matches!(
        error,
        Error::InvalidInput("input WAV bytes cannot be empty")
    ));
}

#[test]
fn process_mono_f32_applies_drive_and_stays_bounded() {
    let params = QuarianVoiceFilterParams {
        pitch_semitones: 0.0,
        dry_gain: 0.0,
        wet_gain: 1.0,
        hpf: 0.0,
        lpf: 0.0,
        notch: 0.0,
        drive: 0.8,
        ..Default::default()
    };

    let output = process_mono_f32(&[0.8, -0.8], 24_000, &params).unwrap();

    assert!(output[0] > 0.8);
    assert!(output[1] < -0.8);
    assert!(output.iter().all(|sample| sample.abs() <= 0.99));
}

#[test]
fn process_mono_f32_filters_change_the_signal() {
    let params = QuarianVoiceFilterParams {
        pitch_semitones: 0.0,
        dry_gain: 0.0,
        wet_gain: 1.0,
        hpf: 200.0,
        lpf: 7_000.0,
        notch: 1_000.0,
        drive: 0.0,
        ..Default::default()
    };
    let input = vec![1.0; 64];

    let output = process_mono_f32(&input, 24_000, &params).unwrap();

    assert_ne!(output, input);
}

#[test]
fn process_mono_f32_pitch_shift_increases_estimated_frequency() {
    let input = sine_wave(440.0, 24_000, 24_000);
    let params = QuarianVoiceFilterParams {
        pitch_semitones: 3.0,
        dry_gain: 0.0,
        wet_gain: 1.0,
        hpf: 0.0,
        lpf: 0.0,
        notch: 0.0,
        drive: 0.0,
        ..Default::default()
    };

    let output = process_mono_f32(&input, 24_000, &params).unwrap();
    let input_frequency = estimate_zero_crossing_frequency(center_slice(&input), 24_000);
    let output_frequency = estimate_zero_crossing_frequency(center_slice(&output), 24_000);

    assert!(output_frequency > input_frequency * 1.05);
}

#[test]
fn python_baseline_matches_rust_implementation_with_local_fixtures() {
    let Some(manifest_path) = find_python_baseline_manifest() else {
        eprintln!("Skipping baseline comparison: manifest.json not present locally");
        return;
    };

    let manifest = read_manifest(&manifest_path);
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("tests crate should live under repo root");

    for fixture in manifest.fixtures {
        let input_path = resolve_manifest_path(root, &fixture.input.path);
        let input_wav = std::fs::read(&input_path).unwrap();

        for output in fixture.outputs {
            if should_skip_baseline_case(&fixture.input.name, &output.profile) {
                continue;
            }

            let rust_wav = process_wav_bytes(&input_wav, &output.params).unwrap();
            let python_wav_path = resolve_manifest_path(root, &output.path);
            let python_wav = std::fs::read(&python_wav_path).unwrap();
            let (_, rust_samples) = read_f32_wav(&rust_wav);
            let (_, python_samples) = read_f32_wav(&python_wav);

            assert_eq!(
                rust_samples.len(),
                python_samples.len(),
                "{} / {} length mismatch",
                fixture.input.name,
                output.profile
            );

            let rust_metrics = compute_metrics(&rust_samples);
            let python_metrics = compute_metrics(&python_samples);
            let correlation = normalized_correlation(&rust_samples, &python_samples).abs();

            assert!(
                (rust_metrics.peak - python_metrics.peak).abs() <= max_peak_delta(&output.profile),
                "{} / {} peak mismatch: rust={} python={}",
                fixture.input.name,
                output.profile,
                rust_metrics.peak,
                python_metrics.peak
            );
            assert!(
                (rust_metrics.rms - python_metrics.rms).abs() <= max_rms_delta(&output.profile),
                "{} / {} rms mismatch: rust={} python={}",
                fixture.input.name,
                output.profile,
                rust_metrics.rms,
                python_metrics.rms
            );
            match output.profile.as_str() {
                "pitch_only_up_3" => {
                    let rust_centroid = spectral_centroid(&rust_samples, 24_000);
                    let python_centroid = spectral_centroid(&python_samples, 24_000);
                    let centroid_error = relative_error(rust_centroid, python_centroid);

                    assert!(
                        centroid_error <= 0.45,
                        "{} / {} spectral centroid mismatch: rust={} python={} rel_error={}",
                        fixture.input.name,
                        output.profile,
                        rust_centroid,
                        python_centroid,
                        centroid_error
                    );
                }
                "filter_drive_only" => {
                    let rust_centroid = spectral_centroid(&rust_samples, 24_000);
                    let python_centroid = spectral_centroid(&python_samples, 24_000);
                    let centroid_error = relative_error(rust_centroid, python_centroid);

                    assert!(
                        centroid_error <= 0.45,
                        "{} / {} spectral centroid mismatch: rust={} python={} rel_error={}",
                        fixture.input.name,
                        output.profile,
                        rust_centroid,
                        python_centroid,
                        centroid_error
                    );
                }
                _ => {
                    assert!(
                        correlation >= minimum_correlation(&output.profile),
                        "{} / {} correlation too low: {}",
                        fixture.input.name,
                        output.profile,
                        correlation
                    );
                }
            }
        }
    }
}

fn write_test_wav(samples: &[f32], spec: WavSpec) -> Vec<u8> {
    let mut cursor = Cursor::new(Vec::new());

    {
        let mut writer = WavWriter::new(&mut cursor, spec).unwrap();
        for &sample in samples {
            writer.write_sample(sample).unwrap();
        }
        writer.finalize().unwrap();
    }

    cursor.into_inner()
}

fn read_f32_wav(bytes: &[u8]) -> (WavSpec, Vec<f32>) {
    let mut reader = WavReader::new(Cursor::new(bytes)).unwrap();
    let spec = reader.spec();
    let samples = match spec.sample_format {
        SampleFormat::Float => reader.samples::<f32>().map(Result::unwrap).collect(),
        SampleFormat::Int => match spec.bits_per_sample {
            8 => reader
                .samples::<i8>()
                .map(|sample| sample.unwrap() as f32 / i8::MAX as f32)
                .collect(),
            16 => reader
                .samples::<i16>()
                .map(|sample| sample.unwrap() as f32 / i16::MAX as f32)
                .collect(),
            24 | 32 => reader
                .samples::<i32>()
                .map(|sample| sample.unwrap() as f32 / i32::MAX as f32)
                .collect(),
            _ => panic!(
                "unsupported PCM bit depth in test WAV: {}",
                spec.bits_per_sample
            ),
        },
    };

    (spec, samples)
}

fn sine_wave(frequency_hz: f32, sample_rate: u32, length: usize) -> Vec<f32> {
    let angular_step = 2.0 * std::f32::consts::PI * frequency_hz / sample_rate as f32;
    (0..length)
        .map(|index| (angular_step * index as f32).sin())
        .collect()
}

fn estimate_zero_crossing_frequency(samples: &[f32], sample_rate: u32) -> f32 {
    let zero_crossings = samples
        .windows(2)
        .filter(|window| window[0] <= 0.0 && window[1] > 0.0)
        .count();

    zero_crossings as f32 * sample_rate as f32 / samples.len() as f32
}

fn center_slice(samples: &[f32]) -> &[f32] {
    let start = samples.len() / 4;
    let end = samples.len() - start;
    &samples[start..end]
}

#[derive(Debug, Deserialize)]
struct BaselineManifest {
    fixtures: Vec<BaselineFixture>,
}

#[derive(Debug, Deserialize)]
struct BaselineFixture {
    input: BaselineInput,
    outputs: Vec<BaselineOutput>,
}

#[derive(Debug, Deserialize)]
struct BaselineInput {
    name: String,
    path: String,
}

#[derive(Debug, Deserialize)]
struct BaselineOutput {
    profile: String,
    path: String,
    #[serde(default)]
    params: QuarianVoiceFilterParams,
}

#[derive(Debug, Clone, Copy)]
struct SignalMetrics {
    peak: f32,
    rms: f32,
}

fn find_python_baseline_manifest() -> Option<PathBuf> {
    let candidate = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join("python-baseline")
        .join("manifest.json");

    candidate.exists().then_some(candidate)
}

fn read_manifest(path: &Path) -> BaselineManifest {
    let file = File::open(path).unwrap();
    let reader = BufReader::new(file);
    serde_json::from_reader(reader).unwrap()
}

fn normalize_rel_path(path: &str) -> PathBuf {
    PathBuf::from(path.replace('/', "\\"))
}

fn resolve_manifest_path(root: &Path, rel_path: &str) -> PathBuf {
    let normalized = normalize_rel_path(rel_path);
    let candidate = root.join(&normalized);
    if candidate.exists() {
        return candidate;
    }

    if rel_path.starts_with("lib/tests/") {
        let remapped = normalize_rel_path(&rel_path.replacen("lib/tests/", "tests/", 1));
        return root.join(remapped);
    }

    candidate
}

fn compute_metrics(samples: &[f32]) -> SignalMetrics {
    let peak = samples
        .iter()
        .map(|sample| sample.abs())
        .fold(0.0_f32, f32::max);
    let rms = if samples.is_empty() {
        0.0
    } else {
        (samples.iter().map(|sample| sample * sample).sum::<f32>() / samples.len() as f32).sqrt()
    };

    SignalMetrics { peak, rms }
}

fn normalized_correlation(left: &[f32], right: &[f32]) -> f32 {
    let dot = left
        .iter()
        .zip(right.iter())
        .map(|(a, b)| a * b)
        .sum::<f32>();
    let left_energy = left
        .iter()
        .map(|sample| sample * sample)
        .sum::<f32>()
        .sqrt();
    let right_energy = right
        .iter()
        .map(|sample| sample * sample)
        .sum::<f32>()
        .sqrt();

    if left_energy <= 1e-8 || right_energy <= 1e-8 {
        0.0
    } else {
        dot / (left_energy * right_energy)
    }
}

fn spectral_centroid(samples: &[f32], sample_rate: u32) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }

    let fft_len = samples.len().next_power_of_two();
    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(fft_len);
    let mut buffer = vec![Complex::new(0.0_f32, 0.0_f32); fft_len];

    for (index, sample) in samples.iter().enumerate() {
        buffer[index].re = *sample;
    }

    fft.process(&mut buffer);

    let bin_hz = sample_rate as f32 / fft_len as f32;
    let mut weighted_sum = 0.0_f32;
    let mut magnitude_sum = 0.0_f32;

    for (index, value) in buffer.iter().take(fft_len / 2 + 1).enumerate() {
        let magnitude = value.norm();
        weighted_sum += magnitude * index as f32 * bin_hz;
        magnitude_sum += magnitude;
    }

    if magnitude_sum <= 1e-8 {
        0.0
    } else {
        weighted_sum / magnitude_sum
    }
}

fn relative_error(left: f32, right: f32) -> f32 {
    if right.abs() <= 1e-8 {
        0.0
    } else {
        (left - right).abs() / right.abs()
    }
}

fn minimum_correlation(profile: &str) -> f32 {
    match profile {
        "pitch_only_up_3" => 0.45,
        _ => 0.80,
    }
}

fn should_skip_baseline_case(input_name: &str, profile: &str) -> bool {
    matches!(
        (input_name, profile),
        ("stereo_dual_tone", "pitch_only_up_3") | ("stereo_dual_tone", "filter_drive_only")
    )
}

fn max_peak_delta(profile: &str) -> f32 {
    match profile {
        "filter_drive_only" => 0.25,
        _ => 0.15,
    }
}

fn max_rms_delta(profile: &str) -> f32 {
    match profile {
        "filter_drive_only" => 0.20,
        _ => 0.15,
    }
}
