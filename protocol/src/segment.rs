use bytes::{BufMut, Bytes, BytesMut};
use std::time::SystemTime;

pub struct Segment {
    pub(crate) session_id: u32,
    pub(crate) command: u8,
    pub(crate) fragment_id: u8,
    pub(crate) window_size: u16,
    pub(crate) timestamp: u32,
    pub(crate) sequence_num: u32,
    pub(crate) unacked_sequence_num: u32,
    pub(crate) resend_time: u32,
    pub(crate) rto: u32,
    pub(crate) fastack: u32,
    pub(crate) xmit: u32,
    pub(crate) data: BytesMut,
}

impl Default for Segment {
    fn default() -> Self {
        Segment::new(BytesMut::new())
    }
}

impl Segment {
    pub fn new(data: BytesMut) -> Self {
        Self {
            session_id: 0,
            command: 0,
            fragment_id: 0,
            window_size: 0,
            timestamp: 0,
            sequence_num: 0,
            unacked_sequence_num: 0,
            resend_time: 0,
            rto: 0,
            fastack: 0,
            xmit: 0,
            data,
        }
    }

    pub fn encode(&self, buf: &mut BytesMut) {
        buf.put_u32_be(self.session_id);
        buf.put_u8(self.command);
        buf.put_u8(self.fragment_id);
        buf.put_u16_be(self.window_size);
        buf.put_u32_be(self.timestamp);
        buf.put_u32_be(self.sequence_num);
        buf.put_u32_be(self.unacked_sequence_num);
        buf.put_u32_be(self.data.len() as u32);
        buf.put_slice(&self.data);
    }
}
