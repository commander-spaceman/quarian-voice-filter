mod dsp;
mod error;
mod filters;
mod params;
mod phase_vocoder;
mod pitch;
mod resample;
mod stft;
mod wav;

pub use crate::error::Error;
pub use crate::params::QuarianVoiceFilterParams;
pub use crate::wav::MonoAudio;

pub fn process_wav_bytes(
    input: &[u8],
    params: &QuarianVoiceFilterParams,
) -> Result<Vec<u8>, Error> {
    process_wav_bytes_with_mode(input, params, wav::OutputMode::Preserve)
}

pub fn process_wav_bytes_stereo(
    input: &[u8],
    params: &QuarianVoiceFilterParams,
) -> Result<Vec<u8>, Error> {
    process_wav_bytes_with_mode(input, params, wav::OutputMode::ForceStereo)
}

fn process_wav_bytes_with_mode(
    input: &[u8],
    params: &QuarianVoiceFilterParams,
    output_mode: wav::OutputMode,
) -> Result<Vec<u8>, Error> {
    let mono = wav::decode_wav_bytes(input)?;
    let processed = process_mono_f32(&mono.samples, mono.sample_rate, params)?;
    wav::encode_wav_bytes(&processed, mono.sample_rate, mono.channels, output_mode)
}

pub fn process_mono_f32(
    samples: &[f32],
    sample_rate: u32,
    params: &QuarianVoiceFilterParams,
) -> Result<Vec<f32>, Error> {
    if sample_rate == 0 {
        return Err(Error::InvalidInput("sample_rate must be greater than zero"));
    }

    Ok(dsp::process_mono_f32(samples, sample_rate, params))
}
