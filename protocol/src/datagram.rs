use crate::guarantees::{DeliveryGuarantee, OrderingGuarantee};
use bytes::{Bytes, BytesMut};
use crc::crc32;
use lazy_static::lazy_static;

/// Represents a request to send a payload (with a particular delivery guarantee) to process.
pub struct Datagram<'a> {
    pub(crate) stream_id: usize,
    pub(crate) delivery: DeliveryGuarantee,
    pub(crate) ordering: OrderingGuarantee,
    pub(crate) payload: &'a [u8],
}

impl<'a> Datagram<'a> {
    /// Essentially bare UDP. May arrive out of order, or not at all. This is best for data
    /// that is unimportant, or data that you send very frequently so even if some datagrams are
    /// missed newer datagrams will compensate.
    pub fn unreliable(payload: &'a [u8]) -> Self {
        Self {
            delivery: DeliveryGuarantee::Unreliable,
            ordering: OrderingGuarantee::None,
            stream_id: 0xFF,
            payload,
        }
    }

    /// Sequenced datagrams are the same as unreliable datagrams, except that only the newest
    /// datagram is ever accepted. Older datagrams are ignored.
    /// e.g. [1, 4, 3, 2, 4] returns [1, 4, 4] to the client.
    pub fn sequenced(payload: &'a [u8], stream_id: usize) -> Self {
        Self {
            delivery: DeliveryGuarantee::Unreliable,
            ordering: OrderingGuarantee::Sequenced,
            stream_id,
            payload,
        }
    }

    /// Reliable datagrams are UDP datagrams monitored by a reliabililty layer to ensure they arrive
    /// at the destination. Prevents duplication.
    /// e.g. [1, 4, 3, 2, 4] returns [1, 4, 3, 2] with a smaller chance of losing a datagram.
    pub fn reliable(payload: &'a [u8]) -> Self {
        Self {
            delivery: DeliveryGuarantee::Reliable,
            ordering: OrderingGuarantee::None,
            stream_id: 0xFF,
            payload,
        }
    }

    /// Reliable sequenced datagrams are UDP datagrams monitored by a reliability layer to ensure
    /// they arrive at the destination and are sequenced at the destination. Prevents duplication.
    /// e.g. [1, 4, 3, 2, 4] returns [1, 4] with a smaller chance of losing a datagram.
    pub fn reliable_sequenced(payload: &'a [u8], stream_id: usize) -> Self {
        Self {
            delivery: DeliveryGuarantee::Reliable,
            ordering: OrderingGuarantee::Sequenced,
            stream_id,
            payload,
        }
    }

    /// Reliable ordered datagrams are UDP datagrams monitored by a reliability layer to ensure they
    /// arrive at the destination and are ordered at the destination. Prevents duplication. This
    /// will act similarly to TCP
    /// e.g. [1, 4, 3, 2, 4] returns [1, 2, 3, 4] with a smaller chance of losing a datagram.
    pub fn reliable_ordered(payload: &'a [u8], stream_id: usize) -> Self {
        Self {
            delivery: DeliveryGuarantee::Reliable,
            ordering: OrderingGuarantee::Ordered,
            stream_id,
            payload,
        }
    }
}

pub fn full<T: Into<BytesMut>>(payload: T) -> ProcessedDatagram {
    ProcessedDatagram::Full {
        payload: payload.into(),
    }
}

pub fn fragment<T: Into<BytesMut>>(payload: T) -> ProcessedDatagram {
    ProcessedDatagram::Fragment {
        payload: payload.into(),
    }
}

pub enum ProcessedDatagram {
    Fragment { payload: BytesMut },
    Full { payload: BytesMut },
}

impl ProcessedDatagram {
    pub fn serialize() -> Bytes {
        Bytes::new()
    }
}

lazy_static! {
    static ref PROTOCOL_VERSION: String = format!(
        "{}-{}.{}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION_MAJOR"),
        env!("CARGO_PKG_VERSION_MINOR")
    );
}

fn calc_checksum(payload: &[u8]) -> u32 {
    crc32::checksum_ieee(&[PROTOCOL_VERSION.as_bytes(), payload].concat())
}

#[cfg(test)]
mod test {
    use super::{Datagram, DeliveryGuarantee, OrderingGuarantee};

    fn test_payload() -> &'static [u8] {
        "hello world".as_bytes()
    }

    #[test]
    fn ensure_unreliable_creation() {
        let datagram = Datagram::unreliable(test_payload());
        assert_eq!(datagram.delivery, DeliveryGuarantee::Unreliable);
        assert_eq!(datagram.ordering, OrderingGuarantee::None);
        assert_eq!(datagram.stream_id, 0xFF);
    }

    #[test]
    fn ensure_sequenced_creation() {
        let datagram = Datagram::sequenced(test_payload(), 0);
        assert_eq!(datagram.delivery, DeliveryGuarantee::Unreliable);
        assert_eq!(datagram.ordering, OrderingGuarantee::Sequenced);
        assert_eq!(datagram.stream_id, 0);
    }

    #[test]
    fn ensure_reliable_creation() {
        let datagram = Datagram::reliable(test_payload());
        assert_eq!(datagram.delivery, DeliveryGuarantee::Reliable);
        assert_eq!(datagram.ordering, OrderingGuarantee::None);
        assert_eq!(datagram.stream_id, 0xFF);
    }

    #[test]
    fn ensure_reliable_sequenced_creation() {
        let datagram = Datagram::reliable_sequenced(test_payload(), 0);
        assert_eq!(datagram.delivery, DeliveryGuarantee::Reliable);
        assert_eq!(datagram.ordering, OrderingGuarantee::Sequenced);
        assert_eq!(datagram.stream_id, 0);
    }

    #[test]
    fn ensure_reliable_ordered_creation() {
        let datagram = Datagram::reliable_ordered(test_payload(), 0);
        assert_eq!(datagram.delivery, DeliveryGuarantee::Reliable);
        assert_eq!(datagram.ordering, OrderingGuarantee::Ordered);
        assert_eq!(datagram.stream_id, 0);
    }
}
