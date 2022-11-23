use std::ops::Range;
use bird_protocol::{ProtocolReadable, ProtocolSize, ProtocolWritable};
use bird_protocol::derive::{ProtocolReadable, ProtocolWritable};

#[derive(ProtocolWritable, ProtocolReadable)]
pub struct Handshake<'a> {
    pub protocol_version: i32,
    pub server_address: &'a str,
    pub server_port: u16,
    pub next_state: u8,
}

impl ProtocolSize for Handshake<'_> {
    const SIZE: Range<u32> = (0..0);
}