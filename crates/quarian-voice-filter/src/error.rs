use std::error::Error as StdError;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum Error {
    Unsupported(&'static str),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported(message) => f.write_str(message),
        }
    }
}

impl StdError for Error {}
