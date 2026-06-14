mod error;
mod params;
mod wav;

pub use crate::error::Error;
pub use crate::params::QuarianVoiceFilterParams;
pub use crate::wav::MonoAudio;

pub fn process_wav_bytes(
    input: &[u8],
    params: &QuarianVoiceFilterParams,
) -> Result<Vec<u8>, Error> {
    let mono = wav::decode_wav_bytes(input)?;
    let processed = process_mono_f32(&mono.samples, mono.sample_rate, params)?;
    wav::encode_wav_bytes(&processed, mono.sample_rate)
}

pub fn process_mono_f32(
    samples: &[f32],
    sample_rate: u32,
    _params: &QuarianVoiceFilterParams,
) -> Result<Vec<f32>, Error> {
    if sample_rate == 0 {
        return Err(Error::InvalidInput("sample_rate must be greater than zero"));
    }

    Ok(samples.to_vec())
}
