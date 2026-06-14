use std::error::Error as StdError;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum Error {
    InvalidInput(&'static str),
    WavDecode(hound::Error),
    WavEncode(hound::Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInput(message) => f.write_str(message),
            Self::WavDecode(err) => write!(f, "failed to decode wav: {err}"),
            Self::WavEncode(err) => write!(f, "failed to encode wav: {err}"),
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::WavDecode(err) | Self::WavEncode(err) => Some(err),
            Self::InvalidInput(_) => None,
        }
    }
}
