#[derive(Copy, Clone, Debug, PartialEq)]
pub enum DeliveryGuarantee {
    Unreliable,
    Reliable
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum OrderingGuarantee {
    None,
    Ordered,
    Sequenced
}
