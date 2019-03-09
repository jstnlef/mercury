use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

pub type ProtocolResult<T> = Result<T, ProtocolError>;

#[derive(Debug, PartialEq)]
pub enum ProtocolError {
    EmptyPayload,
    NumberOfFragmentsGreaterThanWindowSize,
    IncompleteMessage,
    PayloadTooLarge(usize, usize),
    InvalidStreamId,
    InvalidConfiguration(&'static str),
}

impl Display for ProtocolError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            ProtocolError::EmptyPayload => write!(f, "Attempted to send an empty buffer."),
            ProtocolError::NumberOfFragmentsGreaterThanWindowSize => {
                write!(f, "Number of fragments required is greater than the window size.")
            }
            ProtocolError::IncompleteMessage => {
                write!(f, "Attempted to peek_size on an incomplete or missing message")
            }
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
