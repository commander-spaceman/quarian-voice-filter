use crate::filters;
use crate::pitch;
use crate::QuarianVoiceFilterParams;

const MAX_OUTPUT_PEAK: f32 = 0.99;
const NOTCH_Q: f32 = 30.0;

pub fn process_mono_f32(
    samples: &[f32],
    sample_rate: u32,
    params: &QuarianVoiceFilterParams,
) -> Vec<f32> {
    let mut wet = if params.pitch_semitones.abs() > f32::EPSILON {
        pitch::pitch_shift(samples, sample_rate, params.pitch_semitones)
    } else {
        samples.to_vec()
    };

    if params.hpf > 0.0 {
        filters::apply_high_pass(&mut wet, sample_rate, params.hpf);
    }

    if params.lpf > 0.0 {
        filters::apply_low_pass(&mut wet, sample_rate, params.lpf);
    }

    if params.notch > 0.0 {
        filters::apply_notch(&mut wet, sample_rate, params.notch, NOTCH_Q);
    }

    if params.drive > 0.0 {
        apply_drive(&mut wet, params.drive);
    }

    let mut output = mix_dry_wet(samples, &wet, params.dry_gain, params.wet_gain);
    normalize_peak(&mut output, MAX_OUTPUT_PEAK);
    output
}

fn apply_drive(samples: &mut [f32], drive: f32) {
    let scale = 1.0 + drive * 4.0;
    for sample in samples {
        *sample = (*sample * scale).tanh();
    }
}

fn mix_dry_wet(dry: &[f32], wet: &[f32], dry_gain: f32, wet_gain: f32) -> Vec<f32> {
    dry.iter()
        .zip(wet.iter())
        .map(|(dry_sample, wet_sample)| dry_sample * dry_gain + wet_sample * wet_gain)
        .collect()
}

fn normalize_peak(samples: &mut [f32], max_peak: f32) {
    let peak = samples
        .iter()
        .map(|sample| sample.abs())
        .fold(0.0_f32, f32::max);

    if peak > max_peak {
        let scale = max_peak / peak;
        for sample in samples {
            *sample *= scale;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::process_mono_f32;
    use crate::QuarianVoiceFilterParams;

    #[test]
    fn uses_dry_path_when_wet_gain_is_zero() {
        let samples = vec![0.25, -0.5, 0.75];
        let params = QuarianVoiceFilterParams {
            dry_gain: 1.0,
            wet_gain: 0.0,
            ..Default::default()
        };

        let output = process_mono_f32(&samples, 24_000, &params);
        assert_eq!(output, samples);
    }

    #[test]
    fn normalizes_when_mix_exceeds_peak_limit() {
        let samples = vec![1.0, -1.0];
        let params = QuarianVoiceFilterParams {
            dry_gain: 1.0,
            wet_gain: 1.0,
            hpf: 0.0,
            lpf: 0.0,
            notch: 0.0,
            drive: 0.0,
            ..Default::default()
        };

        let output = process_mono_f32(&samples, 24_000, &params);
        let peak = output
            .iter()
            .map(|sample| sample.abs())
            .fold(0.0_f32, f32::max);

        assert!((peak - 0.99).abs() < 1e-6);
    }

    #[test]
    fn pitch_shift_changes_waveform_when_enabled() {
        let input = sine_wave(440.0, 24_000, 2_048);
        let params = QuarianVoiceFilterParams {
            pitch_semitones: 3.0,
            dry_gain: 0.0,
            wet_gain: 1.0,
            hpf: 0.0,
            lpf: 0.0,
            notch: 0.0,
            drive: 0.0,
        };

        let output = process_mono_f32(&input, 24_000, &params);

        assert_eq!(output.len(), input.len());
        assert_ne!(output, input);
    }

    fn sine_wave(frequency_hz: f32, sample_rate: u32, length: usize) -> Vec<f32> {
        let angular_step = 2.0 * std::f32::consts::PI * frequency_hz / sample_rate as f32;
        (0..length)
            .map(|index| (angular_step * index as f32).sin())
            .collect()
    }
}
