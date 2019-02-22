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
    pub fn unreliable(payload: &'a [u8]) -> Self {
        Self {
            delivery: DeliveryGuarantee::Unreliable,
            ordering: OrderingGuarantee::None,
            stream_id: 0xFF,
            payload,
        }
    }

    pub fn sequenced(payload: &'a [u8], stream_id: usize) -> Self {
        Self {
            delivery: DeliveryGuarantee::Unreliable,
            ordering: OrderingGuarantee::Sequenced,
            stream_id,
            payload,
        }
    }

    pub fn reliable(payload: &'a [u8]) -> Self {
        Self {
            delivery: DeliveryGuarantee::Reliable,
            ordering: OrderingGuarantee::None,
            stream_id: 0xFF,
            payload,
        }
    }

    pub fn reliable_sequenced(payload: &'a [u8], stream_id: usize) -> Self {
        Self {
            delivery: DeliveryGuarantee::Reliable,
            ordering: OrderingGuarantee::Sequenced,
            stream_id,
            payload,
        }
    }

    pub fn reliable_ordered(payload: &'a [u8], stream_id: usize) -> Self {
        Self {
            delivery: DeliveryGuarantee::Reliable,
            ordering: OrderingGuarantee::Ordered,
            stream_id,
            payload,
        }
    }

    pub fn is_reliable(&self) -> bool {
        self.delivery == DeliveryGuarantee::Reliable
    }

    pub fn is_ordered(&self) -> bool {
        self.ordering == OrderingGuarantee::Ordered
    }

    pub fn is_sequenced(&self) -> bool {
        self.ordering == OrderingGuarantee::Sequenced
    }
}

pub fn full<T: Into<BytesMut>>(payload: T) -> ReceiveDatagram {
    ReceiveDatagram::Full {
        payload: payload.into(),
    }
}

pub fn fragment<T: Into<BytesMut>>(payload: T) -> ReceiveDatagram {
    ReceiveDatagram::Fragment {
        payload: payload.into(),
    }
}

pub enum ReceiveDatagram {
    Fragment { payload: BytesMut },
    Full { payload: BytesMut },
}

impl ReceiveDatagram {
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
        assert_eq!(
            datagram.delivery,
            DeliveryGuarantee::Reliable
        );
        assert_eq!(
            datagram.ordering,
            OrderingGuarantee::Sequenced
        );
        assert_eq!(datagram.stream_id, 0);
    }

    #[test]
    fn ensure_reliable_ordered_creation() {
        let datagram = Datagram::reliable_ordered(test_payload(), 0);
        assert_eq!(
            datagram.delivery,
            DeliveryGuarantee::Reliable
        );
        assert_eq!(
            datagram.ordering,
            OrderingGuarantee::Ordered
        );
        assert_eq!(datagram.stream_id, 0);
    }
}
