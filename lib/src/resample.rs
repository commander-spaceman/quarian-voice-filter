use rubato::{FftFixedIn, Resampler};

pub fn resample_mono(samples: &[f32], ratio: f32) -> Option<Vec<f32>> {
    if samples.is_empty() || !ratio.is_finite() || ratio <= 0.0 {
        return Some(samples.to_vec());
    }

    let input_rate = 1_000_000_usize;
    let output_rate = (input_rate as f64 * ratio as f64).round() as usize;
    if output_rate == 0 {
        return None;
    }

    let input = vec![samples
        .iter()
        .map(|sample| *sample as f64)
        .collect::<Vec<_>>()];
    let mut input_slices: Vec<&[f64]> = input.iter().map(|channel| channel.as_slice()).collect();
    let mut resampler = FftFixedIn::<f64>::new(input_rate, output_rate, 1024, 2, 1).ok()?;
    let mut output_buffer = vec![vec![0.0_f64; resampler.output_frames_max()]; 1];
    let mut output = Vec::new();

    while input_slices[0].len() >= resampler.input_frames_next() {
        let (consumed, produced) = resampler
            .process_into_buffer(&input_slices, &mut output_buffer, None)
            .ok()?;

        input_slices[0] = &input_slices[0][consumed..];
        output.extend_from_slice(&output_buffer[0][..produced]);
    }

    if !input_slices[0].is_empty() {
        let (_, produced) = resampler
            .process_partial_into_buffer(Some(&input_slices), &mut output_buffer, None)
            .ok()?;
        output.extend_from_slice(&output_buffer[0][..produced]);
    }

    Some(output.into_iter().map(|sample| sample as f32).collect())
}

#[cfg(test)]
mod tests {
    use super::resample_mono;

    #[test]
    fn resample_changes_length_for_ratio() {
        let input = vec![0.0_f32; 4096];
        let output = resample_mono(&input, 1.25).unwrap();

        assert!(output.len() > input.len());
    }
}
