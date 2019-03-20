use crate::{
    segment::Segment, ProtocolError, ProtocolResult, CMD_ACK, DEADLINK, DEFAULT_MTU, INTERVAL,
    PROTOCOL_OVERHEAD, RECV_WINDOW_SIZE, RTO_DEF, RTO_MIN, SEND_WINDOW_SIZE, THRESH_INIT, RTO_MAX,
    RTO_NDL, ASK_SEND, ASK_TELL, CMD_PUSH, CMD_WASK, CMD_WINS, PROBE_INIT, PROBE_LIMIT, THRESH_MIN
};
use bytes::{Buf, BufMut, BytesMut};
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

    unacked_send_sequence_num: u32,
    next_send_sequence_num: u32,
    next_recv_sequence_num: u32,

    ssthresh: u32,

    floating_rtt: u32,
    static_rtt: u32,
    calculated_rto: u32,
    minimum_rto: u32,

    send_window_size: usize,
    recv_window_size: usize,
    remote_window_size: usize,
    congestion_window_size: usize,

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

    ack_list: Vec<(u32, u32)>,
    payload_buffer: BytesMut,

    // Number of repeated acks to trigger fast retransmissions
    fast_resend: u32,

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

            unacked_send_sequence_num: 0,
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

            send_queue: VecDeque::with_capacity(SEND_WINDOW_SIZE),
            recv_queue: VecDeque::with_capacity(RECV_WINDOW_SIZE),
            send_buffer: VecDeque::new(),
            recv_buffer: VecDeque::new(),

            // TODO: Need to allocate with capacity
            ack_list: Vec::new(),
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
            return Err(ProtocolError::BufferTooSmall);
        }

        let fast_recover = self.recv_queue.len() >= self.recv_window_size;

        let mut cursor = Cursor::new(buffer);

        // Write the full message data into the buffer.
        while let Some(segment) = self.recv_queue.pop_front() {
            cursor.write_all(&segment.data)?;
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

    /// when you received a low level packet (eg. UDP packet), call it
    pub fn input(&mut self, buffer: &[u8]) -> ProtocolResult<usize> {
        let n = buffer.len();
        let mut cursor = Cursor::new(buffer);

        if cursor.remaining() < PROTOCOL_OVERHEAD {
            return Err(ProtocolError::BufferTooSmall);
        }
        let old_unacked = self.unacked_send_sequence_num;
        let mut flag = false;
        let mut maxack: u32 = 0;
        while cursor.remaining() >= PROTOCOL_OVERHEAD {
            let session_id = cursor.get_u32_be();
            if session_id != self.session_id {
                return Err(ProtocolError::InvalidSessionId);
            }

            let command = cursor.get_u8();
            let fragment_id = cursor.get_u8();
            let window_size = cursor.get_u16_be();
            let timestamp = cursor.get_u32_be();
            let sequence_num = cursor.get_u32_be();
            let unacked_sequence_num = cursor.get_u32_be();
            let len = cursor.get_u32_be() as usize;

            if cursor.remaining() < len {
                return Err(ProtocolError::IncompleteMessage);
            }

            if command != CMD_PUSH && command != CMD_ACK && command != CMD_WASK &&
                command != CMD_WINS
            {
                return Err(ProtocolError::InvalidCommand);
            }

            self.remote_window_size = window_size as usize;
            self.parse_unacked(unacked_sequence_num);
            self.shrink_buffer();
            if command == CMD_ACK {
                let rtt = time_diff(self.current_time, timestamp);
                if rtt >= 0 {
                    self.update_ack(rtt as u32);
                }
                self.parse_ack(sequence_num);
                self.shrink_buffer();
                if !flag {
                    flag = true;
                    maxack = sequence_num;
                } else {
                    if sequence_num > maxack {
                        maxack = sequence_num;
                    }
                }
            } else if command == CMD_PUSH {
                if sequence_num < self.next_recv_sequence_num + self.recv_window_size as u32 {
                    self.ack_list.push((sequence_num, timestamp));
                    if sequence_num >= self.next_recv_sequence_num {
                        let mut segment = Segment::default();
                        segment.session_id = session_id;
                        segment.command = command;
                        segment.fragment_id = fragment_id;
                        segment.window_size = window_size as u16;
                        segment.timestamp = timestamp;
                        segment.sequence_num = sequence_num;
                        segment.unacked_sequence_num = unacked_sequence_num;
                        segment.data.resize(len, 0);
                        cursor.read_exact(&mut segment.data)?;
                        self.parse_data(segment);
                    }
                }
            } else if command == CMD_WASK {
                // ready to send back KCP_CMD_WINS in `flush`
                // tell remote my window size
                self.probe |= ASK_TELL;
            } else if command == CMD_WINS {
                // do nothing
            }
        }

        if flag {
            self.parse_fastack(maxack);
        }

        if self.unacked_send_sequence_num > old_unacked {
            if self.congestion_window_size < self.remote_window_size {
                let mss = self.max_segment_size as u32;
                if self.congestion_window_size < self.ssthresh as usize {
                    self.congestion_window_size += 1;
                    self.incr += mss;
                } else {
                    if self.incr < mss {
                        self.incr = mss;
                    }
                    self.incr += (mss * mss) / self.incr + (mss / 16);
                    if (self.congestion_window_size + 1) as u32 * mss <= self.incr {
                        self.congestion_window_size += 1;
                    }
                }
                if self.congestion_window_size > self.remote_window_size {
                    self.congestion_window_size = self.remote_window_size;
                    self.incr = self.remote_window_size as u32 * mss;
                }
            }
        }
        Ok(n - cursor.remaining())
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
        for segment in self.recv_queue.iter() {
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
                    cursor.read_exact(&mut segment.data[old_len..new_len])?;
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
            return Err(ProtocolError::FragmentsGreaterThanWindowSize);
        }

        if num_fragments == 0 {
            num_fragments = 1
        }

        // Handle fragmentation if we're not in streaming mode.
        for i in 0..num_fragments {
            let new_size = cmp::min(self.max_segment_size as usize, cursor.remaining());
            let mut segment = Segment::default();
            segment.data.resize(new_size, 0);
            cursor.read_exact(&mut segment.data)?;
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

    /// fastest: nodelay(1, 20, 2, true)
    /// `nodelay`: 0:disable(default), 1:enable
    /// `interval`: internal update timer interval in millisec, default is 100ms
    /// `resend`: 0:disable fast resend(default), 1:enable fast resend
    /// `use_congestion_control`: true: normal congestion control(default), false: disable congestion control
    pub fn nodelay(&mut self, nodelay: i32, interval: i32, resend: i32, use_congestion_control: bool) {
        if nodelay >= 0 {
            let nodelay = nodelay as u32;
            self.nodelay = nodelay;
            if nodelay > 0 {
                self.minimum_rto = RTO_NDL;
            } else {
                self.minimum_rto = RTO_MIN;
            }
        }
        if interval >= 0 {
            let mut interval = interval as u32;
            if interval > 5000 {
                interval = 5000;
            } else if interval < 10 {
                interval = 10;
            }
            self.interval = interval;
        }
        if resend >= 0 {
            self.fast_resend = resend as u32;
        }
        self.use_congestion_control = use_congestion_control;
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

    fn parse_data(&mut self, segment: Segment) {
        let sn = segment.sequence_num;
        if sn >= self.next_recv_sequence_num + self.recv_window_size as u32 || sn < self.next_recv_sequence_num {
            return;
        }

        let mut repeat = false;
        let mut index: usize = self.recv_buffer.len();
        for seg in self.recv_buffer.iter().rev() {
            if sn == seg.sequence_num {
                repeat = true;
                break;
            } else if sn > seg.sequence_num {
                break;
            }
            index -= 1;
        }

        if !repeat {
            self.recv_buffer.insert(index, segment);
        }

        // move available data from rcv_buf -> rcv_queue
        index = 0;
        let mut queue_len = self.recv_queue.len();
        for seg in self.recv_buffer.iter() {
            if seg.sequence_num == self.next_recv_sequence_num && queue_len < self.recv_window_size as usize {
                queue_len += 1;
                self.next_recv_sequence_num += 1;
                index += 1;
            } else {
                break;
            }
        }
        if index > 0 {
            let new_rcv_buf = self.recv_buffer.split_off(index);
            self.recv_queue.append(&mut self.recv_buffer);
            self.recv_buffer = new_rcv_buf;
        }
    }

    fn update_ack(&mut self, rtt: u32) {
        if self.static_rtt == 0 {
            self.static_rtt = rtt;
            self.floating_rtt = rtt >> 1;
        } else {
            let delta = if rtt > self.static_rtt {
                rtt - self.static_rtt
            } else {
                self.static_rtt - rtt
            };
            self.floating_rtt = (3 * self.floating_rtt + delta) >> 2;
            self.static_rtt = (7 * self.static_rtt + rtt) >> 3;
            if self.static_rtt < 1 {
                self.static_rtt = 1;
            }
        }
        let rto = self.static_rtt + cmp::max(self.interval, 4 * self.floating_rtt);
        self.calculated_rto = bound(self.minimum_rto, rto, RTO_MAX);
    }

    #[inline]
    fn shrink_buffer(&mut self) {
        self.unacked_send_sequence_num = match self.send_buffer.front() {
            Some(segment) => segment.sequence_num,
            None => self.next_send_sequence_num,
        };
    }

    fn parse_ack(&mut self, sequence_num: u32) {
        if sequence_num < self.unacked_send_sequence_num
            || sequence_num >= self.next_send_sequence_num
        {
            return;
        }
        for i in 0..self.send_buffer.len() {
            let segment = &self.send_buffer[i];
            if sequence_num == segment.sequence_num {
                self.send_buffer.remove(i);
                break;
            } else if sequence_num < segment.sequence_num {
                break;
            }
        }
    }

    fn parse_unacked(&mut self, unacked_sequence_num: u32) {
        while let Some(segment) = self.send_buffer.pop_front() {
            if unacked_sequence_num <= segment.sequence_num {
                break;
            }
        }
    }

    fn parse_fastack(&mut self, sequence_num: u32) {
        if sequence_num < self.unacked_send_sequence_num
            || sequence_num >= self.next_send_sequence_num
        {
            return;
        }
        for segment in &mut self.send_buffer {
            if sequence_num < segment.sequence_num {
                break;
            } else if sequence_num != segment.sequence_num {
                segment.fastack += 1;
            }
        }
    }

    // Flushes pending data.
    // TODO: Go over how this works again and refactor if necessary.
    fn flush(&mut self) {
        if !self.update_called {
            return;
        }

        let current = self.current_time;
        let mut lost = false;
        let mut change = false;

        let mut segment = Segment::default();
        segment.session_id = self.session_id;
        segment.command = CMD_ACK;
        segment.window_size = self.num_open_slots_in_recv_queue() as u16;
        segment.unacked_sequence_num = self.next_recv_sequence_num;

        // flush acknowledges
        for (sequence_num, timestamp) in self.ack_list.iter() {
            if self.payload_buffer.remaining_mut() + PROTOCOL_OVERHEAD > self.max_transmission_unit
            {
                // TODO: Write out bytes
                self.payload_buffer.clear();
            }
            segment.sequence_num = *sequence_num;
            segment.timestamp = *timestamp;
            segment.encode(&mut self.payload_buffer);
        }
        self.ack_list.clear();

        // probe window size (if remote window size equals zero)
        if self.remote_window_size == 0 {
            if self.probe_wait == 0 {
                self.probe_wait = PROBE_INIT;
                self.next_probe_time = self.current_time + self.probe_wait;
            } else {
                if time_diff(self.current_time, self.next_probe_time) >= 0 {
                    if self.probe_wait < PROBE_INIT {
                        self.probe_wait = PROBE_INIT;
                    }
                    self.probe_wait += self.probe_wait / 2;
                    if self.probe_wait > PROBE_LIMIT {
                        self.probe_wait = PROBE_LIMIT;
                    }
                    self.next_probe_time = self.current_time + self.probe_wait;
                    self.probe |= ASK_SEND;
                }
            }
        } else {
            self.next_probe_time = 0;
            self.probe_wait = 0;
        }

        // flush window probing commands
        if (self.probe & ASK_SEND) != 0 {
            segment.command = CMD_WASK;
            if self.payload_buffer.remaining_mut() + PROTOCOL_OVERHEAD > self.max_transmission_unit
            {
                // TODO: Write out bytes
                self.payload_buffer.clear();
            }
            segment.encode(&mut self.payload_buffer);
        }

        // flush window probing commands
        if (self.probe & ASK_TELL) != 0 {
            segment.command = CMD_WINS;
            if self.payload_buffer.remaining_mut() + PROTOCOL_OVERHEAD > self.max_transmission_unit
            {
                // TODO: Write out bytes
                self.payload_buffer.clear();
            }
            segment.encode(&mut self.payload_buffer);
        }

        self.probe = 0;

        // calculate window size
        let mut congestion_window_size = cmp::min(self.send_window_size, self.remote_window_size);
        if self.use_congestion_control {
            congestion_window_size = cmp::min(self.congestion_window_size, congestion_window_size);
        }

        // move data from send_queue to send_buffer
        while self.next_send_sequence_num
            < self.unacked_send_sequence_num + congestion_window_size as u32
        {
            if let Some(mut new_segment) = self.send_queue.pop_front() {
                new_segment.session_id = self.session_id;
                new_segment.command = CMD_PUSH;
                new_segment.window_size = segment.window_size;
                new_segment.timestamp = current;
                new_segment.sequence_num = self.next_send_sequence_num;
                self.next_send_sequence_num += 1;
                new_segment.unacked_sequence_num = self.next_recv_sequence_num;
                new_segment.resend_time = current;
                new_segment.rto = self.calculated_rto;
                new_segment.fastack = 0;
                new_segment.xmit = 0;
                self.send_buffer.push_back(new_segment);
            } else {
                break;
            }
        }

        // calculate resent
        let resent = if self.fast_resend > 0 {
            self.fast_resend
        } else {
            u32::max_value()
        };
        let rto_min = if self.nodelay == 0 {
            self.calculated_rto >> 3
        } else {
            0
        };

        // flush data segments
        for buffer_segment in self.send_buffer.iter_mut() {
            let mut need_send = false;
            if buffer_segment.xmit == 0 {
                need_send = true;
                buffer_segment.xmit += 1;
                buffer_segment.rto = self.calculated_rto;
                buffer_segment.resend_time = current + buffer_segment.rto + rto_min;
            } else if time_diff(current, buffer_segment.resend_time) >= 0 {
                need_send = true;
                buffer_segment.xmit += 1;
                self.xmit += 1;
                if self.nodelay == 0 {
                    buffer_segment.rto += self.calculated_rto;
                } else {
                    buffer_segment.rto += self.calculated_rto >> 2;
                }
                buffer_segment.resend_time = current + buffer_segment.rto;
                lost = true;
            } else if buffer_segment.fastack >= resent {
                need_send = true;
                buffer_segment.xmit += 1;
                buffer_segment.fastack = 0;
                buffer_segment.resend_time = current + buffer_segment.rto;
                change = true;
            }

            if need_send {
                buffer_segment.timestamp = current;
                buffer_segment.window_size = segment.window_size;
                buffer_segment.unacked_sequence_num = self.next_recv_sequence_num;

                let len = buffer_segment.data.len();
                let need = PROTOCOL_OVERHEAD + len;

                if self.payload_buffer.remaining_mut() + need > self.max_transmission_unit {
                    // TODO: Need to write here.
                    self.payload_buffer.clear();
                }
                buffer_segment.encode(&mut self.payload_buffer);

                // never used
                // if segment.xmit >= self.dead_link {
                //     self.state = -1;
                // }
            }
        }

        // flush remaining segments
        if self.payload_buffer.remaining_mut() > 0 {
            // TODO: Need to write here.
            self.payload_buffer.clear();
        }

        // update ssthresh
        if change {
            let in_flight = self.next_send_sequence_num - self.unacked_send_sequence_num;
            self.ssthresh = in_flight >> 2;
            if self.ssthresh < THRESH_MIN {
                self.ssthresh = THRESH_MIN;
            }
            self.congestion_window_size = (self.ssthresh + resent) as usize;
            self.incr = (self.congestion_window_size * self.max_segment_size) as u32;
        }

        if lost {
            self.ssthresh = (congestion_window_size >> 2) as u32;
            if self.ssthresh < THRESH_MIN {
                self.ssthresh = THRESH_MIN;
            }
            self.congestion_window_size = 1;
            self.incr = self.max_segment_size as u32;
        }

        if self.congestion_window_size < 1 {
            self.congestion_window_size = 1;
            self.incr = self.max_segment_size as u32;
        }
    }

    // Calculates the number of open slots in the receive queue based on the set recv window size.
    fn num_open_slots_in_recv_queue(&self) -> usize {
        if self.recv_queue.len() < self.recv_window_size {
            self.recv_window_size - self.recv_queue.len()
        } else {
            0
        }
    }
}

#[inline]
fn time_diff(later: u32, earlier: u32) -> i32 {
    later as i32 - earlier as i32
}

#[inline]
fn bound(lower: u32, value: u32, upper: u32) -> u32 {
    cmp::min(cmp::max(lower, value), upper)
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
            ProtocolError::BufferTooSmall
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
