use crate::ProtocolResult;
use bytes::{BufMut, Bytes, BytesMut};
use std::io::Write;

#[derive(Default)]
pub struct Segment {
    pub(crate) conv: u32,
    pub(crate) cmd: u8,
    pub(crate) fragment_id: u8,
    pub(crate) wnd: u16,
    pub(crate) ts: u32,
    pub(crate) sn: u32,
    pub(crate) una: u32,
    pub(crate) resendts: u32,
    pub(crate) rto: u32,
    pub(crate) fastack: u32,
    pub(crate) xmit: u32,
    pub(crate) data: BytesMut,
}

impl Segment {
    pub fn new(&self, data: BytesMut) -> Self {
        Self {
            conv: 0,
            cmd: 0,
            fragment_id: 0,
            wnd: 0,
            ts: 0,
            sn: 0,
            una: 0,
            resendts: 0,
            rto: 0,
            fastack: 0,
            xmit: 0,
            data,
        }
    }

    pub fn encode(&self, buf: &mut BytesMut) {
        buf.put_u32_be(self.conv);
        buf.put_u8(self.cmd);
        buf.put_u8(self.fragment_id);
        buf.put_u16_be(self.wnd);
        buf.put_u32_be(self.ts);
        buf.put_u32_be(self.sn);
        buf.put_u32_be(self.una);
        buf.put_u32_be(self.data.len() as u32);
        buf.put_slice(&self.data);
    }
}