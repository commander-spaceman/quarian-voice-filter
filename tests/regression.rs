use std::io::Cursor;

use hound::{SampleFormat, WavReader, WavSpec, WavWriter};
use quarian_voice_filter::{process_mono_f32, process_wav_bytes, Error, QuarianVoiceFilterParams};

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
    let samples = reader.samples::<f32>().map(Result::unwrap).collect();

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
