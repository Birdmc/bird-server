use std::ops::Range;
use bird_protocol::{ProtocolReadable, ProtocolSize, ProtocolWritable, VarInt};
use bird_protocol::derive::{ProtocolReadable, ProtocolSize, ProtocolWritable};

#[derive(ProtocolWritable, ProtocolReadable, ProtocolSize)]
#[bp(ty = i32, variant = VarInt)]
pub enum HandshakeNextState {
    #[bp(value = 1)]
    Status = 1,
    Login
}

#[derive(ProtocolWritable, ProtocolReadable, ProtocolSize)]
pub struct Handshake<'a> {
    #[bp(variant = VarInt)]
    pub protocol_version: i32,
    pub server_address: &'a str,
    pub server_port: u16,
    pub next_state: HandshakeNextState,
}