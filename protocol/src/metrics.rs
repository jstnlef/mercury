use std::fmt;

/// Stores various metrics information. e.g. number of datagrams/fragments sent, bandwidth
/// calculations, etc
#[derive(Debug)]
pub struct Metrics {
    counters: [u64; DataPoint::Length as usize],
    packet_loss: f32,
    sent_bandwidth_kbps: f32,
    received_bandwidth_kbps: f32,
    acked_bandwidth_kbps: f32,
}

impl Metrics {
    pub fn new() -> Self {
        Self {
            counters: [0; DataPoint::Length as usize],
            packet_loss: 0.0,
            sent_bandwidth_kbps: 0.0,
            received_bandwidth_kbps: 0.0,
            acked_bandwidth_kbps: 0.0,
        }
    }

    // Returns the count of a particular data point.
    pub fn get_count(&self, data_point: DataPoint) -> u64 {
        self.counters[data_point as usize]
    }

    // Increments the value of a particular data point.
    pub(crate) fn increment(&mut self, data_point: DataPoint) {
        self.counters[data_point as usize] += 1;
    }
}

impl fmt::Display for Metrics {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Do useful thing here")
    }
}

#[derive(Copy, Clone, Debug, PartialOrd, PartialEq)]
pub enum DataPoint {
    PacketsSent = 0,
    PacketsReceived = 1,
    PacketsAcked = 2,
    PacketsStale = 3,
    PacketsInvalid = 4,
    PacketsTooLargeToSend = 5,
    PacketsTooLargeToReceive = 6,
    FragmentsSent = 7,
    FragmentsReceived = 8,
    FragmentsInvalid = 9,
    Length = 10,
}

#[cfg(test)]
mod test {
    use super::{DataPoint, Metrics};

    #[test]
    fn can_increment_and_fetch_count() {
        let mut metrics = Metrics::new();
        metrics.increment(DataPoint::PacketsSent);
        assert_eq!(metrics.get_count(DataPoint::PacketsSent), 1);
    }

    #[test]
    fn can_increment_many() {
        let mut metrics = Metrics::new();
        for _ in 0..10 {
            metrics.increment(DataPoint::PacketsReceived);
        }
        assert_eq!(metrics.get_count(DataPoint::PacketsReceived), 10)
    }
}
