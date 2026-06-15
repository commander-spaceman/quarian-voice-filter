use rustfft::num_complex::Complex;

use crate::stft::Spectrogram;

pub fn stretch(spectrogram: &Spectrogram, rate: f32) -> Spectrogram {
    if spectrogram.frames.is_empty() || !rate.is_finite() || rate <= 0.0 {
        return spectrogram.clone();
    }

    let frame_len = spectrogram.frames.len();
    let bin_count = spectrogram.frames[0].len();
    let time_steps = time_steps(frame_len, rate);
    let phi_advance = phase_advance(bin_count, spectrogram.n_fft, spectrogram.hop_length);
    let mut phase_acc = phases(&spectrogram.frames[0]);
    let padded_frames = padded_frames(&spectrogram.frames, bin_count);
    let mut stretched = Vec::with_capacity(time_steps.len());

    for step in time_steps {
        let left = step.floor() as usize;
        let right = left + 1;
        let alpha = step.fract();
        let left_frame = &padded_frames[left];
        let right_frame = &padded_frames[right];
        let mut output_frame = Vec::with_capacity(bin_count);

        for bin in 0..bin_count {
            let magnitude =
                magnitude(left_frame[bin]) * (1.0 - alpha) + magnitude(right_frame[bin]) * alpha;
            output_frame.push(Complex::from_polar(magnitude, phase_acc[bin]));

            let delta =
                wrap_phase(phase(right_frame[bin]) - phase(left_frame[bin]) - phi_advance[bin]);
            phase_acc[bin] += phi_advance[bin] + delta;
        }

        stretched.push(output_frame);
    }

    Spectrogram {
        n_fft: spectrogram.n_fft,
        hop_length: spectrogram.hop_length,
        frames: stretched,
    }
}

fn time_steps(frame_len: usize, rate: f32) -> Vec<f32> {
    let mut steps = Vec::new();
    let mut position = 0.0_f32;
    while position < frame_len as f32 {
        steps.push(position);
        position += rate;
    }
    steps
}

fn phase_advance(bin_count: usize, n_fft: usize, hop_length: usize) -> Vec<f32> {
    (0..bin_count)
        .map(|bin| hop_length as f32 * 2.0 * std::f32::consts::PI * bin as f32 / n_fft as f32)
        .collect()
}

fn phases(frame: &[Complex<f32>]) -> Vec<f32> {
    frame.iter().map(|value| phase(*value)).collect()
}

fn padded_frames(frames: &[Vec<Complex<f32>>], bin_count: usize) -> Vec<Vec<Complex<f32>>> {
    let mut padded = frames.to_vec();
    padded.push(vec![Complex::new(0.0_f32, 0.0_f32); bin_count]);
    padded.push(vec![Complex::new(0.0_f32, 0.0_f32); bin_count]);
    padded
}

fn phase(value: Complex<f32>) -> f32 {
    value.arg()
}

fn magnitude(value: Complex<f32>) -> f32 {
    value.norm()
}

fn wrap_phase(value: f32) -> f32 {
    value - 2.0 * std::f32::consts::PI * (value / (2.0 * std::f32::consts::PI)).round()
}

#[cfg(test)]
mod tests {
    use rustfft::num_complex::Complex;

    use super::stretch;
    use crate::stft::Spectrogram;

    #[test]
    fn stretch_changes_frame_count() {
        let frame = vec![Complex::new(1.0_f32, 0.0_f32); 16];
        let spectrogram = Spectrogram {
            n_fft: 32,
            hop_length: 8,
            frames: vec![frame.clone(), frame.clone(), frame.clone(), frame],
        };

        let stretched = stretch(&spectrogram, 0.5);
        assert!(stretched.frames.len() > spectrogram.frames.len());
    }

    #[test]
    fn first_output_frame_keeps_initial_phase() {
        let spectrogram = Spectrogram {
            n_fft: 32,
            hop_length: 8,
            frames: vec![
                vec![Complex::from_polar(1.0_f32, 0.25_f32); 4],
                vec![Complex::from_polar(2.0_f32, 1.0_f32); 4],
            ],
        };

        let stretched = stretch(&spectrogram, 0.5);

        for bin in 0..4 {
            assert!((stretched.frames[0][bin].arg() - 0.25).abs() < 1e-6);
        }
    }
}
