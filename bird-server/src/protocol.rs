use std::borrow::Cow;
use std::ops::Range;
use bitfield_struct::bitfield;
use euclid::default::Vector3D;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use bird_chat::component::Component;
use bird_chat::identifier::Identifier;
use bird_protocol::{*, ProtocolPacketState::*, ProtocolPacketBound::*};
use bird_protocol::derive::{ProtocolAll, ProtocolPacket, ProtocolReadable, ProtocolSize, ProtocolWritable};

#[derive(ProtocolAll, Clone, Copy, PartialEq, Debug)]
pub struct Slot<'a> {
    #[bp(variant = VarInt)]
    pub item_id: i32,
    pub item_count: i8,
    #[bp(variant = RemainingBytesArray)]
    pub nbt: &'a [u8],
}

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
    },
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

#[derive(ProtocolAll, ProtocolPacket, Clone, Copy, PartialEq, Debug)]
#[bp(id = 0x0, state = Play, bound = Client)]
pub struct SpawnEntity {
    #[bp(variant = VarInt)]
    pub entity_id: i32,
    pub entity_uuid: Uuid,
    #[bp(variant = VarInt)]
    pub entity_type: i32,
    pub position: Vector3D<f64>,
    #[bp(variant = Angle)]
    pub pitch: f32,
    #[bp(variant = Angle)]
    pub yaw: f32,
    #[bp(variant = Angle)]
    pub head_yaw: f32,
    #[bp(variant = VarInt)]
    pub data: i32,
    pub velocity: Vector3D<i16>,
}

#[derive(ProtocolAll, ProtocolPacket, Clone, Copy, PartialEq, Debug)]
#[bp(id = 0x1, state = Play, bound = Client)]
pub struct SpawnExperienceOrb {
    #[bp(variant = VarInt)]
    pub entity_id: i32,
    pub position: Vector3D<f64>,
    pub count: i16,
}

#[derive(ProtocolAll, ProtocolPacket, Clone, Copy, PartialEq, Debug)]
#[bp(id = 0x2, state = Play, bound = Client)]
pub struct SpawnPlayer {
    #[bp(variant = VarInt)]
    pub entity_id: i32,
    pub player_uuid: Uuid,
    pub position: Vector3D<f64>,
    #[bp(variant = Angle)]
    pub yaw: f32,
    #[bp(variant = Angle)]
    pub pitch: f32,
}

#[derive(ProtocolAll, Clone, Copy, PartialEq, Debug)]
#[bp(ty = u8)]
pub enum EntityAnimation {
    SwingMainArm,
    TakeDamage,
    LeaveBed,
    SwingOffHand,
    CriticalEffect,
    MagicCriticalEffect,
}

#[derive(ProtocolAll, ProtocolPacket, Clone, Copy, PartialEq, Debug)]
#[bp(id = 0x3, state = Play, bound = Client)]
pub struct EntityAnimation2C {
    #[bp(variant = VarInt)]
    pub entity_id: i32,
    pub animation: EntityAnimation,
}

// Identifies block id in award statistics
pub type AwardStatisticBlock = i32;

// Identified item id in award statistics
pub type AwardStatisticItem = i32;

// Identifier entity id in award statistics
pub type AwardStatisticEntity = i32;

#[derive(ProtocolAll, Clone, Copy, PartialEq, Debug)]
#[bp(ty = i32, variant = VarInt)]
pub enum AwardStatisticCustom {
    LeaveGame,
    PlayOneMinute,
    TimeSinceDeath,
    TimeSinceRest,
    SneakTime,
    WalkOneCm,
    CrouchOneCm,
    SprintOneCm,
    WalkOnWaterOneCm,
    FallOneCm,
    ClimbOneCm,
    FlyOneCm,
    WalkUnderWaterOneCm,
    MinecartOneCm,
    BoatOneCm,
    PigOneCm,
    HorseOneCm,
    AviateOneCm,
    SwimOneCm,
    StriderOneCm,
    Jump,
    Drop,
    DamageDealt,
    DamageDealtAbsorbed,
    DamageDealtResisted,
    DamageTaken,
    DamageBlockedByShield,
    DamageAbsorbed,
    DamageResisted,
    Deaths,
    MobKills,
    AnimalsBred,
    PlayerKills,
    FishCaught,
    TalkedToVillager,
    TradedWithVillager,
    EatCakeSlice,
    FillCauldron,
    UseCauldron,
    CleanArmor,
    CleanBanner,
    CleanShulkerBox,
    InteractWithBrewingStand,
    InteractWithBeacon,
    InspectDropper,
    InspectHopper,
    InspectDispenser,
    PlayNoteBlock,
    TuneNoteBlock,
    PotFlower,
    TriggerTrappedChest,
    OpenEnderchest,
    EnchantItem,
    PlayRecord,
    InteractWithFurnace,
    InteractWithCraftingTable,
    OpenChest,
    SleepInBed,
    OpenShulkerBox,
    OpenBarrel,
    InteractWithBlastFurnace,
    InteractWithSmoker,
    InteractWithLectern,
    InteractWithCampfire,
    InteractWithCartographyTable,
    InteractWithLoom,
    InteractWithStoneCutter,
    BellRing,
    RaidTrigger,
    RaidWin,
    InteractWithAnvil,
    InteractWithGrindstone,
    TargetHit,
    InteractWithSmithingTable,
}

