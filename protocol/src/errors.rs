use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    io,
};

pub type ProtocolResult<T> = Result<T, ProtocolError>;

#[derive(Debug)]
pub enum ProtocolError {
    EmptyPayload,
    FragmentsGreaterThanWindowSize,
    IncompleteMessage,
    EmptyRecvQueue,
    BufferTooSmall,
    InvalidSessionId,
    InvalidCommand,
    IOError(io::Error),

    PayloadTooLarge(usize, usize),
    InvalidStreamId,
    InvalidConfiguration(&'static str),
}

impl Display for ProtocolError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            ProtocolError::EmptyPayload => write!(f, "Attempted to send an empty buffer."),
            ProtocolError::FragmentsGreaterThanWindowSize => write!(
                f,
                "Number of fragments required is greater than the window size."
            ),
            ProtocolError::IncompleteMessage => write!(
                f,
                "Attempted to peek_size on an incomplete or missing message"
            ),
            ProtocolError::EmptyRecvQueue => {
                write!(f, "Attempted to recv when the recv_queue is empty.")
            }
            ProtocolError::BufferTooSmall => write!(
                f,
                "Attempted to recv with a buffer too small to hold the payload."
            ),
            ProtocolError::InvalidSessionId => write!(f, "Session id doesn't match."),
            ProtocolError::InvalidCommand => write!(f, "Unrecognized command."),
            ProtocolError::IOError(e) => write!(f, "An IO Error occurred. Reason: {:?}.", e),
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

impl From<io::Error> for ProtocolError {
    fn from(inner: io::Error) -> ProtocolError {
        ProtocolError::IOError(inner)
    }
}

impl PartialEq for ProtocolError {
    fn eq(&self, other: &ProtocolError) -> bool {
        match (self, other) {
            (ProtocolError::EmptyPayload, ProtocolError::EmptyPayload) => true,
            (
                ProtocolError::FragmentsGreaterThanWindowSize,
                ProtocolError::FragmentsGreaterThanWindowSize,
            ) => true,
            (ProtocolError::IncompleteMessage, ProtocolError::IncompleteMessage) => true,
            (ProtocolError::EmptyRecvQueue, ProtocolError::EmptyRecvQueue) => true,
            (ProtocolError::BufferTooSmall, ProtocolError::BufferTooSmall) => true,
            (ProtocolError::PayloadTooLarge(_, _), ProtocolError::PayloadTooLarge(_, _)) => true,
            (ProtocolError::InvalidStreamId, ProtocolError::InvalidStreamId) => true,
            (ProtocolError::InvalidConfiguration(_), ProtocolError::InvalidConfiguration(_)) => {
                true
            }
            (ProtocolError::IOError(_), ProtocolError::IOError(_)) => true,
            (_, _) => false,
        }
    }
}
