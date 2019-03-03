use std::fmt;

/// This is a direct port of the bandwidth calculations found in
/// https://github.com/networkprotocol/reliable.io
macro_rules! calc_bandwidth {
    ($prop_name: expr, $bytes: ident, $time: ident, $smoothing: expr) => {
        let current_bandwidth = (($bytes as f64 / $time) * 8.0 / 1000.0) as f32;
        if ($prop_name - current_bandwidth).abs() > 0.00001 {
            $prop_name += (current_bandwidth - $prop_name) * $smoothing;
        } else {
            $prop_name = current_bandwidth;
        }
    };
}

/// Stores various metrics information. e.g. number of datagrams/fragments sent, bandwidth
/// calculations, etc
#[derive(Debug)]
pub struct Metrics {
    counters: [u64; DataPoint::Length as usize],
    packet_loss: f32,
    sent_bandwidth_kbps: f32,
    received_bandwidth_kbps: f32,
    acked_bandwidth_kbps: f32,

    // Config values to tweak the calculated fields
    bandwidth_smoothing_factor: f32
}

impl Metrics {
    pub fn new(bandwidth_smoothing_factor: f32) -> Self {
        Self {
            counters: [0; DataPoint::Length as usize],
            packet_loss: 0.0,
            sent_bandwidth_kbps: 0.0,
            received_bandwidth_kbps: 0.0,
            acked_bandwidth_kbps: 0.0,
            bandwidth_smoothing_factor
        }
    }

    // Returns the count of a particular data point.
    pub fn get_count(&self, data_point: DataPoint) -> u64 {
        self.counters[data_point as usize]
    }

    // Returns the calculated sent_bandwidth_kbps
    pub fn sent_bandwidth_kbps(&self) -> f32 {
        self.sent_bandwidth_kbps
    }

    // Returns the calculated received_bandwidth_kbps
    pub fn received_bandwidth_kbps(&self) -> f32 {
        self.received_bandwidth_kbps
    }

    // Returns the calculated acked_bandwidth_kbps
    pub fn acked_bandwidth_kbps(&self) -> f32 {
        self.acked_bandwidth_kbps
    }

    // Increments the value of a particular data point.
    pub(crate) fn increment(&mut self, data_point: DataPoint) {
        self.counters[data_point as usize] += 1;
    }

    // Calculate the sent bandwidth given the bytes sent and the time_delta_ms.
    pub(crate) fn calculate_sent_bandwidth(&mut self, bytes_sent: usize, time_delta_ms: f64) {
        calc_bandwidth!(self.sent_bandwidth_kbps, bytes_sent, time_delta_ms, self.bandwidth_smoothing_factor);
    }

    // Calculate the received bandwidth given the bytes received and the time_delta_ms.
    pub(crate) fn calculate_receive_bandwidth(&mut self, bytes_received: usize, time_delta_ms: f64) {
        calc_bandwidth!(self.received_bandwidth_kbps, bytes_received, time_delta_ms, self.bandwidth_smoothing_factor);
    }

    // Calculate the acked bandwidth given the bytes acked and the time_delta_ms.
    pub(crate) fn calculate_acked_bandwidth(&mut self, bytes_acked: usize, time_delta_ms: f64) {
        calc_bandwidth!(self.acked_bandwidth_kbps, bytes_acked, time_delta_ms, self.bandwidth_smoothing_factor);
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
    FragmentsSent = 6,
    FragmentsReceived = 7,
    FragmentsInvalid = 8,
    Length = 9,
}

#[cfg(test)]
mod test {
    use super::{DataPoint, Metrics};

    #[test]
    fn can_increment_and_fetch_count() {
        let mut metrics = Metrics::new(0.1);
        metrics.increment(DataPoint::PacketsSent);
        assert_eq!(metrics.get_count(DataPoint::PacketsSent), 1);
    }

    #[test]
    fn can_increment_many() {
        let mut metrics = Metrics::new(0.1);
        for _ in 0..10 {
            metrics.increment(DataPoint::PacketsReceived);
        }
        assert_eq!(metrics.get_count(DataPoint::PacketsReceived), 10)
    }

    #[test]
    fn test_calc_sent_bandwidth() {
        let mut metrics = Metrics::new(0.1);
        // Run it a few hundred times since we smooth up to the bandwidth value
        for _ in 0..1000 {
            metrics.calculate_sent_bandwidth(1000, 1.0);
        }

        assert_eq!(metrics.sent_bandwidth_kbps(), 8.0)
    }

    #[test]
    fn test_calc_received_bandwidth() {
        let mut metrics = Metrics::new(0.1);
        // Run it a few hundred times since we smooth up to the bandwidth value
        for _ in 0..1000 {
            metrics.calculate_receive_bandwidth(1000, 1.0);
        }

        assert_eq!(metrics.received_bandwidth_kbps(), 8.0)
    }

    #[test]
    fn test_calc_acked_bandwidth() {
        let mut metrics = Metrics::new(0.1);
        // Run it a few hundred times since we smooth up to the bandwidth value
        for _ in 0..1000 {
            metrics.calculate_acked_bandwidth(1000, 1.0);
        }

        assert_eq!(metrics.acked_bandwidth_kbps(), 8.0)
    }
}
