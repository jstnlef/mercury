use crate::{
    config::Config,
    datagram::{ReceiveDatagram, Datagram},
    errors::{ProtocolError, ProtocolResult},
    metrics::Metrics,
};
use bytes::{Bytes, BytesMut};
use std::io;

/// `Endpoint` provides the interface into the protocol handling
pub struct Endpoint {
    config: Config,

    // Congestion Control
    rtt: f32,

    metrics: Metrics,
}

impl Endpoint {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            rtt: 0.0,
            metrics: Metrics::new(),
        }
    }

    /// Process a datagram to send. Returns a Bytes object representing the appropriately serialized
    /// datagram.
    pub fn on_send(&mut self, datagram: Datagram) -> ProtocolResult<Bytes> {
        if datagram.payload.len() > self.config.max_payload_size_bytes() {
            return Err(ProtocolError::PayloadTooLarge);
        }

        Ok(Bytes::new())
    }

    /// Process received data into a datagram
    pub fn on_receive(&mut self, datagram: &[u8]) -> ProtocolResult<ReceiveDatagram> {
        Ok(ReceiveDatagram::Full { payload: "".into() })
    }
}

#[cfg(test)]
mod test {
    use super::{Config, Endpoint, ProtocolError, Datagram};

    #[test]
    fn large_payload_on_send_will_result_in_error() {
        let config = Config::default()
            .with_max_fragments(1)
            .with_fragment_size_bytes(1);
        let mut endpoint = Endpoint::new(config);
        let payload = "Hello world!".as_bytes();
        let datagram = Datagram::unreliable(payload);
        assert_eq!(
            endpoint.on_send(datagram).unwrap_err(),
            ProtocolError::PayloadTooLarge
        );
    }
}
