mod config;
mod connection;
mod datagram;
mod endpoint;
mod errors;
mod guarantees;
mod metrics;
mod segment;
mod sequence_buffer;
mod streams;

pub use crate::{
    datagram::Datagram,
    endpoint::Endpoint,
    errors::{ProtocolError, ProtocolResult},
};

// no delay min rto
const RTO_NDL: u32 = 30;
// normal min rto
const RTO_MIN: i32 = 100;
const RTO_DEF: i32 = 200;
const RTO_MAX: u32 = 60_000;
// cmd: push data
const CMD_PUSH: u32 = 81;
// cmd: ack
const CMD_ACK: u8 = 82;
// cmd: window probe (ask)
const CMD_WASK: u32 = 83;
// cmd: window size (tell)
const CMD_WINS: u32 = 84;
// need to send KCP_CMD_WASK
const ASK_SEND: u32 = 0b01;
// need to send KCP_CMD_WINS
const ASK_TELL: u32 = 0b10;
const SEND_WINDOW_SIZE: usize = 32;
const RECV_WINDOW_SIZE: usize = 32;
const DEFAULT_MTU: usize = 1_400;
const ACK_FAST: u32 = 3;
const INTERVAL: u64 = 100;
const PROTOCOL_OVERHEAD: usize = 24;
const DEADLINK: u32 = 20;
const THRESH_INIT: u32 = 2;
const HRESH_MIN: u32 = 2;
// 7 secs to probe window size
const PROBE_INIT: u32 = 7_000;
// up to 120 secs to probe window
const PROBE_LIMIT: u32 = 120_000;
