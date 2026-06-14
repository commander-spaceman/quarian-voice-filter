use biquad::{Biquad, Coefficients, DirectForm1, Hertz, ToHertz, Type, Q_BUTTERWORTH_F32};

pub fn apply_high_pass(samples: &mut [f32], sample_rate: u32, cutoff_hz: f32) {
    apply_filter(
        samples,
        sample_rate,
        cutoff_hz,
        Type::HighPass,
        Q_BUTTERWORTH_F32,
    );
}

pub fn apply_low_pass(samples: &mut [f32], sample_rate: u32, cutoff_hz: f32) {
    apply_filter(
        samples,
        sample_rate,
        cutoff_hz,
        Type::LowPass,
        Q_BUTTERWORTH_F32,
    );
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
