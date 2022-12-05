use std::borrow::Cow;
use std::ops::Range;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use bird_chat::component::Component;
use bird_chat::identifier::Identifier;
use bird_protocol::{*, ProtocolPacketState::*, ProtocolPacketBound::*};
use bird_protocol::derive::{ProtocolAll, ProtocolPacket, ProtocolReadable, ProtocolSize, ProtocolWritable};

#[derive(ProtocolAll, Clone, Copy, PartialEq, Debug)]
#[bp(ty = i32, variant = VarInt)]
pub enum HandshakeNextState {
    #[bp(value = 1)]
    Status = 1,
    Login,
}

#[derive(ProtocolAll, ProtocolPacket, Clone, Copy, PartialEq, Debug)]
#[bp(id = 0x0, state = Handshake, bound = Server)]
pub struct Handshake<'a> {
    #[bp(variant = VarInt)]
    pub protocol_version: i32,
    pub server_address: &'a str,
    pub server_port: u16,
    pub next_state: HandshakeNextState,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct StatusResponseObject<'a> {
    #[serde(borrow)]
    pub version: StatusResponseVersion<'a>,
    #[serde(borrow)]
    pub players: StatusResponsePlayers<'a>,
    #[serde(borrow)]
    pub description: either::Either<&'a str, Component<'a>>,
    #[serde(borrow)]
    pub favicon: Option<&'a str>,
    #[serde(default)]
    pub previews_chat: bool,
    #[serde(default)]
    pub enforces_secure_chat: bool,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
pub struct StatusResponseVersion<'a> {
    #[serde(borrow)]
    pub name: &'a str,
    pub protocol: i32,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct StatusResponsePlayers<'a> {
    pub max: i32,
    #[serde(borrow)]
    pub sample: Cow<'a, [StatusResponsePlayersSample<'a>]>,
    pub online: i32,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct StatusResponsePlayersSample<'a> {
    #[serde(borrow)]
    pub name: &'a str,
    pub id: Uuid,
}

#[derive(ProtocolAll, ProtocolPacket, Clone, PartialEq, Debug)]
#[bp(id = 0x0, state = Status, bound = Client)]
pub struct StatusResponse<'a>(
    #[bp(variant = Json)]
    pub StatusResponseObject<'a>
);

#[derive(ProtocolAll, ProtocolPacket, Clone, Copy, PartialEq, Debug)]
#[bp(id = 0x1, state = Status, bound = Client)]
pub struct PingResponse {
    pub payload: u64,
}

#[derive(ProtocolAll, ProtocolPacket, Clone, Copy, PartialEq, Debug)]
#[bp(id = 0x0, state = Status, bound = Server)]
pub struct StatusRequest;

#[derive(ProtocolAll, ProtocolPacket, Clone, Copy, PartialEq, Debug)]
#[bp(id = 0x1, state = Status, bound = Server)]
pub struct PingRequest {
    pub payload: u64,
}

#[derive(ProtocolAll, ProtocolPacket, Clone, PartialEq, Debug)]
#[bp(id = 0x0, state = Login, bound = Client)]
pub struct LoginDisconnect<'a> {
    #[bp(variant = Json)]
    pub reason: Component<'a>,
}

#[derive(ProtocolAll, ProtocolPacket, Clone, Copy, PartialEq, Debug)]
#[bp(id = 0x1, state = Login, bound = Client)]
pub struct EncryptionRequest<'a> {
    pub server_id: &'a str,
    #[bp(variant = "LengthProvidedBytesArray<i32, VarInt>")]
    pub public_key: &'a [u8],
    #[bp(variant = "LengthProvidedBytesArray<i32, VarInt>")]
    pub verify_token: &'a [u8],
}

#[derive(ProtocolAll, Clone, Copy, PartialEq, Debug)]
pub struct LoginSuccessProperty<'a> {
    pub name: &'a str,
    pub value: &'a str,
    pub signature: Option<&'a str>,
}

#[derive(ProtocolAll, ProtocolPacket, Clone, PartialEq, Debug)]
#[bp(id = 0x2, state = Login, bound = Client)]
pub struct LoginSuccess<'a> {
    pub uuid: Uuid,
    pub username: &'a str,
    #[bp(variant = "LengthProvidedArray<i32, VarInt, LoginSuccessProperty<'a>, LoginSuccessProperty<'a>>")]
    pub properties: Cow<'a, [LoginSuccessProperty<'a>]>,
}

#[derive(ProtocolAll, ProtocolPacket, Clone, Copy, PartialEq, Debug)]
#[bp(id = 0x3, state = Login, bound = Client)]
pub struct SetCompression {
    #[bp(variant = VarInt)]
    pub threshold: i32,
}

#[derive(ProtocolAll, ProtocolPacket, Clone, PartialEq, Debug)]
#[bp(id = 0x4, state = Login, bound = Client)]
pub struct LoginPluginRequest<'a> {
    #[bp(variant = VarInt)]
    pub message_id: i32,
    pub channel: Identifier<'a>,
    #[bp(variant = RemainingBytesArray)]
    pub data: &'a [u8],
}

#[derive(ProtocolAll, Clone, Copy, PartialEq, Debug)]
pub struct LoginStartSignatureData<'a> {
    pub timestamp: u64,
    #[bp(variant = "LengthProvidedBytesArray<i32, VarInt>")]
    pub public_key: &'a [u8],
    #[bp(variant = "LengthProvidedBytesArray<i32, VarInt>")]
    pub signature: &'a [u8],
}

#[derive(ProtocolAll, ProtocolPacket, Clone, Copy, PartialEq, Debug)]
#[bp(id = 0x0, state = Login, bound = Server)]
pub struct LoginStart<'a> {
    pub name: &'a str,
    pub signature_data: Option<LoginStartSignatureData<'a>>,
    pub uuid: Option<Uuid>,
}

#[derive(ProtocolAll, Clone, Copy, PartialEq, Debug)]
#[bp(ty = bool)]
pub enum EncryptionResponseVariant<'a> {
    #[bp(value = true)]
    VerifyToken {
        #[bp(variant = "LengthProvidedBytesArray<i32, VarInt>")]
        verify_token: &'a [u8]
    },
    #[bp(value = false)]
    Otherwise {
        salt: i64,
        #[bp(variant = "LengthProvidedBytesArray<i32, VarInt>")]
        message_signature: &'a [u8],
    }
}

#[derive(ProtocolAll, ProtocolPacket, Clone, Copy, PartialEq, Debug)]
#[bp(id = 0x1, state = Login, bound = Server)]
pub struct EncryptionResponse<'a> {
    #[bp(variant = "LengthProvidedBytesArray<i32, VarInt>")]
    pub shared_secret: &'a [u8],
    pub variant: EncryptionResponseVariant<'a>,
}

#[derive(ProtocolAll, ProtocolPacket, Clone, Copy, PartialEq, Debug)]
#[bp(id = 0x2, state = Login, bound = Server)]
pub struct LoginPluginResponse<'a> {
    #[bp(variant = VarInt)]
    pub message_id: i32,
    pub successful: bool,
    #[bp(variant = RemainingBytesArray)]
    pub data: &'a [u8],
}