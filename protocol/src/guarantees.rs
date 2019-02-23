/// These guarantees are heavily influenced by (if not shamelessly ripped from) the guarantees
/// specified by RakNet "http://www.jenkinssoftware.com/raknet/manual/reliabilitytypes.html"

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum DeliveryGuarantee {
    Unreliable,
    Reliable,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum OrderingGuarantee {
    None,
    Ordered,
    Sequenced,
}
