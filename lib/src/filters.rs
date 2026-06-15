use biquad::{Biquad, Coefficients, DirectForm1, Hertz, ToHertz, Type};

const BUTTERWORTH_ORDER_4_Q: [f32; 2] = [0.541_196_1, 1.306_563];

pub fn apply_high_pass(samples: &mut [f32], sample_rate: u32, cutoff_hz: f32) {
    apply_butterworth_order_4(samples, sample_rate, cutoff_hz, Type::HighPass);
}

pub fn apply_low_pass(samples: &mut [f32], sample_rate: u32, cutoff_hz: f32) {
    apply_butterworth_order_4(samples, sample_rate, cutoff_hz, Type::LowPass);
}

pub fn apply_notch(samples: &mut [f32], sample_rate: u32, cutoff_hz: f32, q: f32) {
    apply_filter(samples, sample_rate, cutoff_hz, Type::Notch, q);
}

fn apply_filter(
    samples: &mut [f32],
    sample_rate: u32,
    cutoff_hz: f32,
    filter_type: Type<f32>,
    q: f32,
) {
    let sample_rate_hz = (sample_rate as f32).hz();
    let Some(cutoff_hz) = sanitize_cutoff(cutoff_hz, sample_rate_hz) else {
        return;
    };

    let Ok(coefficients) =
        Coefficients::<f32>::from_params(filter_type, sample_rate_hz, cutoff_hz, q)
    else {
        return;
    };

    let mut filter = DirectForm1::<f32>::new(coefficients);
    for sample in samples {
        *sample = filter.run(*sample);
    }
}

fn apply_butterworth_order_4(
    samples: &mut [f32],
    sample_rate: u32,
    cutoff_hz: f32,
    filter_type: Type<f32>,
) {
    for q in BUTTERWORTH_ORDER_4_Q {
        apply_filter(samples, sample_rate, cutoff_hz, filter_type, q);
    }
}

fn sanitize_cutoff(cutoff_hz: f32, sample_rate_hz: Hertz<f32>) -> Option<Hertz<f32>> {
    if cutoff_hz <= 0.0 {
        return None;
    }

    let nyquist = sample_rate_hz.hz() / 2.0;
    if cutoff_hz >= nyquist {
        return None;
    }

    Some(cutoff_hz.hz())
}

#[cfg(test)]
mod tests {
    use super::{apply_high_pass, apply_low_pass};

    #[test]
    fn high_pass_order_4_strongly_reduces_dc() {
        let mut samples = vec![1.0_f32; 4_096];

        apply_high_pass(&mut samples, 24_000, 200.0);

        let tail_average = samples[3_072..]
            .iter()
            .map(|sample| sample.abs())
            .sum::<f32>()
            / 1_024.0;
        assert!(tail_average < 1e-3);
    }

    #[test]
    fn low_pass_order_4_preserves_low_frequency_energy() {
        let mut samples: Vec<f32> = (0..1024)
            .map(|index| {
                let t = index as f32 / 24_000.0;
                (2.0 * std::f32::consts::PI * 440.0 * t).sin()
            })
            .collect();
        let input_peak = samples
            .iter()
            .copied()
            .fold(0.0_f32, |peak, sample| peak.max(sample.abs()));

        apply_low_pass(&mut samples, 24_000, 7_000.0);

        let output_peak = samples
            .iter()
            .copied()
            .fold(0.0_f32, |peak, sample| peak.max(sample.abs()));
        assert!(output_peak > input_peak * 0.7);
    }
}
