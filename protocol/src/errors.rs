use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

pub type ProtocolResult<T> = Result<T, ProtocolError>;

#[derive(Debug, PartialEq)]
pub enum ProtocolError {
    PayloadTooLarge,
    InvalidStreamId,
    InvalidConfiguration(&'static str),
}

impl Display for ProtocolError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            ProtocolError::PayloadTooLarge => {
                write!(f, "The payload size was bigger than the max allowed size.")
            }
            ProtocolError::InvalidStreamId => write!(f, "The desired stream id is too large."),
            ProtocolError::InvalidConfiguration(s) => write!(f, "Invalid Configuration: {}", s),
        }
    }
}

impl Error for ProtocolError {}