#[derive(ProtocolAll, Clone, Copy, PartialEq, Debug)]
#[bp(ty = i32, variant = VarInt)]
pub enum AwardStatistic {
    Mined(
        #[bp(variant = VarInt)]
        AwardStatisticBlock
    ),
    Crafted(
        #[bp(variant = VarInt)]
        AwardStatisticItem
    ),
    Used(
        #[bp(variant = VarInt)]
        AwardStatisticItem
    ),
    Broken(
        #[bp(variant = VarInt)]
        AwardStatisticItem
    ),
    PickedUp(
        #[bp(variant = VarInt)]
        AwardStatisticItem
    ),
    Dropped(
        #[bp(variant = VarInt)]
        AwardStatisticItem
    ),
    Killed(
        #[bp(variant = VarInt)]
        AwardStatisticEntity
    ),
    KilledBy(
        #[bp(variant = VarInt)]
        AwardStatisticEntity
    ),
    Custom(AwardStatisticCustom),
}

#[derive(ProtocolAll, ProtocolPacket, Clone, PartialEq, Debug)]
#[bp(id = 0x4, state = Play, bound = Client)]
pub struct AwardStatistics<'a> {
    #[bp(variant = "LengthProvidedArray<i32, VarInt, AwardStatistic, AwardStatistic>")]
    pub statistics: Cow<'a, [AwardStatistic]>,
    #[bp(variant = VarInt)]
    pub value: i32,
}

#[derive(ProtocolAll, ProtocolPacket, Clone, Copy, PartialEq, Debug)]
#[bp(id = 0x5, state = Play, bound = Client)]
pub struct AcknowledgeBlockChange {
    #[bp(variant = VarInt)]
    pub sequence_id: i32,
}

#[derive(ProtocolAll, ProtocolPacket, Clone, Copy, PartialEq, Debug)]
#[bp(id = 0x6, state = Play, bound = Client)]
pub struct SetBlockDestroyStage {
    #[bp(variant = VarInt)]
    pub entity_id: i32,
    #[bp(variant = BlockPosition)]
    pub location: Vector3D<i32>,
    pub destroy_stage: u8,
}

#[derive(ProtocolAll, ProtocolPacket, Clone, Copy, PartialEq, Debug)]
#[bp(id = 0x7, state = Play, bound = Client)]
pub struct BlockEntityData<'a> {
    #[bp(variant = BlockPosition)]
    pub location: Vector3D<i32>,
    #[bp(variant = VarInt)]
    pub ty: i32,
    #[bp(variant = RemainingBytesArray)]
    pub nbt_data: &'a [u8],
}

#[derive(ProtocolAll, Clone, Copy, PartialEq, Debug)]
#[bp(ty = u8)]
pub enum BlockActionVariantPistonDirection {
    Down,
    Up,
    South,
    West,
    North,
    East,
}

#[derive(ProtocolAll, Clone, Copy, PartialEq, Debug)]
#[bp(ty = u8)]
pub enum BlockActionVariantBellDirection {
    Down,
    Up,
    North,
    South,
    West,
    East,
}

#[derive(ProtocolAll, Clone, Copy, PartialEq, Debug)]
#[bp(ty = i32, variant = VarInt, key_reverse = true)]
pub enum BlockActionVariant {
    #[bp(value = "(bird_data::block_data::NOTE_BLOCK.id) as i32", ghost = [(order = begin, value = 0u8), (order = end, value = 0u8)])]
    NoteBlock,
    #[bp(value = "(bird_data::block_data::PISTON.id) as i32")]
    Piston {
        retract: bool,
        direction: BlockActionVariantPistonDirection,
    },
    #[bp(value = "(bird_data::block_data::CHEST.id) as i32", ghost = [(order = begin, value = 1u8)])]
    Chest {
        players_looking_in: u8,
    },
    #[bp(value = "(bird_data::block_data::ENDER_CHEST.id) as i32", ghost = [(order = begin, value = 1u8)])]
    EnderChest {
        players_looking_in: u8,
    },
    #[bp(value = "(bird_data::block_data::BEACON.id) as i32", ghost = [(order = begin, value = 1u8), (order = end, value = 0u8)])]
    Beacon,
    #[bp(value = "(bird_data::block_data::SPAWNER.id) as i32", ghost = [(order = begin, value = 1u8), (order = end, value = 0u8)])]
    Spawner,
    #[bp(value = "(bird_data::block_data::END_GATEWAY.id) as i32", ghost = [(order = begin, value = 1u8), (order = end, value = 0u8)])]
    EndGateway,
    #[bp(value = "(bird_data::block_data::SHULKER_BOX.id) as i32", ghost = [(order = begin, value = 1u8)])]
    ShulkerBox {
        players_looking_in: u8,
    },
    #[bp(value = "(bird_data::block_data::BELL.id) as i32", ghost = [(order = begin, value = 1u8)])]
    Bell {
        direction: BlockActionVariantBellDirection,
    }
}

