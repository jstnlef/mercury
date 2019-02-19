use crate::{
    config::Config,
    datagram::{Datagram, ReceiveDatagram},
    delivery::DeliveryGuarantee,
    errors::{ProtocolError, ProtocolResult},
    metrics::Metrics,
    streams::{OrderedStream, SequencedStream},
};
use bytes::{Bytes, BytesMut};
use log::debug;

/// `Endpoint` provides the interface into the protocol handling
pub struct Endpoint {
    config: Config,
    ordered_streams: Box<[OrderedStream]>,
    sequenced_streams: Box<[SequencedStream]>,

    /// Congestion Control
    rtt: f32,

    /// Metrics tracking around `Endpoint` operations
    metrics: Metrics,
}

impl Endpoint {
    pub fn new(config: Config) -> Self {
        let ordered_size = config.ordered_streams_size();
        let sequenced_size = config.sequenced_streams_size();
        Self {
            config,
            ordered_streams: vec![OrderedStream::new(); ordered_size].into_boxed_slice(),
            sequenced_streams: vec![SequencedStream::new(); sequenced_size].into_boxed_slice(),
            rtt: 0.0,
            metrics: Metrics::new(),
        }
    }

    /// Process a datagram to send. Returns a Bytes object representing the appropriately serialized
    /// datagram.
    pub fn send(&mut self, datagram: Datagram) -> ProtocolResult<Bytes> {
        if datagram.is_reliable() {
            self.handle_reliable_send(&datagram)
        } else {
            self.handle_unreliable_send(&datagram)
        }
    }

    /// Process received data into a datagram
    pub fn receive(&mut self, datagram: &[u8]) -> ProtocolResult<ReceiveDatagram> {
        Ok(ReceiveDatagram::Full { payload: "".into() })
    }

    fn handle_reliable_send(&mut self, datagram: &Datagram) -> ProtocolResult<Bytes> {
        if datagram.payload.len() > self.config.max_payload_size_bytes() {
            return Err(ProtocolError::PayloadTooLarge);
        }

        let stream_id = datagram.stream_id;
        if datagram.is_sequenced() {
            if stream_id >= self.sequenced_streams.len() {
                return Err(ProtocolError::InvalidStreamId);
            }
            let stream: &SequencedStream = &self.sequenced_streams[stream_id];

            Ok(Bytes::new())
        } else if datagram.is_ordered() {
            if stream_id >= self.ordered_streams.len() {
                return Err(ProtocolError::InvalidStreamId);
            }
            let stream: &OrderedStream = &self.ordered_streams[stream_id];

            Ok(Bytes::new())
        } else {
            Ok(Bytes::new())
        }
    }

    fn handle_unreliable_send(&mut self, datagram: &Datagram) -> ProtocolResult<Bytes> {
        Ok(Bytes::new())
    }
}

#[cfg(test)]
mod test {
    use super::{Config, Datagram, Endpoint, ProtocolError};

    #[test]
    fn error_on_large_payload_for_reliable_send() {
        let config = Config::default()
            .with_max_fragments(1)
            .with_fragment_size_bytes(1);
        let mut endpoint = Endpoint::new(config);
        let payload = "Hello world!".as_bytes();
        let datagram = Datagram::reliable(payload);
        assert_eq!(
            endpoint.send(datagram).unwrap_err(),
            ProtocolError::PayloadTooLarge
        );
    }

    #[test]
    fn error_on_invalid_stream_id_ordered() {
        let config = Config::default();
        let mut endpoint = Endpoint::new(config);
        let payload = "Hello world!".as_bytes();
        let datagram = Datagram::reliable_ordered(payload, 2);
        assert_eq!(
            endpoint.send(datagram).unwrap_err(),
            ProtocolError::InvalidStreamId
        );
    }

    #[test]
    fn error_on_invalid_stream_id_sequenced() {
        let config = Config::default();
        let mut endpoint = Endpoint::new(config);
        let payload = "Hello world!".as_bytes();
        let datagram = Datagram::reliable_sequenced(payload, 2);
        assert_eq!(
            endpoint.send(datagram).unwrap_err(),
            ProtocolError::InvalidStreamId
        );
    }
}
