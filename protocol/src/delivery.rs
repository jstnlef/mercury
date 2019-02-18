/// These guarantees are heavily influenced by (if not shamelessly ripped from) the guarantees
/// specified by RakNet "http://www.jenkinssoftware.com/raknet/manual/reliabilitytypes.html"
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum DeliveryGuarantee {
    /// Essentially bare UDP. May arrive out of order, or not at all. This is best for data
    /// that is unimportant, or data that you send very frequently so even if some datagrams are
    /// missed newer datagrams will compensate.
    Unreliable,
    /// Sequenced datagrams are the same as unreliable datagrams, except that only the newest
    /// datagram is ever accepted. Older datagrams are ignored.
    /// e.g. [1, 4, 3, 2, 4] returns [1, 4, 4] to the client.
    Sequenced,
    /// Reliable datagrams are UDP datagrams monitored by a reliabililty layer to ensure they arrive
    /// at the destination. Prevents duplication.
    /// e.g. [1, 4, 3, 2, 4] returns [1, 4, 3, 2] with a smaller chance of losing a datagram.
    Reliable,
    /// Reliable ordered datagrams are UDP datagrams monitored by a reliability layer to ensure they
    /// arrive at the destination and are ordered at the destination. Prevents duplication. This
    /// will act similarly to TCP
    /// e.g. [1, 4, 3, 2, 4] returns [1, 2, 3, 4] with a smaller chance of losing a datagram.
    ReliableOrdered,
    /// Reliable sequenced datagrams are UDP datagrams monitored by a reliability layer to ensure
    /// they arrive at the destination and are sequenced at the destination. Prevents duplication.
    /// e.g. [1, 4, 3, 2, 4] returns [1, 4] with a smaller chance of losing a datagram.
    ReliableSequenced,
}