#[derive(ProtocolAll, ProtocolPacket, Clone, Copy, PartialEq, Debug)]
#[bp(id = 0x8, state = Play, bound = Client)]
pub struct BlockAction {
    #[bp(variant = BlockPosition)]
    pub location: Vector3D<i32>,
    pub variant: BlockActionVariant,
}

#[derive(ProtocolAll, ProtocolPacket, Clone, Copy, PartialEq, Debug)]
#[bp(id = 0x9, state = Play, bound = Client)]
pub struct BlockUpdate {
    #[bp(variant = BlockPosition)]
    pub location: Vector3D<i32>,
    #[bp(variant = VarInt)]
    pub block_id: i32,
}

#[derive(ProtocolAll, Clone, Copy, PartialEq, Debug)]
#[bp(ty = i32, variant = VarInt)]
pub enum BossBarColor {
    Pink,
    Blue,
    Red,
    Green,
    Yellow,
    Purple,
    White,
}

#[derive(ProtocolAll, Clone, Copy, PartialEq, Debug)]
#[bp(ty = i32, variant = VarInt)]
pub enum BossBarDivision {
    Zero,
    Six,
    Ten,
    Twelve,
    Twenty,
}

#[bitfield(u8)]
#[derive(ProtocolAll, PartialEq)]
pub struct BossBarFlags {
    pub dark_sky: bool,
    pub dragon_bar: bool,
    pub fog: bool,
    #[bits(5)]
    _pad: u8,
}

#[derive(ProtocolAll, Clone, PartialEq, Debug)]
#[bp(ty = i32, variant = VarInt)]
pub enum BossBarAction<'a> {
    Add {
        title: Component<'a>,
        health: f32,
        color: BossBarColor,
        division: BossBarDivision,
        flags: BossBarFlags,
    },
    Remove,
    UpdateHealth {
        health: f32,
    },
    UpdateTitle {
        title: Component<'a>,
    },
    UpdateStyle {
        color: BossBarColor,
        division: BossBarDivision,
    },
    UpdateFlags {
        flags: BossBarFlags,
    }
}

#[derive(ProtocolAll, ProtocolPacket, Clone, PartialEq, Debug)]
#[bp(id = 0xA, state = Play, bound = Client)]
pub struct BossBar<'a> {
    pub uuid: Uuid,
    pub action: BossBarAction<'a>,
}

#[derive(ProtocolAll, Clone, Copy, PartialEq, Debug)]
#[bp(ty = u8)]
pub enum Difficulty {
    Peaceful,
    Easy,
    Normal,
    Hard,
}

#[derive(ProtocolAll, ProtocolPacket, Clone, Copy, PartialEq, Debug)]
#[bp(id = 0xB, state = Play, bound = Client)]
pub struct ChangeDifficulty {
    pub difficulty: Difficulty,
    pub locked: bool,
}

#[derive(ProtocolAll, ProtocolPacket, Clone, PartialEq, Debug)]
#[bp(id = 0xC, state = Play, bound = Client)]
pub struct ChatPreviewC<'a> {
    pub query_id: i32,
    pub message: Option<Component<'a>>,
}

#[derive(ProtocolAll, ProtocolPacket, Clone, Copy, PartialEq, Debug)]
#[bp(id = 0xD, state = Play, bound = Client)]
pub struct ClearTitles {
    pub reset: bool,
}

#[derive(ProtocolAll, Clone, PartialEq, Debug)]
pub struct CommandSuggestionsMatch<'a> {
    pub insert: &'a str,
    pub tooltip: Option<Component<'a>>,
}

#[derive(ProtocolAll, ProtocolPacket, Clone, PartialEq, Debug)]
#[bp(id = 0xE, state = Play, bound = Client)]
pub struct CommandSuggestionsResponse<'a> {
    #[bp(variant = VarInt)]
    pub id: i32,
    #[bp(variant = VarInt)]
    pub start: i32,
    #[bp(variant = VarInt)]
    pub length: i32,
    #[bp(variant = "LengthProvidedArray<i32, VarInt, CommandSuggestionsMatch<'a>, CommandSuggestionsMatch<'a>>")]
    pub matches: Cow<'a, [CommandSuggestionsMatch<'a>]>,
}

pub const PLAYER_INVENTORY_ID: u8 = 0;

#[derive(ProtocolAll, ProtocolPacket, Clone, Copy, PartialEq, Debug)]
#[bp(id = 0x10, state = Play, bound = Client)]
pub struct CloseContainer {
    pub window_id: u8,
}

#[derive(ProtocolAll, ProtocolPacket, Clone, PartialEq, Debug)]
#[bp(id = 0x11, state = Play, bound = Client)]
pub struct SetContainerContent<'a> {
    pub window_id: u8,
    #[bp(variant = VarInt)]
    pub state_id: i32,
    #[bp(variant = "LengthProvidedArray<i32, VarInt, Option<Slot<'a>>, Option<Slot<'a>>>")]
    pub slot_data: Cow<'a, [Option<Slot<'a>>]>,
    pub carried_item: Option<Slot<'a>>,
}