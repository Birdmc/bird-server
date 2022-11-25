use std::ops::Range;
use bird_protocol::{ProtocolReadable, ProtocolSize, ProtocolWritable, VarInt};
use bird_protocol::derive::{ProtocolReadable, ProtocolWritable};

#[derive(ProtocolWritable, ProtocolReadable)]
pub struct Handshake<'a> {
    #[bp(variant = VarInt)]
    pub protocol_version: i32,
    pub server_address: &'a str,
    pub server_port: u16,
    #[bp(variant = VarInt)]
    pub next_state: i32,
}

impl ProtocolSize for Handshake<'_> {
    const SIZE: Range<u32> = (0..0);
}