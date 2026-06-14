pub fn pitch_shift(samples: &[f32], _sample_rate: u32, semitones: f32) -> Vec<f32> {
    if samples.is_empty() || semitones.abs() < f32::EPSILON {
        return samples.to_vec();
    }

    let rate = 2.0_f32.powf(semitones / 12.0);
    if !rate.is_finite() || rate <= 0.0 {
        return samples.to_vec();
    }

    let center = (samples.len().saturating_sub(1)) as f32 / 2.0;
    (0..samples.len())
        .map(|index| {
            let centered_index = index as f32 - center;
            let source_pos = center + centered_index * rate;
            interpolate_linear(samples, source_pos)
        })
        .collect()
}

fn interpolate_linear(samples: &[f32], position: f32) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }

    if position <= 0.0 {
        return samples[0];
    }

    let max_index = samples.len() - 1;
    if position >= max_index as f32 {
        return samples[max_index];
    }

    let left_index = position.floor() as usize;
    let right_index = (left_index + 1).min(max_index);
    let fraction = position - left_index as f32;

    samples[left_index] * (1.0 - fraction) + samples[right_index] * fraction
}

#[cfg(test)]
mod tests {
    use super::pitch_shift;

    #[test]
    fn keeps_original_length() {
        let input = vec![0.0_f32; 2_048];
        let output = pitch_shift(&input, 24_000, 1.0);

        assert_eq!(output.len(), input.len());
    }

    #[test]
    fn zero_shift_is_identity() {
        let input = vec![0.25_f32, -0.5, 0.75, -1.0];
        let output = pitch_shift(&input, 24_000, 0.0);

        assert_eq!(output, input);
    }
}
