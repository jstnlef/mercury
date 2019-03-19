use crate::ASK_TELL;
use crate::{
    segment::Segment, ProtocolError, ProtocolResult, CMD_ACK, DEADLINK, DEFAULT_MTU, INTERVAL,
    PROTOCOL_OVERHEAD, RECV_WINDOW_SIZE, RTO_DEF, RTO_MIN, SEND_WINDOW_SIZE, THRESH_INIT,
};
use bytes::{Buf, BytesMut};
use log::debug;
use std::{
    cmp,
    collections::VecDeque,
    io::{Cursor, Read, Write},
    ops::Add,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

pub struct ReliableConnection {
    session_id: u32,
    max_transmission_unit: usize,
    max_segment_size: usize,
    // TODO: This should probably be an enum..
    connection_state: u32,

    send_una: u32,
    next_send_sequence_num: u32,
    next_recv_sequence_num: u32,

    ssthresh: u32,

    floating_rtt: i32,
    static_rtt: i32,
    calculated_rto: i32,
    minimum_rto: i32,

    send_window_size: usize,
    recv_window_size: usize,
    remote_window_size: usize,

    congestion_window_size: u32,
    probe: u32,

    current_time: u32,
    interval: u32,
    next_flush_time: u32,
    update_called: bool,

    xmit: u32,

    nodelay: u32,

    next_probe_time: u32,
    probe_wait: u32,

    // Maximum number of retransmissions
    dead_link: u32,
    incr: u32,

    send_queue: VecDeque<Segment>,
    recv_queue: VecDeque<Segment>,
    send_buffer: VecDeque<Segment>,
    recv_buffer: VecDeque<Segment>,

    //    acklist: Vec<(u32, u32)>,

    payload_buffer: BytesMut,

    // Number of repeated acks to trigger fast retransmissions
    fast_resend: i32,

    use_congestion_control: bool,
    in_streaming_mode: bool,
    //    output: W,
}

impl ReliableConnection {
    pub fn new(session_id: u32) -> Self {
        Self {
            session_id,
            max_transmission_unit: DEFAULT_MTU,
            max_segment_size: DEFAULT_MTU - PROTOCOL_OVERHEAD,
            connection_state: 0,

            send_una: 0,
            next_send_sequence_num: 0,
            next_recv_sequence_num: 0,

            ssthresh: THRESH_INIT,

            floating_rtt: 0,
            static_rtt: 0,
            calculated_rto: RTO_DEF,
            minimum_rto: RTO_MIN,

            send_window_size: SEND_WINDOW_SIZE,
            recv_window_size: RECV_WINDOW_SIZE,
            remote_window_size: RECV_WINDOW_SIZE,
            congestion_window_size: 0,
            probe: 0,

            current_time: 0,
            interval: 0,
            next_flush_time: 0,
            update_called: false,

            xmit: 0,

            nodelay: 0,

            next_probe_time: 0,
            probe_wait: 0,

            dead_link: DEADLINK,
            incr: 0,

            send_queue: VecDeque::new(),
            recv_queue: VecDeque::new(),
            send_buffer: VecDeque::new(),
            recv_buffer: VecDeque::new(),

            //    acklist: Vec<(u32, u32)>,

            payload_buffer: BytesMut::with_capacity((DEFAULT_MTU + PROTOCOL_OVERHEAD) * 3),

            fast_resend: 0,

            use_congestion_control: false,
            in_streaming_mode: false,
        }
    }

    pub fn recv(&mut self, buffer: &mut [u8]) -> ProtocolResult<usize> {
        if self.recv_queue.is_empty() {
            return Err(ProtocolError::EmptyRecvQueue);
        }

        let peek_size = self.peek_size()?;

        if peek_size > buffer.len() {
            return Err(ProtocolError::RecvBufferTooSmall);
        }

        let fast_recover = self.recv_queue.len() >= self.recv_window_size;

        let mut cursor = Cursor::new(buffer);

        // Write the full message data into the buffer.
        while let Some(segment) = self.recv_queue.pop_front() {
            cursor.write_all(&segment.data);
            debug!("Received sequence_num: {}", segment.sequence_num);
            if segment.fragment_id == 0 {
                break;
            }
        }
        assert_eq!(cursor.position() as usize, peek_size);

        // Move available data from recv_buffer -> recv_queue
        while let Some(segment) = self.recv_buffer.pop_front() {
            if segment.sequence_num == self.next_recv_sequence_num
                && self.recv_queue.len() < self.recv_window_size
            {
                self.recv_queue.push_back(segment);
                self.next_recv_sequence_num += 1;
            } else {
                break;
            }
        }

        // fast recover
        if self.recv_queue.len() < self.recv_window_size && fast_recover {
            // ready to send back CMD_WINS in `flush`
            // tell remote my window size
            self.probe |= ASK_TELL;
        }

        Ok(cursor.position() as usize)
    }

    /// Returns the size of the next message in the recv_queue.
    pub fn peek_size(&self) -> ProtocolResult<usize> {
        let segment = match self.recv_queue.front() {
            Some(seg) => seg,
            None => return Err(ProtocolError::IncompleteMessage),
        };

        // If we're in streaming mode or this is the only fragment, just return the length of the
        // data.
        if segment.fragment_id == 0 {
            return Ok(segment.data.len());
        }

        // If the next segment is not found in the queue, something is broken.
        if self.recv_queue.len() < (segment.fragment_id + 1) as usize {
            return Err(ProtocolError::IncompleteMessage);
        }

        let mut size = 0;
        for segment in &self.recv_queue {
            size += segment.data.len();
            if segment.fragment_id == 0 {
                break;
            }
        }

        Ok(size)
    }

    /// Appends a payload to the send queue
    pub fn send(&mut self, payload: &[u8]) -> ProtocolResult<()> {
        if payload.is_empty() {
            return Err(ProtocolError::EmptyPayload);
        }

        let mut cursor = Cursor::new(payload);

        // Append to the previous segment in streaming mode (if possible)
        if self.in_streaming_mode {
            if let Some(segment) = self.send_queue.back_mut() {
                let old_len = segment.data.len();
                if old_len < self.max_segment_size {
                    let new_len = cmp::min(old_len + payload.len(), self.max_segment_size);
                    // TODO: Maybe this should be handled by a method on segment
                    segment.data.resize(new_len, 0);
                    cursor.read_exact(&mut segment.data[old_len..new_len]);
                    segment.fragment_id = 0;
                    if cursor.remaining() == 0 {
                        return Ok(());
                    }
                }
            }
        }

        let mut num_fragments = if cursor.remaining() <= self.max_segment_size {
            1
        } else {
            (cursor.remaining() + self.max_segment_size - 1) / self.max_segment_size
        };

        if num_fragments >= RECV_WINDOW_SIZE {
            return Err(ProtocolError::NumberOfFragmentsGreaterThanWindowSize);
        }

        if num_fragments == 0 {
            num_fragments = 1
        }

        // Handle fragmentation if we're not in streaming mode.
        for i in 0..num_fragments {
            let new_size = cmp::min(self.max_segment_size as usize, cursor.remaining());
            let mut segment = Segment::default();
            segment.data.resize(new_size, 0);
            cursor.read_exact(&mut segment.data);
            segment.fragment_id = (if !self.in_streaming_mode {
                num_fragments - i - 1
            } else {
                0
            }) as u8;
            self.send_queue.push_back(segment);
        }

        Ok(())
    }

    /// Updates state (call it repeatedly, every 10ms-100ms), or you can ask
    /// `check` when to call it again (without `input`/`send` calling).
    pub fn update(&mut self, current: u32) {
        self.current_time = current;
        if !self.update_called {
            self.update_called = true;
            self.next_flush_time = self.current_time;
        }

        let mut time_since = time_diff(self.current_time, self.next_flush_time);

        if time_since >= 10_000 || time_since < -10_000 {
            self.next_flush_time = self.current_time;
            time_since = 0;
        }

        if time_since >= 0 {
            self.next_flush_time += self.interval;
            if time_diff(self.current_time, self.next_flush_time) >= 0 {
                self.next_flush_time = self.current_time + self.interval;
            }
            self.flush();
        }
    }

    /// Determines the time when you should next call `update()`
    pub fn check(&self, current: u32) -> u32 {
        if !self.update_called {
            return current;
        }

        let mut ts_flush = self.next_flush_time;

        let time_delta = time_diff(current, ts_flush);
        if time_delta >= 0 {
            return current;
        }

        if time_delta >= 10_000 || time_delta <= -10_000 {
            ts_flush = current;
        }

        let mut tm_packet = u32::max_value();
        for segment in self.send_buffer.iter() {
            let diff = time_diff(segment.resend_time, current);
            if diff <= 0 {
                return current;
            }
            if (diff as u32) < tm_packet {
                tm_packet = diff as u32;
            }
        }

        let tm_flush = time_diff(ts_flush, current) as u32;
        let minimal = cmp::min(cmp::min(tm_packet, tm_flush), self.interval);

        return current + minimal;
    }

    /// Change MTU size, default is DEFAULT_MTU. This method will also resize the payload_buffer
    /// to 3 times the MTU.
    pub fn set_mtu(&mut self, mtu: usize) -> ProtocolResult<()> {
        // TODO: KCP has this check. Why the 50?
        if mtu < 50 || mtu < PROTOCOL_OVERHEAD {
            return Err(ProtocolError::InvalidConfiguration("MTU too small."));
        }

        self.max_transmission_unit = mtu;
        self.max_segment_size = self.max_transmission_unit - PROTOCOL_OVERHEAD;
        let new_size = (mtu + PROTOCOL_OVERHEAD) * 3;
        self.payload_buffer.resize(new_size, 0);

        Ok(())
    }

    // Sets maximum window sizes: send_window_size=32, recv_window_size=32 by default
    pub fn set_window_sizes(&mut self, send_size: usize, recv_size: usize) {
        self.send_window_size = send_size;
        self.recv_window_size = recv_size;
    }

    // Number of segments waiting to be sent.
    pub fn num_segments_awaiting_send(&self) -> usize {
        self.send_buffer.len() + self.send_queue.len()
    }

    // Flushes pending data.
    fn flush(&mut self) {}

    // Calculates the number of open slots in the receive queue based on the set recv window size.
    fn num_open_slots_in_recv_queue(&self) -> usize {
        if self.recv_queue.len() < self.recv_window_size {
            self.recv_window_size - self.recv_queue.len()
        } else {
            0
        }
    }
}

fn time_diff(later: u32, earlier: u32) -> i32 {
    later as i32 - earlier as i32
}

#[cfg(test)]
mod test {
    use super::{time_diff, ProtocolError, ReliableConnection, Segment};
    use bytes::BytesMut;
    use std::io::Bytes;
    use std::{
        thread,
        time::{Duration, SystemTime},
    };

    #[test]
    fn test_recv_with_empty_queue() {
        let mut connection = ReliableConnection::new(0);
        let mut buffer = Vec::with_capacity(10);
        assert_eq!(
            connection.recv(&mut buffer).unwrap_err(),
            ProtocolError::EmptyRecvQueue
        );
    }

    #[test]
    fn test_recv_with_too_small_buffer() {
        let mut connection = ReliableConnection::new(0);
        let mut buffer = Vec::new();
        connection
            .recv_queue
            .push_back(Segment::new(BytesMut::from("test")));
        assert_eq!(
            connection.recv(&mut buffer).unwrap_err(),
            ProtocolError::RecvBufferTooSmall
        );
    }

    // TODO: Add many more tests around recv

    #[test]
    fn test_open_slots_in_recv_queue() {
        let mut connection = ReliableConnection::new(0);
        assert_eq!(connection.recv_window_size, 32);
        assert_eq!(connection.num_open_slots_in_recv_queue(), 32);
        for _ in 0..32 {
            connection.recv_queue.push_back(Segment::default());
        }

        assert_eq!(connection.num_open_slots_in_recv_queue(), 0);
        connection.set_window_sizes(32, 0);
        assert_eq!(connection.num_open_slots_in_recv_queue(), 0);
    }

    #[test]
    fn test_peek_size() {
        let connection = ReliableConnection::new(0);
        assert_eq!(
            connection.peek_size().unwrap_err(),
            ProtocolError::IncompleteMessage
        );
    }

    // TODO: Add many more tests around peek_size

    #[test]
    fn test_send_with_empty_buffer_throws_error() {
        let mut connection = ReliableConnection::new(0);
        assert_eq!(
            connection.send(&vec![]).unwrap_err(),
            ProtocolError::EmptyPayload
        );
    }

    // TODO: Add many more tests around send

    #[test]
    fn test_set_mtu_error_when_too_small() {
        let mut connection = ReliableConnection::new(0);
        // Errors when too small
        assert_eq!(
            connection.set_mtu(0).unwrap_err(),
            ProtocolError::InvalidConfiguration("MTU too small.")
        );
        assert_eq!(
            connection.set_mtu(49).unwrap_err(),
            ProtocolError::InvalidConfiguration("MTU too small.")
        );
    }

    #[test]
    fn test_set_mtu_resize_when_large_truncate_when_small() {
        let mut connection = ReliableConnection::new(0);
        assert_eq!(connection.payload_buffer.len(), 0);
        assert_eq!(connection.payload_buffer.capacity(), 4272);

        assert!(connection.set_mtu(50).is_ok());
        assert_eq!(connection.max_transmission_unit, 50);
        assert_eq!(connection.max_segment_size, 26);
        assert_eq!(connection.payload_buffer.len(), 222);
        assert_eq!(connection.payload_buffer.capacity(), 4272);

        // Looks like Bytes doubles its buffer when resized.
        assert!(connection.set_mtu(1500).is_ok());
        assert_eq!(connection.max_transmission_unit, 1500);
        assert_eq!(connection.max_segment_size, 1476);
        assert_eq!(connection.payload_buffer.len(), 4572);
        assert_eq!(connection.payload_buffer.capacity(), 8544);
    }

    #[test]
    fn test_time_diff() {
        let t1 = 0;
        let t2 = 200;

        assert_eq!(time_diff(t2, t1), 200);
        assert_eq!(time_diff(t1, t2), -200);
    }

    #[test]
    fn test_check() {
        //        let mut connection = ReliableConnection::new(0);
        //        let current = SystemTime::now();
        //        assert_eq!(connection.check(current), current);
        //        connection.update(current);
        //        assert_eq!(connection.check(current), current + connection.interval);
        //        let current = current + Duration::from_millis(200);
        //        assert_eq!(connection.check(current), current);
    }

    // TODO: Add more tests for check
}
