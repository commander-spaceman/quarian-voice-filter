use rubato::{
    audioadapter_buffers::direct::SequentialSliceOfVecs, calculate_cutoff, Async, FixedAsync,
    Resampler, SincInterpolationParameters, SincInterpolationType, WindowFunction,
};

const SINC_LEN: usize = 128;
const SINC_OVERSAMPLING_FACTOR: usize = 256;
const RESAMPLER_CHUNK_SIZE: usize = 1024;
const RESAMPLER_CHANNELS: usize = 1;
const RESAMPLER_MAX_RATIO_RELATIVE: f64 = 1.1;

pub fn resample_mono(samples: &[f32], ratio: f32) -> Option<Vec<f32>> {
    if samples.is_empty() || !ratio.is_finite() || ratio <= 0.0 {
        return Some(samples.to_vec());
    }

    let input: Vec<Vec<f64>> = vec![samples.iter().map(|s| *s as f64).collect()];
    let input_adapter = SequentialSliceOfVecs::new(&input, 1, samples.len()).ok()?;

    let window = WindowFunction::Blackman2;
    let params = SincInterpolationParameters {
        sinc_len: SINC_LEN,
        f_cutoff: calculate_cutoff(SINC_LEN, window),
        interpolation: SincInterpolationType::Cubic,
        oversampling_factor: SINC_OVERSAMPLING_FACTOR,
        window,
    };

    let mut resampler = Async::<f64>::new_sinc(
        ratio as f64,
        RESAMPLER_MAX_RATIO_RELATIVE,
        &params,
        RESAMPLER_CHUNK_SIZE,
        RESAMPLER_CHANNELS,
        FixedAsync::Input,
    )
    .ok()?;

    let output_delay = resampler.output_delay();
    let max_output_frames = resampler.output_frames_max();

    let mut outdata = vec![vec![0.0_f64; max_output_frames]];
    let mut output: Vec<f64> = Vec::new();
    let mut input_offset = 0;
    let mut input_frames_left = samples.len();

    while input_frames_left > 0 {
        let input_frames_next = resampler.input_frames_next();

        let partial_len = if input_frames_left < input_frames_next {
            Some(input_frames_left)
        } else {
            None
        };

        let (frames_read, frames_written) = {
            let mut output_adapter =
                SequentialSliceOfVecs::new_mut(&mut outdata, RESAMPLER_CHANNELS, max_output_frames)
                    .ok()?;

            resampler
                .process_into_buffer(
                    &input_adapter,
                    &mut output_adapter,
                    Some(&rubato::Indexing {
                        input_offset,
                        output_offset: 0,
                        active_channels_mask: None,
                        partial_len,
                    }),
                )
                .ok()?
        };

        output.extend_from_slice(&outdata[0][..frames_written]);
        input_offset += frames_read;
        input_frames_left = input_frames_left.saturating_sub(frames_read);
    }

    let output: Vec<f32> = output.into_iter().map(|s| s as f32).collect();
    let expected_len = ((samples.len() as f32) * ratio).ceil().max(1.0) as usize;
    Some(trim_resampler_delay(&output, output_delay, expected_len))
}

fn trim_resampler_delay(samples: &[f32], delay: usize, expected_len: usize) -> Vec<f32> {
    let start = delay.min(samples.len());
    let end = (start + expected_len).min(samples.len());
    let mut trimmed = samples[start..end].to_vec();

    if trimmed.len() < expected_len {
        trimmed.resize(expected_len, 0.0);
    }

    trimmed
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

    #[test]
    fn resample_uses_expected_output_length() {
        let input = vec![0.0_f32; 999];
        let output = resample_mono(&input, 1.25).unwrap();

        assert_eq!(output.len(), 1249);
    }
}
