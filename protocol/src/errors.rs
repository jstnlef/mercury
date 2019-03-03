use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

pub type ProtocolResult<T> = Result<T, ProtocolError>;

#[derive(Debug, PartialEq)]
pub enum ProtocolError {
    PayloadTooLarge(usize, usize),
    InvalidStreamId,
    InvalidConfiguration(&'static str),
}

impl Display for ProtocolError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            ProtocolError::PayloadTooLarge(size, max_size) => write!(
                f,
                "The payload size ({} bytes) was bigger than the max allowed size ({} bytes).",
                size, max_size
            ),
            ProtocolError::InvalidStreamId => write!(f, "The desired stream id is too large."),
            ProtocolError::InvalidConfiguration(s) => write!(f, "Invalid Configuration: {}", s),
        }
    }
}

impl Error for ProtocolError {}
