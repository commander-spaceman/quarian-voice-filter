mod error;
mod params;

pub use crate::error::Error;
pub use crate::params::QuarianVoiceFilterParams;

pub fn process_wav_bytes(
    _input: &[u8],
    _params: &QuarianVoiceFilterParams,
) -> Result<Vec<u8>, Error> {
    Err(Error::Unsupported(
        "process_wav_bytes is not implemented yet",
    ))
}

pub fn process_mono_f32(
    _samples: &[f32],
    _sample_rate: u32,
    _params: &QuarianVoiceFilterParams,
) -> Result<Vec<f32>, Error> {
    Err(Error::Unsupported(
        "process_mono_f32 is not implemented yet",
    ))
}
