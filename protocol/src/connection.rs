use crate::{
    ProtocolError,
    ProtocolResult,
    DEADLINK, INTERVAL, DEFAULT_MTU, PROTOCOL_OVERHEAD, RTO_DEF, RTO_MIN, THRESH_INIT, RECV_WINDOW_SIZE, SEND_WINDOW_SIZE,
    segment::Segment
};
use bytes::{
    Buf,
    BytesMut
};
use std::{
    collections::VecDeque,
    cmp,
    io::{Cursor, Read}
};

pub struct ReliableConnection {
    conv: u32,
    mtu: usize,
    max_segment_size: usize,
    state: u32,

    send_una: u32,
    send_nxt: u32,
    recv_nxt: u32,

    ts_recent: u32,
    ts_lastack: u32,
    ssthresh: u32,

    rx_rttval: i32,
    rx_srtt: i32,
    rx_rto: i32,
    rx_minrto: i32,

    send_window: usize,
    recv_window: usize,
    rmt_window: usize,

    cwnd: u32,
    probe: u32,

    current: u32,
    interval: u32,
    ts_flush: u32,
    xmit: u32,

    nodelay: u32,
    updated: u32,

    ts_probe: u32,
    probe_wait: u32,

    dead_link: u32,
    incr: u32,

    send_queue: VecDeque<Segment>,
    recv_queue: VecDeque<Segment>,
    send_buffer: VecDeque<Segment>,
    recv_buffer: VecDeque<Segment>,

    //    acklist: Vec<(u32, u32)>,

    // user: String,
    payload_buffer: BytesMut,

    fast_resend: i32,

    nocwnd: i32,
    in_streaming_mode: bool,
    //    output: W,
}

impl ReliableConnection {
    pub fn new(conv: u32) -> Self {
        Self {
            conv,
            mtu: DEFAULT_MTU,
            max_segment_size: DEFAULT_MTU - PROTOCOL_OVERHEAD,
            state: 0,

            send_una: 0,
            send_nxt: 0,
            recv_nxt: 0,

            ts_recent: 0,
            ts_lastack: 0,
            ssthresh: THRESH_INIT,

            rx_rttval: 0,
            rx_srtt: 0,
            rx_rto: RTO_DEF,
            rx_minrto: RTO_MIN,

            send_window: SEND_WINDOW_SIZE,
            recv_window: RECV_WINDOW_SIZE,
            rmt_window: RECV_WINDOW_SIZE,
            cwnd: 0,
            probe: 0,

            current: 0,
            interval: INTERVAL,
            ts_flush: INTERVAL,
            xmit: 0,

            nodelay: 0,
            updated: 0,

            ts_probe: 0,
            probe_wait: 0,

            dead_link: DEADLINK,
            incr: 0,

            send_queue: VecDeque::new(),
            recv_queue: VecDeque::new(),
            send_buffer: VecDeque::new(),
            recv_buffer: VecDeque::new(),

            //    acklist: Vec<(u32, u32)>,

            // user: String,
            payload_buffer: BytesMut::with_capacity((DEFAULT_MTU + PROTOCOL_OVERHEAD) * 3),

            fast_resend: 0,

            nocwnd: 0,
            in_streaming_mode: false,
        }
    }

    pub fn recv(&mut self, payload: &[u8]) -> ProtocolResult<usize> {
        Ok(0)
    }

    /// Returns the size of the next message in the recv_queue.
    pub fn peek_size(&self) -> ProtocolResult<usize> {
        let segment = match self.recv_queue.front() {
            Some(seg) => seg,
            None => return Err(ProtocolError::IncompleteMessage)
        };

        // If we're in streaming mode or this is the only fragment, just return the length of the
        // data.
        if segment.fragment_id == 0 {
            return Ok(segment.data.len());
        }

        // If the next segment is not found in the queue, something is broken.
        if self.recv_queue.len() < (segment.fragment_id + 1) as usize {
            return Err(ProtocolError::IncompleteMessage)
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
            return Err(ProtocolError::NumberOfFragmentsGreaterThanWindowSize)
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
            segment.fragment_id = (if !self.in_streaming_mode { num_fragments - i - 1 } else { 0 }) as u8;
            self.send_queue.push_back(segment);
        }

        Ok(())
    }

    /// Change MTU size, default is DEFAULT_MTU. This method will also resize the payload_buffer
    /// to 3 times the MTU.
    pub fn set_mtu(&mut self, mtu: usize) -> ProtocolResult<()> {
        // TODO: KCP has this check. Why the 50?
        if mtu < 50 || mtu < PROTOCOL_OVERHEAD {
            return Err(ProtocolError::InvalidConfiguration("MTU too small."));
        }

        self.mtu = mtu;
        self.max_segment_size = self.mtu - PROTOCOL_OVERHEAD;
        let new_size = (mtu + PROTOCOL_OVERHEAD) * 3;
        self.payload_buffer.resize(new_size, 0);

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::{ProtocolError, ReliableConnection};

    #[test]
    fn test_peek_size() {
        let mut connection = ReliableConnection::new(0);
        assert_eq!(
            connection.peek_size().unwrap_err(),
            ProtocolError::IncompleteMessage
        );
    }

    // TODO: Add many more tests around peek_size

    #[test]
    fn send_with_empty_buffer_throws_error() {
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
        assert_eq!(connection.set_mtu(0).unwrap_err(), ProtocolError::InvalidConfiguration("MTU too small."));
        assert_eq!(connection.set_mtu(49).unwrap_err(), ProtocolError::InvalidConfiguration("MTU too small."));
    }

    #[test]
    fn test_set_mtu_resize_when_large_truncate_when_small() {
        let mut connection = ReliableConnection::new(0);
        assert_eq!(connection.payload_buffer.len(), 0);
        assert_eq!(connection.payload_buffer.capacity(), 4272);

        assert!(connection.set_mtu(50).is_ok());
        assert_eq!(connection.mtu, 50);
        assert_eq!(connection.max_segment_size, 26);
        assert_eq!(connection.payload_buffer.len(), 222);
        assert_eq!(connection.payload_buffer.capacity(), 4272);

        // Looks like Bytes doubles its buffer when resized.
        assert!(connection.set_mtu(1500).is_ok());
        assert_eq!(connection.mtu, 1500);
        assert_eq!(connection.max_segment_size, 1476);
        assert_eq!(connection.payload_buffer.len(), 4572);
        assert_eq!(connection.payload_buffer.capacity(), 8544);
    }
}
