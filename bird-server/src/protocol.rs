use std::borrow::Cow;
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::ops::{Range, Shl};
use bitfield_struct::bitfield;
use euclid::default::{Vector2D, Vector3D};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use bird_chat::component::Component;
use bird_chat::identifier::Identifier;
use bird_protocol::{*, ProtocolPacketState::*, ProtocolPacketBound::*};
use bird_protocol::derive::{BirdNBT, ProtocolAll, ProtocolPacket, ProtocolReadable, ProtocolSize, ProtocolWritable};
use bird_util::*;
use crate::nbt::{NbtElement, read_compound_enter, read_named_nbt_tag, write_compound_enter, write_nbt_string};

#[derive(ProtocolAll, Clone, Copy, PartialEq, Debug)]
pub struct Slot<'a> {
    #[bp(variant = VarInt)]
    pub item_id: i32,
    pub item_count: i8,
    #[bp(variant = NbtBytes)]
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
pub struct StatusResponseSS2C<'a>(
    #[bp(variant = Json)]
    pub StatusResponseObject<'a>
);

#[derive(ProtocolAll, ProtocolPacket, Clone, Copy, PartialEq, Debug)]
#[bp(id = 0x1, state = Status, bound = Client)]
pub struct PingResponseSS2C {
    pub payload: u64,
}

#[derive(ProtocolAll, ProtocolPacket, Clone, Copy, PartialEq, Debug)]
#[bp(id = 0x0, state = Status, bound = Server)]
pub struct StatusRequest;

#[derive(ProtocolAll, ProtocolPacket, Clone, Copy, PartialEq, Debug)]
#[bp(id = 0x1, state = Status, bound = Server)]
pub struct PingRequestSC2S {
    pub payload: u64,
}

#[derive(ProtocolAll, ProtocolPacket, Clone, PartialEq, Debug)]
#[bp(id = 0x0, state = Login, bound = Client)]
pub struct LoginDisconnectLS2C<'a> {
    #[bp(variant = Json)]
    pub reason: Component<'a>,
}

#[derive(ProtocolAll, ProtocolPacket, Clone, Copy, PartialEq, Debug)]
#[bp(id = 0x1, state = Login, bound = Client)]
pub struct EncryptionRequestLS2C<'a> {
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
pub struct LoginSuccessLS2C<'a> {
    pub uuid: Uuid,
    pub username: &'a str,
    #[bp(variant = "LengthProvidedArray<i32, VarInt, LoginSuccessProperty<'a>, LoginSuccessProperty<'a>>")]
    pub properties: Cow<'a, [LoginSuccessProperty<'a>]>,
}

#[derive(ProtocolAll, ProtocolPacket, Clone, Copy, PartialEq, Debug)]
#[bp(id = 0x3, state = Login, bound = Client)]
pub struct SetCompressionLS2C {
    #[bp(variant = VarInt)]
    pub threshold: i32,
}

#[derive(ProtocolAll, ProtocolPacket, Clone, PartialEq, Debug)]
#[bp(id = 0x4, state = Login, bound = Client)]
pub struct LoginPluginRequestLS2C<'a> {
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
pub struct LoginStartLC2S<'a> {
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
pub struct EncryptionResponseLC2S<'a> {
    #[bp(variant = "LengthProvidedBytesArray<i32, VarInt>")]
    pub shared_secret: &'a [u8],
    pub variant: EncryptionResponseVariant<'a>,
}

#[derive(ProtocolAll, ProtocolPacket, Clone, Copy, PartialEq, Debug)]
#[bp(id = 0x2, state = Login, bound = Server)]
pub struct LoginPluginResponseLC2S<'a> {
    #[bp(variant = VarInt)]
    pub message_id: i32,
    pub successful: bool,
    #[bp(variant = RemainingBytesArray)]
    pub data: &'a [u8],
}

#[derive(ProtocolAll, ProtocolPacket, Clone, Copy, PartialEq, Debug)]
#[bp(id = 0x0, state = Play, bound = Client)]
pub struct SpawnEntityPS2C {
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
pub struct SpawnExperienceOrbPS2C {
    #[bp(variant = VarInt)]
    pub entity_id: i32,
    pub position: Vector3D<f64>,
    pub count: i16,
}

#[derive(ProtocolAll, ProtocolPacket, Clone, Copy, PartialEq, Debug)]
#[bp(id = 0x2, state = Play, bound = Client)]
pub struct SpawnPlayerPS2C {
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
pub struct EntityAnimationPS2C {
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
pub struct AwardStatisticsPS2C<'a> {
    #[bp(variant = "LengthProvidedArray<i32, VarInt, AwardStatistic, AwardStatistic>")]
    pub statistics: Cow<'a, [AwardStatistic]>,
    #[bp(variant = VarInt)]
    pub value: i32,
}

#[derive(ProtocolAll, ProtocolPacket, Clone, Copy, PartialEq, Debug)]
#[bp(id = 0x5, state = Play, bound = Client)]
pub struct AcknowledgeBlockChangePS2C {
    #[bp(variant = VarInt)]
    pub sequence_id: i32,
}

#[derive(ProtocolAll, ProtocolPacket, Clone, Copy, PartialEq, Debug)]
#[bp(id = 0x6, state = Play, bound = Client)]
pub struct SetBlockDestroyStagePS2C {
    #[bp(variant = VarInt)]
    pub entity_id: i32,
    #[bp(variant = BlockPosition)]
    pub location: Vector3D<i32>,
    pub destroy_stage: u8,
}

#[derive(ProtocolAll, ProtocolPacket, Clone, Copy, PartialEq, Debug)]
#[bp(id = 0x7, state = Play, bound = Client)]
pub struct BlockEntityDataPS2C<'a> {
    #[bp(variant = BlockPosition)]
    pub location: Vector3D<i32>,
    #[bp(variant = VarInt)]
    pub ty: i32,
    #[bp(variant = NbtBytes)]
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
    },
}

#[derive(ProtocolAll, ProtocolPacket, Clone, Copy, PartialEq, Debug)]
#[bp(id = 0x8, state = Play, bound = Client)]
pub struct BlockActionPS2C {
    #[bp(variant = BlockPosition)]
    pub location: Vector3D<i32>,
    pub variant: BlockActionVariant,
}

#[derive(ProtocolAll, ProtocolPacket, Clone, Copy, PartialEq, Debug)]
#[bp(id = 0x9, state = Play, bound = Client)]
pub struct BlockUpdatePS2C {
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
    },
}

#[derive(ProtocolAll, ProtocolPacket, Clone, PartialEq, Debug)]
#[bp(id = 0xA, state = Play, bound = Client)]
pub struct BossBarPS2C<'a> {
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
pub struct ChangeDifficultyPS2C {
    pub difficulty: Difficulty,
    pub locked: bool,
}

#[derive(ProtocolAll, ProtocolPacket, Clone, Copy, PartialEq, Debug)]
#[bp(id = 0xC, state = Play, bound = Client)]
pub struct ClearTitles {
    pub reset: bool,
}

// #[derive(ProtocolAll, ProtocolPacket, Clone, PartialEq, Debug)]
// #[bp(id = 0xC, state = Play, bound = Client)]
// pub struct ChatPreviewPS2C<'a> {
//     pub query_id: i32,
//     pub message: Option<Component<'a>>,
// }

#[derive(ProtocolAll, Clone, PartialEq, Debug)]
pub struct CommandSuggestionsMatch<'a> {
    pub insert: &'a str,
    pub tooltip: Option<Component<'a>>,
}

#[derive(ProtocolAll, ProtocolPacket, Clone, PartialEq, Debug)]
#[bp(id = 0xD, state = Play, bound = Client)]
pub struct CommandSuggestionsResponsePS2C<'a> {
    #[bp(variant = VarInt)]
    pub id: i32,
    #[bp(variant = VarInt)]
    pub start: i32,
    #[bp(variant = VarInt)]
    pub length: i32,
    #[bp(variant = "LengthProvidedArray<i32, VarInt, CommandSuggestionsMatch<'a>, CommandSuggestionsMatch<'a>>")]
    pub matches: Cow<'a, [CommandSuggestionsMatch<'a>]>,
}

pub const ROOT_NODE_TYPE: u8 = 0;
pub const LITERAL_NODE_TYPE: u8 = 1;
pub const ARGUMENT_NODE_TYPE: u8 = 2;

#[bitfield(i8)]
#[derive(ProtocolAll, PartialEq)]
pub struct BrigadierNodeFlags {
    #[bits(2)]
    pub node_type: u8,
    pub executable: bool,
    pub redirect: bool,
    pub suggestions_type: bool,
    #[bits(3)]
    _pad: u8,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct BrigadierNodeRangeProperties<T> {
    pub min: Option<T>,
    pub max: Option<T>,
}

impl<T> ProtocolSize for BrigadierNodeRangeProperties<T>
    where T: ProtocolSize {
    const SIZE: Range<u32> = add_protocol_sizes_ty!(
        Option<T>,
        Option<T>,
        u8
    );
}

impl<T> ProtocolWritable for BrigadierNodeRangeProperties<T>
    where T: ProtocolWritable {
    fn write<W: ProtocolWriter>(&self, writer: &mut W) -> anyhow::Result<()> {
        let flags = if self.min.is_some() { 1u8 } else { 0u8 } | if self.max.is_some() { 2u8 } else { 0u8 };
        flags.write(writer)?;
        if let Some(ref to_write) = self.min { to_write.write(writer)? };
        if let Some(ref to_write) = self.max { to_write.write(writer)? };
        Ok(())
    }
}

impl<'a, T> ProtocolReadable<'a> for BrigadierNodeRangeProperties<T>
    where T: ProtocolReadable<'a> {
    fn read<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<Self> {
        let flags = u8::read(cursor)?;
        let min = match flags & 0x2 != 0 {
            true => Some(T::read(cursor)?),
            false => None,
        };
        let max = match flags & 0x1 != 0 {
            true => Some(T::read(cursor)?),
            false => None,
        };
        Ok(Self { min, max })
    }
}

#[derive(ProtocolAll, Clone, Copy, PartialEq, Debug)]
#[bp(ty = i32, variant = VarInt)]
pub enum BrigadierNodeParserString {
    SingleWord,
    QuotablePhrase,
    GreedyPhrase,
}

#[bitfield(u8)]
#[derive(ProtocolAll, PartialEq)]
pub struct BrigadierNodeParseEntity {
    pub single: bool,
    pub only_players: bool,
    #[bits(6)]
    _gap: u8,
}

#[derive(ProtocolAll, Clone, PartialEq, Debug)]
#[bp(ty = i32, variant = VarInt)]
pub enum BrigadierNodeParser<'a> {
    Bool,
    Float(BrigadierNodeRangeProperties<f32>),
    Double(BrigadierNodeRangeProperties<f64>),
    Integer(BrigadierNodeRangeProperties<i32>),
    Long(BrigadierNodeRangeProperties<i64>),
    String(BrigadierNodeParserString),
    Entity(BrigadierNodeParseEntity),
    GameProfile,
    BlockPos,
    ColumnPos,
    Vec3,
    Vec2,
    BlockState,
    BlockPredicate,
    ItemStack,
    ItemPredicate,
    Color,
    Component,
    Message,
    Nbt,
    NbtTag,
    NbtPath,
    Objective,
    ObjectiveCriteria,
    Operation,
    Particle,
    Angle,
    Rotation,
    ScoreboardSlot,
    ScoreHolder {
        multiple: bool,
    },
    Swizzle,
    Team,
    ItemSlot,
    ResourceLocation,
    MobEffect,
    Function,
    EntityAnchor,
    IntRange,
    FloatRange,
    ItemEnchantment,
    EntitySummon,
    Dimension,
    Time,
    ResourceOrTag {
        registry: Identifier<'a>,
    },
    Resource {
        registry: Identifier<'a>,
    },
    TemplateMirror,
    // ?
    TemplateRotation,
    // ?
    Uuid,
}

#[derive(Clone, PartialEq, Debug)]
pub struct BrigadierNode<'a> {
    pub executable: bool,
    pub children: Cow<'a, [i32]>,
    pub redirect_node: Option<i32>,
    pub name: Option<&'a str>,
    pub parser: Option<BrigadierNodeParser<'a>>,
    pub suggestions_type: Option<Identifier<'a>>,
}

impl<'a> ProtocolSize for BrigadierNode<'a> {
    const SIZE: Range<u32> = (
        add_protocol_sizes_ty!(
            u8,
            LengthProvidedArray<i32, VarInt, i32, i32>,
        ).start
            ..
            add_protocol_sizes_ty!(
            u8,
            LengthProvidedArray<i32, VarInt, i32, i32>,
            VarInt,
            &'a str,
            BrigadierNodeParser<'a>,
            Identifier<'a>,
        ).end
    );
}

impl<'a> ProtocolWritable for BrigadierNode<'a> {
    fn write<W: ProtocolWriter>(&self, writer: &mut W) -> anyhow::Result<()> {
        let flags = BrigadierNodeFlags::new()
            .with_node_type(match self.name {
                Some(_) => match self.parser {
                    Some(_) => ARGUMENT_NODE_TYPE,
                    None => LITERAL_NODE_TYPE,
                },
                None => ROOT_NODE_TYPE,
            })
            .with_executable(self.executable)
            .with_redirect(self.redirect_node.is_some())
            .with_suggestions_type(self.suggestions_type.is_some());
        flags.write(writer)?;
        LengthProvidedArray::<i32, VarInt, i32, i32>::write_variant(&self.children, writer)?;
        if let Some(ref to_write) = self.redirect_node { to_write.write(writer)? };
        if let Some(ref to_write) = self.parser { to_write.write(writer)? };
        if let Some(ref to_write) = self.suggestions_type { to_write.write(writer)? };
        Ok(())
    }
}

impl<'a> ProtocolReadable<'a> for BrigadierNode<'a> {
    fn read<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<Self> {
        let flags = BrigadierNodeFlags::read(cursor)?;
        let children = LengthProvidedArray::<i32, VarInt, i32, i32>::read_variant(cursor)?;
        let redirect_node = match flags.redirect() {
            true => Some(VarInt::read_variant(cursor)?),
            false => None,
        };
        let (name, parser) = match flags.node_type() {
            ROOT_NODE_TYPE => (None, None),
            LITERAL_NODE_TYPE => (Some(<&'a str>::read(cursor)?), None),
            _ => (Some(<&'a str>::read(cursor)?), Some(BrigadierNodeParser::read(cursor)?)),
        };
        let suggestions_type = match flags.suggestions_type() {
            true => Some(Identifier::read(cursor)?),
            false => None,
        };
        Ok(Self {
            executable: flags.executable(),
            children,
            redirect_node,
            name,
            parser,
            suggestions_type,
        })
    }
}

#[derive(ProtocolAll, ProtocolPacket, Clone, PartialEq, Debug)]
#[bp(id = 0xE, state = Play, bound = Client)]
pub struct CommandsPS2C<'a> {
    #[bp(variant = "LengthProvidedArray<i32, VarInt, BrigadierNode<'a>, BrigadierNode<'a>>")]
    pub nodes: Cow<'a, [BrigadierNode<'a>]>,
    #[bp(variant = VarInt)]
    pub root_index: i32,
}

pub const PLAYER_INVENTORY_ID: u8 = 0;

#[derive(ProtocolAll, ProtocolPacket, Clone, Copy, PartialEq, Debug)]
#[bp(id = 0xF, state = Play, bound = Client)]
pub struct CloseContainerPS2C {
    pub window_id: u8,
}

#[derive(ProtocolAll, ProtocolPacket, Clone, PartialEq, Debug)]
#[bp(id = 0x10, state = Play, bound = Client)]
pub struct SetContainerContentPS2C<'a> {
    pub window_id: u8,
    #[bp(variant = VarInt)]
    pub state_id: i32,
    #[bp(variant = "LengthProvidedArray<i32, VarInt, Option<Slot<'a>>, Option<Slot<'a>>>")]
    pub slot_data: Cow<'a, [Option<Slot<'a>>]>,
    pub carried_item: Option<Slot<'a>>,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum FurnaceProperty {
    FireIcon,
    MaximumFuelBurnTime,
    ProgressArrow,
    MaximumProgress,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum EnchantmentTableSlot {
    Top,
    Middle,
    Bottom,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum EnchantmentTableProperty {
    LevelRequirement(EnchantmentTableSlot),
    Seed,
    EnchantmentId(EnchantmentTableSlot),
    EnchantmentLevel(EnchantmentTableSlot),
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum BeaconProperty {
    PowerLevel,
    FirstPotionEffect,
    SecondPotionEffect,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum BrewingStandProperty {
    BrewTime,
    FuelTime,
}

#[derive(ProtocolAll, ProtocolPacket, Clone, Copy, PartialEq, Debug)]
#[bp(id = 0x11, state = Play, bound = Client)]
pub struct SetContainerPropertyPS2C {
    pub window_id: u8,
    pub property: i16,
    pub value: i16,
}

impl TryFrom<i16> for FurnaceProperty {
    type Error = ();

    fn try_from(value: i16) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(FurnaceProperty::FireIcon),
            1 => Ok(FurnaceProperty::MaximumFuelBurnTime),
            2 => Ok(FurnaceProperty::ProgressArrow),
            3 => Ok(FurnaceProperty::MaximumProgress),
            _ => Err(()),
        }
    }
}

impl From<FurnaceProperty> for i16 {
    fn from(value: FurnaceProperty) -> Self {
        match value {
            FurnaceProperty::FireIcon => 0,
            FurnaceProperty::MaximumFuelBurnTime => 1,
            FurnaceProperty::ProgressArrow => 2,
            FurnaceProperty::MaximumProgress => 3,
        }
    }
}

impl TryFrom<i16> for EnchantmentTableProperty {
    type Error = ();

    fn try_from(value: i16) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(EnchantmentTableProperty::LevelRequirement(EnchantmentTableSlot::Top)),
            1 => Ok(EnchantmentTableProperty::LevelRequirement(EnchantmentTableSlot::Middle)),
            2 => Ok(EnchantmentTableProperty::LevelRequirement(EnchantmentTableSlot::Bottom)),
            3 => Ok(EnchantmentTableProperty::Seed),
            4 => Ok(EnchantmentTableProperty::EnchantmentId(EnchantmentTableSlot::Top)),
            5 => Ok(EnchantmentTableProperty::EnchantmentId(EnchantmentTableSlot::Middle)),
            6 => Ok(EnchantmentTableProperty::EnchantmentId(EnchantmentTableSlot::Bottom)),
            7 => Ok(EnchantmentTableProperty::EnchantmentLevel(EnchantmentTableSlot::Top)),
            8 => Ok(EnchantmentTableProperty::EnchantmentLevel(EnchantmentTableSlot::Middle)),
            9 => Ok(EnchantmentTableProperty::EnchantmentLevel(EnchantmentTableSlot::Bottom)),
            _ => Err(()),
        }
    }
}

impl From<EnchantmentTableProperty> for i16 {
    fn from(value: EnchantmentTableProperty) -> Self {
        match value {
            EnchantmentTableProperty::LevelRequirement(EnchantmentTableSlot::Top) => 0,
            EnchantmentTableProperty::LevelRequirement(EnchantmentTableSlot::Middle) => 1,
            EnchantmentTableProperty::LevelRequirement(EnchantmentTableSlot::Bottom) => 2,
            EnchantmentTableProperty::Seed => 3,
            EnchantmentTableProperty::EnchantmentId(EnchantmentTableSlot::Top) => 4,
            EnchantmentTableProperty::EnchantmentId(EnchantmentTableSlot::Middle) => 5,
            EnchantmentTableProperty::EnchantmentId(EnchantmentTableSlot::Bottom) => 6,
            EnchantmentTableProperty::EnchantmentLevel(EnchantmentTableSlot::Top) => 7,
            EnchantmentTableProperty::EnchantmentLevel(EnchantmentTableSlot::Middle) => 8,
            EnchantmentTableProperty::EnchantmentLevel(EnchantmentTableSlot::Bottom) => 9,
        }
    }
}

impl TryFrom<i16> for BeaconProperty {
    type Error = ();

    fn try_from(value: i16) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(BeaconProperty::PowerLevel),
            1 => Ok(BeaconProperty::FirstPotionEffect),
            2 => Ok(BeaconProperty::SecondPotionEffect),
            _ => Err(()),
        }
    }
}

impl From<BeaconProperty> for i16 {
    fn from(value: BeaconProperty) -> Self {
        match value {
            BeaconProperty::PowerLevel => 0,
            BeaconProperty::FirstPotionEffect => 1,
            BeaconProperty::SecondPotionEffect => 2,
        }
    }
}

impl TryFrom<i16> for BrewingStandProperty {
    type Error = ();

    fn try_from(value: i16) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(BrewingStandProperty::BrewTime),
            1 => Ok(BrewingStandProperty::FuelTime),
            _ => Err(()),
        }
    }
}

impl From<BrewingStandProperty> for i16 {
    fn from(value: BrewingStandProperty) -> Self {
        match value {
            BrewingStandProperty::BrewTime => 0,
            BrewingStandProperty::FuelTime => 1,
        }
    }
}

pub const CURSOR_SLOT_ID: i16 = -1;
pub const CURSOR_WINDOW_ID: i8 = -1;

#[derive(ProtocolAll, ProtocolPacket, Clone, Copy, PartialEq, Debug)]
#[bp(id = 0x12, state = Play, bound = Client)]
pub struct SetContainerSlotPS2C<'a> {
    pub window_id: i8,
    #[bp(variant = VarInt)]
    pub state_id: i32,
    pub slot: i16,
    pub slot_data: Option<Slot<'a>>,
}

#[derive(ProtocolAll, ProtocolPacket, Clone, Copy, PartialEq, Debug)]
#[bp(id = 0x13, state = Play, bound = Client)]
pub struct SetCooldownPS2C {
    #[bp(variant = VarInt)]
    pub item_id: i32,
    #[bp(variant = VarInt)]
    pub cooldown_ticks: i32,
}

#[derive(ProtocolAll, Clone, Copy, PartialEq, Debug)]
#[bp(ty = i32, variant = VarInt)]
pub enum ChatSuggestionAction {
    Add,
    Remove,
    Set,
}

#[derive(ProtocolAll, ProtocolPacket, Clone, PartialEq, Debug)]
#[bp(id = 0x14, state = Play, bound = Client)]
pub struct ChatSuggestionsPS2C<'a> {
    pub action: ChatSuggestionAction,
    #[bp(variant = "LengthProvidedArray<i32, VarInt, &'a str, &'a str>")]
    pub entries: Cow<'a, [&'a str]>,
}

#[derive(ProtocolAll, ProtocolPacket, Clone, PartialEq, Debug)]
#[bp(id = 0x15, state = Play, bound = Client)]
pub struct PluginMessagePS2C<'a> {
    pub channel: Identifier<'a>,
    #[bp(variant = RemainingBytesArray)]
    pub data: &'a [u8],
}

#[derive(ProtocolAll, ProtocolPacket, Clone, Copy, PartialEq, Debug)]
#[bp(id = 0x16, state = Play, bound = Client)]
pub struct DeleteMessagePS2C<'a> {
    #[bp(variant = "LengthProvidedBytesArray<i32, VarInt>")]
    pub signature: &'a [u8],
}

#[derive(ProtocolAll, ProtocolPacket, Clone, PartialEq, Debug)]
#[bp(id = 0x17, state = Play, bound = Client)]
pub struct DisconnectPS2C<'a> {
    pub reason: Component<'a>,
}

#[derive(ProtocolAll, ProtocolPacket, Clone, PartialEq, Debug)]
#[bp(id = 0x18, state = Play, bound = Client)]
pub struct DisguisedChatMessagePS2C<'a> {
    pub message: Component<'a>,
    #[bp(variant = VarInt)]
    pub chat_type: i32,
    pub chat_type_name: Component<'a>,
    pub target_name: Option<Component<'a>>,
}

#[derive(ProtocolAll, Clone, Copy, PartialEq, Debug)]
#[bp(ty = i32, variant = VarInt)]
pub enum CustomSoundCategory {
    Master,
    Music,
    Record,
    Weather,
    Block,
    Hostile,
    Neutral,
    Player,
    Ambient,
    Voice,
}

// #[derive(ProtocolAll, ProtocolPacket, Clone, PartialEq, Debug)]
// #[bp(id = 0x17, state = Play, bound = Client)]
// pub struct CustomSoundEffectPS2C<'a> {
//     pub sound_name: Identifier<'a>,
//     pub sound_category: CustomSoundCategory,
//     #[bp(variant = "FixedPointNumber<i32, 3>")]
//     pub effect_position_x: f32,
//     #[bp(variant = "FixedPointNumber<i32, 3>")]
//     pub effect_position_y: f32,
//     #[bp(variant = "FixedPointNumber<i32, 3>")]
//     pub effect_position_z: f32,
//     pub volume: f32,
//     pub pitch: f32,
//     pub seed: i64,
// }

// #[derive(ProtocolAll, ProtocolPacket, Clone, Copy, PartialEq, Debug)]
// #[bp(id = 0x18, state = Play, bound = Client)]
// pub struct HideMessagePS2C<'a> {
//     #[bp(variant = "LengthProvidedBytesArray<i32, VarInt>")]
//     pub signature: &'a [u8],
// }

#[derive(ProtocolAll, Clone, Copy, PartialEq, Debug)]
#[bp(ty = i8)]
pub enum EntityEventStatus {
    // TODO
}

#[derive(ProtocolAll, ProtocolPacket, Clone, Copy, PartialEq, Debug)]
#[bp(id = 0x19, state = Play, bound = Client)]
pub struct EntityEventPS2C {
    pub entity_id: i32,
}

#[derive(ProtocolAll, ProtocolPacket, Clone, Copy, PartialEq, Debug)]
#[bp(id = 0x1A, state = Play, bound = Client)]
pub struct ExplosionPS2C<'a> {
    pub location: Vector3D<f32>,
    pub strength: f32,
    #[bp(variant = "LengthProvidedRawArray<i32, VarInt, Vector3D<i8>, Vector3D<i8>>")]
    pub records: &'a [Vector3D<i8>],
    pub motion: Vector3D<f32>,
}

#[derive(ProtocolAll, ProtocolPacket, Clone, Copy, PartialEq, Debug)]
#[bp(id = 0x1B, state = Play, bound = Client)]
pub struct UnloadChunkPS2C {
    pub chunk_x: i32,
    pub chunk_z: i32,
}

#[derive(ProtocolAll, Clone, Copy, PartialEq, Debug)]
#[bp(ty = f32)]
pub enum GameEventGameMode {
    Survival,
    Creative,
    Adventure,
    Spectator,
}

#[derive(ProtocolAll, Clone, Copy, PartialEq, Debug)]
#[bp(ty = f32)]
pub enum GameEventDemo {
    ShowWelcome,
    #[bp(value = 101f32)]
    TellMovementControls,
    TellJumpControl,
    TellInventoryControl,
    TellDemoIsOver,
}

#[derive(ProtocolAll, Clone, Copy, PartialEq, Debug)]
#[bp(ty = f32)]
pub enum GameEventWinGame {
    RespawnPlayer,
    RollTheCredits,
}

#[derive(ProtocolAll, Clone, Copy, PartialEq, Debug)]
#[bp(ty = f32)]
pub enum GameEventRespawnScreen {
    EnableScreen,
    ImmediatelyRespawn,
}

#[derive(ProtocolAll, ProtocolPacket, Clone, Copy, PartialEq, Debug)]
#[bp(id = 0x1C, state = Play, bound = Client, ty = u8)]
pub enum GameEventPS2C {
    #[bp(ghost = [(order = begin, value = 0f32)])]
    NoRespawnBlockAvailable,
    #[bp(ghost = [(order = begin, value = 0f32)])]
    EndRaining,
    #[bp(ghost = [(order = begin, value = 0f32)])]
    BeginRaining,
    ChangeGameMode(GameEventGameMode),
    WinGame(GameEventWinGame),
    DemoEvent(GameEventDemo),
    #[bp(ghost = [(order = begin, value = 0f32)])]
    ArrowHitPlayer,
    RainLevelChange(f32),
    ThunderLevelChange(f32),
    #[bp(ghost = [(order = begin, value = 0f32)])]
    PufferfishSting,
    #[bp(ghost = [(order = begin, value = 0f32)])]
    ElderGuardianMobAppearance,
    EnableRespawnScreen(GameEventRespawnScreen),
}

#[derive(ProtocolAll, ProtocolPacket, Clone, Copy, PartialEq, Debug)]
#[bp(id = 0x1D, state = Play, bound = Client)]
pub struct OpenHorseScreenPS2C {
    pub window_id: u8,
    #[bp(variant = VarInt)]
    pub slots: i32,
    pub entity_id: i32,
}

#[derive(ProtocolAll, ProtocolPacket, Clone, Copy, PartialEq, Debug)]
#[bp(id = 0x1E, state = Play, bound = Client)]
pub struct InitializeWorldBorderPS2C {
    pub x: f64,
    pub y: f64,
    pub old_diameter: f64,
    pub new_diameter: f64,
    #[bp(variant = VarLong)]
    pub speed: i64,
    #[bp(variant = VarInt)]
    pub portal_teleport_boundary: i32,
    #[bp(variant = VarInt)]
    pub warning_blocks: i32,
    #[bp(variant = VarInt)]
    pub warning_seconds: i32,
}

#[derive(ProtocolAll, ProtocolPacket, Clone, Copy, PartialEq, Debug)]
#[bp(id = 0x1F, state = Play, bound = Client)]
pub struct KeepAlivePS2C {
    pub keep_alive_id: i64,
}

#[derive(Debug)]
pub struct GapCompactLongsWriter<'a, W: ProtocolWriter> {
    writer: &'a mut W,
    current: u64,
    bits: u8,
    elements_in_long: u8,
    gap: u8,
    current_index: u8,
}

impl<'a, W: ProtocolWriter> GapCompactLongsWriter<'a, W> {
    /// # Safety
    /// The caller must ensure that the number of bits is less or equals to 64
    pub unsafe fn new(writer: &'a mut W, bits: u8) -> Self {
        debug_assert!(bits <= 64);
        Self {
            writer,
            current: 0,
            bits,
            elements_in_long: 64 / bits,
            gap: 64 % bits,
            current_index: 0,
        }
    }

    /// # Safety.
    /// The caller must ensure that the number is not longer than bits
    pub unsafe fn write(&mut self, number: u64) -> anyhow::Result<()> {
        debug_assert!(number < (1 << (self.bits + 1)));
        if self.current_index == self.elements_in_long {
            self.current.write(self.writer)?;
            self.current = 0;
            self.current_index = 0;
        }
        self.current |= number << (self.current_index * self.bits + self.gap);
        self.current_index += 1;
        Ok(())
    }

    /// # Safety
    /// The caller must ensure that each number in iterator is not longer than bits
    pub unsafe fn write_all(&mut self, iterator: impl Iterator<Item=u64>) -> anyhow::Result<()> {
        for num in iterator {
            self.write(num)?
        }
        Ok(())
    }

    /// # Safety.
    /// The caller must ensure that the number is not longer than bits
    pub unsafe fn write_and_finish(mut self, number: u64) -> anyhow::Result<()> {
        self.write(number)?;
        self.finish()
    }

    /// # Safety
    /// The caller must ensure that each number in iterator is not longer than bits
    pub unsafe fn write_all_and_finish(mut self, iterator: impl Iterator<Item=u64>) -> anyhow::Result<()> {
        self.write_all(iterator)?;
        self.finish()
    }

    pub fn finish(self) -> anyhow::Result<()> {
        if self.current_index != 0 {
            self.current.write(self.writer)?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
pub struct GapCompactLongsReader<I, const COUNT: usize> {
    iterator: I,
    current_long: u64,
    next_long: Option<u64>,
    bits: u8,
    gap: u8,
    elements_in_long: u8,
    end_index: u8,
    current_index: u8,
    mask: u64,
}

impl<I: Iterator<Item=u64>, const COUNT: usize> GapCompactLongsReader<I, COUNT> {
    /// # Safety
    /// The caller must ensure that number of bits is less or equals to 64
    pub unsafe fn new(mut iterator: I, bits: u8) -> Option<Self> {
        debug_assert!(bits <= 64);
        let gap = 64 % bits;
        let elements_in_long = 64 / bits;
        let current_long = iterator.next()? >> gap;
        let next_long = iterator.next();
        Some(Self {
            iterator,
            current_long,
            next_long,
            bits,
            gap,
            elements_in_long,
            mask: (1 << (bits as u64)) - 1,
            end_index: {
                let result = COUNT % (elements_in_long as usize);
                if result == 0 { elements_in_long } else { result as u8 }
            },
            current_index: 0,
        })
    }
}

impl<I: Iterator<Item=u64>, const COUNT: usize> Iterator for GapCompactLongsReader<I, COUNT> {
    type Item = u64;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next_long.is_none() && self.current_index == self.end_index {
            return None;
        }
        if self.current_index == self.elements_in_long {
            self.current_index = 0;
            self.current_long = unsafe { self.next_long.unwrap_unchecked() } >> self.gap;
            self.next_long = self.iterator.next();
        }
        let result = self.current_long & self.mask;
        self.current_long >>= self.bits;
        self.current_index += 1;
        Some(result)
    }
}

/// # Safety
/// The caller must ensure that number of bits is less or equals to 64
pub const unsafe fn compact_longs_array_length(elements: usize, bits: u8) -> usize {
    debug_assert!(bits <= 64);
    let elements_in_long = (64 / bits) as usize;
    elements / elements_in_long + (if elements % elements_in_long == 0 { 0 } else { 1 })
}

pub const CHUNK_DATA_HEIGHT_MAP_KEY: &'static str = "MOTION_BLOCKING";

// TODO should it be only MOTION_BLOCKING or WORLD_SURFACE also?

#[derive(Clone, Copy, PartialEq, Debug)]
#[repr(transparent)]
pub struct ChunkDataHeightMap<'a>(BorrowedLongArray<'a>);

#[derive(Clone, Copy, PartialEq, Debug)]
#[doc(hidden)]
pub enum BorrowedLongArray<'a> {
    Raw(&'a [u8]),
    Longs(&'a [u64]),
}

impl<'a> Iterator for BorrowedLongArray<'a> {
    type Item = u64;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Raw(raw) => u64::read(raw).ok(),
            Self::Longs(long) => {
                let number = *long.get(0)?;
                *long = &long[1..];
                Some(number)
            }
        }
    }
}

impl<'a> IntoIterator for ChunkDataHeightMap<'a> {
    type Item = u64;
    type IntoIter = GapCompactLongsReader<BorrowedLongArray<'a>, 256>;

    fn into_iter(self) -> Self::IntoIter {
        // SAFETY: It is sure that array of inner struct is not empty.
        unsafe { Self::IntoIter::new(self.0, 9).unwrap_unchecked() }
    }
}

impl<'a> ChunkDataHeightMap<'a> {
    /// # Safety.
    /// The caller must ensure that the length of data slice is 37 * 8
    pub const unsafe fn new_raw(data: &'a [u8]) -> Self {
        debug_assert!(data.len() == 37 * 8);
        Self(BorrowedLongArray::Raw(data))
    }

    /// # Safety.
    /// The caller must ensure that the length of data is 37
    pub const unsafe fn new_longs(data: &'a [u64]) -> Self {
        debug_assert!(data.len() == 37);
        Self(BorrowedLongArray::Longs(data))
    }
}

impl<'a> ProtocolSize for ChunkDataHeightMap<'a> {
    const SIZE: Range<u32> = Nbt::SIZE;
}

impl<'a> ProtocolReadable<'a> for ChunkDataHeightMap<'a> {
    fn read<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<Self> {
        read_compound_enter(cursor)?;
        match read_named_nbt_tag(CHUNK_DATA_HEIGHT_MAP_KEY, cursor)? {
            Some(NbtElement::LongArray(data)) => match data.len() == 37 * 8 {
                true => Ok(Self(BorrowedLongArray::Raw(data))),
                false => Err(ProtocolError::Any(anyhow::Error::msg("MOTION_BLOCKING must be NbtLongArray with exactly 37 length")))
            },
            _ => Err(ProtocolError::Any(anyhow::Error::msg("MOTION_BLOCKING is not NbtLongArray or not present"))),
        }
    }
}

impl<'a> ProtocolWritable for ChunkDataHeightMap<'a> {
    fn write<W: ProtocolWriter>(&self, writer: &mut W) -> anyhow::Result<()> {
        write_compound_enter(writer)?;
        12i8.write(writer)?;
        write_nbt_string(CHUNK_DATA_HEIGHT_MAP_KEY, writer)?;
        match self.0 {
            BorrowedLongArray::Raw(raw) => {
                37i32.write(writer)?; // the length of raw
                writer.write_bytes(raw)
            }
            BorrowedLongArray::Longs(array) => LengthProvidedArray::<i32, i32, u64, u64>::write_variant(array, writer)?,
        }
        0i8.write(writer)
    }
}

pub trait PalettedContainerBitsDeterminer {
    fn get(values: usize) -> u8;
}

#[derive(Clone, Debug)]
pub struct PalettedContainer<T, const MAX_VALUE: i32, const LENGTH: usize> {
    inner: PalettedContainerInner<LENGTH>,
    _marker: PhantomData<T>,
}

#[derive(Clone, Debug)]
enum PalettedContainerInner<const LENGTH: usize> {
    Single(i32),
    Indirect(Vec<i32>, Box<[i32; LENGTH]>),
    Direct(Box<[i32; LENGTH]>),
}

impl<T, const MAX_VALUE: i32, const LENGTH: usize> PalettedContainer<T, MAX_VALUE, LENGTH>
    where
        T: PalettedContainerBitsDeterminer {
    const DIRECT_START: u8 = const_log2_ceil(MAX_VALUE as u64) as u8;

    pub fn new_direct(values: Box<[i32; LENGTH]>) -> Self {
        Self {
            inner: PalettedContainerInner::Direct(values),
            _marker: PhantomData,
        }
    }

    pub fn new_indirect(values: Vec<i32>, indexes: Box<[i32; LENGTH]>) -> Self {
        Self {
            inner: PalettedContainerInner::Indirect(values, indexes),
            _marker: PhantomData,
        }
    }

    pub const fn new_single(value: i32) -> Self {
        Self {
            inner: PalettedContainerInner::Single(value),
            _marker: PhantomData,
        }
    }
}

impl<T, const MAX_VALUE: i32, const LENGTH: usize> ProtocolSize for PalettedContainer<T, MAX_VALUE, LENGTH> {
    const SIZE: Range<u32> = u8::SIZE.start + VarInt::SIZE.start..u32::MAX;
}

impl<T, const MAX_VALUE: i32, const LENGTH: usize> PalettedContainer<T, MAX_VALUE, LENGTH>
    where
        T: PalettedContainerBitsDeterminer {
    const MAX_BITS: u8 = {
        let result = const_log2_ceil(MAX_VALUE as u64) as u8;
        assert!(result <= 64);
        result
    };
}

impl<T, const MAX_VALUE: i32, const LENGTH: usize> ProtocolWritable for PalettedContainer<T, MAX_VALUE, LENGTH>
    where
        T: PalettedContainerBitsDeterminer {
    fn write<W: ProtocolWriter>(&self, writer: &mut W) -> anyhow::Result<()> {
        match self.inner {
            PalettedContainerInner::Single(single) => {
                0u8.write(writer)?;
                VarInt::write_variant(&single, writer)?;
                VarInt::write_variant(&0, writer)
            }
            PalettedContainerInner::Indirect(ref values, ref indexes) => {
                let bits_per_entry = T::get(values.len());
                bits_per_entry.write(writer)?;
                LengthProvidedArray::<i32, VarInt, i32, i32>::write_variant(values, writer)?;
                VarInt::write_variant(&(unsafe { compact_longs_array_length(LENGTH, bits_per_entry) } as i32), writer)?;
                unsafe { GapCompactLongsWriter::new(writer, bits_per_entry).write_all_and_finish(indexes.iter().map(|val| *val as u64)) }
            }
            PalettedContainerInner::Direct(ref direct) => {
                Self::MAX_BITS.write(writer)?;
                VarInt::write_variant(&(unsafe { compact_longs_array_length(LENGTH, Self::MAX_BITS) } as i32), writer)?;
                unsafe { GapCompactLongsWriter::new(writer, Self::MAX_BITS).write_all_and_finish(direct.iter().map(|val| *val as u64)) }
            }
        }
    }
}

impl<'a, T, const MAX_VALUE: i32, const LENGTH: usize> ProtocolReadable<'a> for PalettedContainer<T, MAX_VALUE, LENGTH>
    where
        T: PalettedContainerBitsDeterminer + 'a {
    fn read<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<Self> {
        let bits = u8::read(cursor)?;
        Ok(if bits == 0 {
            let single = VarInt::read_variant(cursor)?;
            let _: i32 = VarInt::read_variant(cursor)?;
            Self::new_single(single)
        } else if bits < Self::MAX_BITS {
            let values = LengthProvidedArray::<i32, VarInt, i32, i32>::read_variant(cursor)?;
            let count: i32 = VarInt::read_variant(cursor)?;
            // It is said that count is ignored by vanilla client (should we ignore it also and calculate count by ourselves?)
            debug_assert!(count == unsafe { compact_longs_array_length(LENGTH, bits) as i32 });
            let indexes_iter: GapCompactLongsReader<_, LENGTH> = unsafe {
                GapCompactLongsReader::new(
                    ProtocolCursorIterator::<'_, 'a, _, _, u64, u64>::new(
                        cursor,
                        ProtocolCursorIteratorCountLimiter { count: count as _ }),
                    bits,
                )
            }
                .ok_or(ProtocolError::Any(anyhow::Error::msg("Empty array in paletted container (indirect variant)")))?;
            Self::new_indirect(
                values,
                Box::new(
                    indexes_iter
                        .map(|val| val as i32)
                        .collect::<Vec<_>>()
                        .try_into()
                        .map_err(|_| ProtocolError::Any(anyhow::Error::msg("Bad length of indexes")))?
                ),
            )
        } else {
            let count: i32 = VarInt::read_variant(cursor)?;
            debug_assert!(count == unsafe { compact_longs_array_length(LENGTH, Self::MAX_BITS) as i32 });
            let iter: GapCompactLongsReader<_, LENGTH> = unsafe {
                GapCompactLongsReader::new(
                    ProtocolCursorIterator::<'_, 'a, _, _, u64, u64>::new(
                        cursor,
                        ProtocolCursorIteratorCountLimiter { count: count as _ },
                    ),
                    Self::MAX_BITS,
                )
            }
                .ok_or(ProtocolError::Any(anyhow::Error::msg("Empty array in paletted container (direct variant)")))?;
            Self::new_direct(Box::new(
                iter
                    .map(|val| val as i32)
                    .collect::<Vec<_>>()
                    .try_into()
                    .map_err(|_| ProtocolError::Any(anyhow::Error::msg("Bad length of direct")))?
            ))
        })
    }
}

#[derive(ProtocolAll, Clone, Copy, Debug)]
pub struct ChunkSectionsData<'a> {
    #[bp(variant = "LengthProvidedBytesArray<i32, VarInt>")]
    pub data: &'a [u8],
}

#[derive(Clone, Copy, Debug)]
pub struct BlockStatesBits;

#[derive(Clone, Copy, Debug)]
pub struct BiomesBits;

impl PalettedContainerBitsDeterminer for BlockStatesBits {
    fn get(values: usize) -> u8 {
        match values <= 16 {
            true => 4,
            false => const_log2_ceil(values as u64) as u8,
        }
    }
}

impl PalettedContainerBitsDeterminer for BiomesBits {
    fn get(values: usize) -> u8 {
        const_log2_ceil(values as u64) as u8
    }
}

#[derive(ProtocolSize, ProtocolWritable, Clone, Debug)]
pub struct ChunkSectionData {
    pub block_count: i16,
    pub block_states: PalettedContainer<BlockStatesBits, { bird_data::BLOCK_STATE_COUNT as i32 }, 4096>,
    pub biomes: PalettedContainer<BiomesBits, { bird_data::BIOME_COUNT as i32 }, 64>,
}

// TODO fix issue with ProtocolReadable proc-macro (now it is not working)

impl<'a> ProtocolReadable<'a> for ChunkSectionData {
    fn read<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<Self> {
        Ok(Self {
            block_count: i16::read(cursor)?,
            block_states: PalettedContainer::read(cursor)?,
            biomes: PalettedContainer::read(cursor)?,
        })
    }
}

#[derive(ProtocolAll, Clone, Copy, Debug)]
pub struct ChunkData<'a> {
    pub height_map: ChunkDataHeightMap<'a>,
    pub chunk_sections: ChunkSectionsData<'a>,
}

#[derive(Clone, Copy, Debug)]
pub struct BitSet<'a>(BorrowedLongArray<'a>);

impl<'a> BitSet<'a> {
    fn get_bit_from_words(words: &[u64], index: usize) -> Option<bool> {
        words.get(Self::get_word_index(index)).map(|val| val & (1u64.overflowing_shl(index as u32).0) != 0)
    }

    /// # Safety
    /// The caller must ensure that the length of raw can be divided by 8
    unsafe fn get_bit_from_raw(raw: &[u8], index: usize) -> Option<bool> {
        // All longs are inverted so how we can get required byte?
        // Each byte contains 8 bits
        // So to get the position of our index we should divide by 8 (or move right by 3)
        // Because our long is inverted we should get the real position of byte
        // When our index is 0 (0b0) the real index is 7 (0b111)
        // When our index is 1 (0b1) the real index is 6 (0b110)
        // And so on...
        // So we should reverse last 3 bits
        // And so we can get the real position of required bit
        const MASK: usize = usize::MAX.overflowing_shl(3).0;
        let mut order = (index & MASK) >> 3;
        order = (order & MASK) | ((!order) & 0b111);
        raw.get(order).map(|val| val & (1u8.overflowing_shl(index as u32).0) != 0)
    }

    #[inline]
    const fn get_word_index(index: usize) -> usize {
        index >> 6
    }

    pub const fn new_words(words: &'a [u64]) -> Self {
        Self(BorrowedLongArray::Longs(words))
    }

    /// # Safety
    /// The caller must ensure that the length of raw can be divided by 8
    pub const unsafe fn new_raw(raw: &'a [u8]) -> Self {
        debug_assert!(raw.len() % 8 == 0);
        Self(BorrowedLongArray::Raw(raw))
    }

    pub fn get(&self, index: usize) -> Option<bool> {
        match self.0 {
            BorrowedLongArray::Raw(raw) => unsafe { Self::get_bit_from_raw(raw, index) },
            BorrowedLongArray::Longs(words) => Self::get_bit_from_words(words, index)
        }
    }

    pub fn long_iter(&self) -> impl Iterator<Item=u64> + 'a {
        self.clone().0
    }
}

impl<'a> ProtocolSize for BitSet<'a> {
    const SIZE: Range<u32> = (0..u32::MAX);
}

impl<'a> ProtocolWritable for BitSet<'a> {
    fn write<W: ProtocolWriter>(&self, writer: &mut W) -> anyhow::Result<()> {
        match self.0 {
            BorrowedLongArray::Raw(raw) => {
                VarInt::write_variant(&(raw.len() as i32 / 8), writer)?;
                writer.write_bytes(raw);
                Ok(())
            }
            BorrowedLongArray::Longs(longs) =>
                LengthProvidedArray::<i32, VarInt, u64, u64>::write_variant(longs, writer),
        }
    }
}

impl<'a> ProtocolReadable<'a> for BitSet<'a> {
    fn read<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<Self> {
        let length: i32 = VarInt::read_variant(cursor)?;
        Ok(Self(BorrowedLongArray::Raw(cursor.take_bytes((length * 8) as usize)?)))
    }
}

#[derive(Clone, Debug)]
pub struct OwnedBitSet {
    pub words: Vec<u64>,
}

impl OwnedBitSet {
    pub fn new() -> Self {
        Self { words: Vec::new() }
    }

    pub fn get_bit_set(&self) -> BitSet {
        BitSet::new_words(&self.words)
    }

    pub fn get(&self, index: usize) -> Option<bool> {
        BitSet::get_bit_from_words(&self.words, index)
    }

    pub fn set(&mut self, index: usize) {
        let word_index = BitSet::get_word_index(index);
        if word_index >= self.words.len() {
            self.words.resize(word_index + 1, 0);
        }
        self.words[word_index] |= (1u64.overflowing_shl(index as u32).0);
    }

    pub fn clear(&mut self, index: usize) {
        let word_index = BitSet::get_word_index(index);
        if word_index < self.words.len() {
            self.words[word_index] &= !(1u64.overflowing_shl(index as u32).0);
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct LightArray<'a> {
    // TODO change it to &'a [u8; 2048]
    bytes: &'a [u8],
}

impl<'a> ProtocolSize for LightArray<'a> {
    const SIZE: Range<u32> = (VarInt::SIZE.start + 2048..VarInt::SIZE.end + 2048);
}

impl<'a> ProtocolWritable for LightArray<'a> {
    fn write<W: ProtocolWriter>(&self, writer: &mut W) -> anyhow::Result<()> {
        VarInt::write_variant(&2048, writer)?;
        writer.write_bytes(self.bytes);
        Ok(())
    }
}

impl<'a> ProtocolReadable<'a> for LightArray<'a> {
    fn read<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<Self> {
        let length: i32 = VarInt::read_variant(cursor)?;
        if length != 2048i32 {
            return Err(ProtocolError::Any(anyhow::Error::msg("The length of light array is not 2048")));
        }
        Ok(Self { bytes: cursor.take_bytes(2048)? })
    }
}

impl<'a> LightArray<'a> {
    const unsafe fn get_index(position: Vector3D<u8>) -> (usize, bool) {
        debug_assert!(position.x < 16);
        debug_assert!(position.y < 16);
        debug_assert!(position.z < 16);
        let index = ((position.y as usize) << 8) | ((position.z as usize) << 4) | (position.x as usize);
        (index >> 1, index & 0x1 == 0)
    }

    const unsafe fn get_from_array(data: &[u8], position: Vector3D<u8>) -> u8 {
        let (index, right) = Self::get_index(position);
        match right {
            true => data[index] & 0xf,
            false => (data[index] & 0xf0) >> 4,
        }
    }

    /// # Safety
    /// The caller must ensure that the length of bytes is 2048
    pub const unsafe fn new(bytes: &'a [u8]) -> Self {
        debug_assert!(bytes.len() == 2048);
        Self { bytes }
    }

    pub const fn get_bytes(&self) -> &'a [u8] {
        self.bytes
    }

    /// # Safety
    /// The caller must ensure that each parameter is less than 16
    pub const unsafe fn get(&self, position: Vector3D<u8>) -> u8 {
        Self::get_from_array(self.bytes, position)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct OwnedLightArray {
    data: [u8; 2048],
    // We are counting not empty bytes
    not_empty: u16,
}

impl OwnedLightArray {
    pub const fn new() -> Self {
        Self { data: [0; 2048], not_empty: 0 }
    }

    /// # Safety
    /// The caller must ensure that each parameter is less than 16
    pub unsafe fn set(&mut self, position: Vector3D<u8>, value: u8) {
        debug_assert!(value < 16);
        let (index, right) = LightArray::get_index(position);
        let res = match right {
            true => self.data[index] & 0xf0 | value,
            false => self.data[index] & 0x0f | (value << 4),
        };
        if self.data[index] != 0 { self.not_empty -= 1 };
        if res != 0 { self.not_empty += 1 };
        self.data[index] = res;
    }

    /// # Safety
    /// The caller must ensure that each parameter is less than 16
    pub const unsafe fn get(&self, position: Vector3D<u8>) -> u8 {
        LightArray::get_from_array(&self.data, position)
    }

    pub const fn as_light_array(&self) -> LightArray {
        unsafe { LightArray::new(&self.data) }
    }

    pub const fn is_empty(&self) -> bool {
        self.not_empty == 0
    }
}

#[derive(ProtocolAll, Clone, Debug)]
pub struct LightData<'a> {
    pub trust_edges: bool,
    pub sky_light_mask: BitSet<'a>,
    pub block_light_mask: BitSet<'a>,
    pub empty_sky_light_mask: BitSet<'a>,
    pub empty_block_light_mask: BitSet<'a>,
    #[bp(variant = "LengthProvidedArray<i32, VarInt, LightArray<'a>, LightArray<'a>>")]
    pub sky_light_arrays: Cow<'a, [LightArray<'a>]>,
    #[bp(variant = "LengthProvidedArray<i32, VarInt, LightArray<'a>, LightArray<'a>>")]
    pub block_light_arrays: Cow<'a, [LightArray<'a>]>,
}

#[bitfield(u8)]
#[derive(ProtocolAll)]
pub struct PackedBlockChunkXZ {
    #[bits(4)]
    pub x: u8,
    #[bits(4)]
    pub z: u8,
}

#[derive(ProtocolAll, Clone, Copy, Debug)]
pub struct ChunkDataAndUpdateLightBlockEntity<'a> {
    pub xz: PackedBlockChunkXZ,
    pub y: i16,
    #[bp(variant = VarInt)]
    pub ty: i32,
    #[bp(variant = NbtBytes)]
    pub data: &'a [u8],
}

#[derive(ProtocolAll, ProtocolPacket, Clone, Debug)]
#[bp(id = 0x20, state = Play, bound = Client)]
pub struct ChunkDataAndUpdateLightPS2C<'a> {
    pub chunk: Vector2D<i32>,
    pub chunk_data: ChunkData<'a>,
    #[bp(variant = "LengthProvidedArray<i32, VarInt, ChunkDataAndUpdateLightBlockEntity<'a>, ChunkDataAndUpdateLightBlockEntity<'a>>")]
    pub block_entities: Cow<'a, [ChunkDataAndUpdateLightBlockEntity<'a>]>,
    pub light_data: LightData<'a>,
}

#[derive(Clone, Copy, Debug)]
pub enum SmokeDirection {
    Down,
    Up,
    North,
    South,
    West,
    East,
}

impl TryFrom<u8> for SmokeDirection {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(SmokeDirection::Down),
            1 => Ok(SmokeDirection::Up),
            2 => Ok(SmokeDirection::North),
            3 => Ok(SmokeDirection::South),
            4 => Ok(SmokeDirection::West),
            5 => Ok(SmokeDirection::East),
            _ => Err(()),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum WorldEvent {
    // Sounds
    Dispense,
    FailedDispense,
    DispenserShoots,
    EnderEyeLaunches,
    FireworkShots,
    IronDoorOpens,
    WoodenDoorOpens,
    WoodenTrapdoorOpens,
    FenceGateOpens,
    FireExtinguishes,
    PlayRecord { record_id: i32 },
    IronDoorCloses,
    WoodenDoorCloses,
    WoodenTrapDoorCloses,
    FenceGateCloses,
    GhastWarns,
    GhastShoots,
    EnderDragonShoots,
    BlazeShoots,
    ZombieAttacksWoodenDoor,
    ZombieAttacksIronDoor,
    ZombieBreaksWoodenDoor,
    WitherBreaksBlock,
    WitherSpawns,
    WitherShoots,
    BatTakesOff,
    ZombieInfects,
    ZombieVillagerConverts,
    EnderDragonDeath,
    AnvilDestroy,
    AnvilUse,
    AnvilLand,
    PortalTravel,
    ChorusFlowerGrown,
    ChorusFlowerDeath,
    BrewingStandBrew,
    IronTrapdoorOpens,
    IronTrapdoorCloses,
    OverworldEndPortalCreates,
    PhantomBites,
    ZombieConvertsToDrowned,
    HuskConvertsToZombie,
    GrindstoneUsed,
    BookPageTurned,
    // 1043
    // Particles
    ComposterComposts,
    LavaConvertsBlock,
    RedstoneTorchBurnsOut,
    EnderEyePlace,
    // 1503
    Smoke { direction: SmokeDirection },
    BlockBreak { block_state: i32 },
    SplashPotion { color: i32 },
    EyeOfEnderBreak,
    MobSpawn,
    BonemealParticles { amount: i32 },
    DragonBreath,
    InstantSplashPotion { color: i32 },
    EnderDragonDestroysBlock,
    WetSpongeVaporizesInNether,
    // 2009
    EndGatewaySpawn,
    EnderDragonGrowl,
    ElectricSpark,
    CopperApplyWax,
    CopperRemoveWax,
    CopperScrapeOxidation, // 3005
}

impl WorldEvent {
    pub fn new(id: i32, value: i32) -> Option<Self> {
        Some(match id {
            1000 => WorldEvent::Dispense,
            1001 => WorldEvent::FailedDispense,
            1002 => WorldEvent::DispenserShoots,
            1003 => WorldEvent::EnderEyeLaunches,
            1004 => WorldEvent::FireworkShots,
            1005 => WorldEvent::IronDoorOpens,
            1006 => WorldEvent::WoodenDoorOpens,
            1007 => WorldEvent::WoodenTrapdoorOpens,
            1008 => WorldEvent::FenceGateOpens,
            1009 => WorldEvent::FireExtinguishes,
            1010 => WorldEvent::PlayRecord { record_id: value },
            1011 => WorldEvent::IronDoorCloses,
            1012 => WorldEvent::WoodenDoorCloses,
            1013 => WorldEvent::WoodenTrapDoorCloses,
            1014 => WorldEvent::FenceGateCloses,
            1015 => WorldEvent::GhastWarns,
            1016 => WorldEvent::GhastShoots,
            1017 => WorldEvent::EnderDragonShoots,
            1018 => WorldEvent::BlazeShoots,
            1019 => WorldEvent::ZombieAttacksWoodenDoor,
            1020 => WorldEvent::ZombieAttacksIronDoor,
            1021 => WorldEvent::ZombieBreaksWoodenDoor,
            1022 => WorldEvent::WitherBreaksBlock,
            1023 => WorldEvent::WitherSpawns,
            1024 => WorldEvent::WitherShoots,
            1025 => WorldEvent::BatTakesOff,
            1026 => WorldEvent::ZombieInfects,
            1027 => WorldEvent::ZombieVillagerConverts,
            1028 => WorldEvent::EnderDragonDeath,
            1029 => WorldEvent::AnvilDestroy,
            1030 => WorldEvent::AnvilUse,
            1031 => WorldEvent::AnvilLand,
            1032 => WorldEvent::PortalTravel,
            1033 => WorldEvent::ChorusFlowerGrown,
            1034 => WorldEvent::ChorusFlowerDeath,
            1035 => WorldEvent::BrewingStandBrew,
            1036 => WorldEvent::IronTrapdoorOpens,
            1037 => WorldEvent::IronTrapdoorCloses,
            1038 => WorldEvent::OverworldEndPortalCreates,
            1039 => WorldEvent::PhantomBites,
            1040 => WorldEvent::ZombieConvertsToDrowned,
            1041 => WorldEvent::HuskConvertsToZombie,
            1042 => WorldEvent::GrindstoneUsed,
            1043 => WorldEvent::BookPageTurned,
            1500 => WorldEvent::ComposterComposts,
            1501 => WorldEvent::LavaConvertsBlock,
            1502 => WorldEvent::RedstoneTorchBurnsOut,
            1503 => WorldEvent::EnderEyePlace,
            2000 => WorldEvent::Smoke { direction: SmokeDirection::try_from(value as u8).ok()? },
            2001 => WorldEvent::BlockBreak { block_state: value },
            2002 => WorldEvent::SplashPotion { color: value },
            2003 => WorldEvent::EyeOfEnderBreak,
            2004 => WorldEvent::MobSpawn,
            2005 => WorldEvent::BonemealParticles { amount: value },
            2006 => WorldEvent::DragonBreath,
            2007 => WorldEvent::InstantSplashPotion { color: value },
            2008 => WorldEvent::EnderDragonDestroysBlock,
            2009 => WorldEvent::WetSpongeVaporizesInNether,
            3000 => WorldEvent::EndGatewaySpawn,
            3001 => WorldEvent::EnderDragonGrowl,
            3002 => WorldEvent::ElectricSpark,
            3003 => WorldEvent::CopperApplyWax,
            3004 => WorldEvent::CopperRemoveWax,
            3005 => WorldEvent::CopperScrapeOxidation,
            _ => None?
        })
    }

    pub fn get_id_value(&self) -> (i32, i32) {
        match self {
            WorldEvent::Dispense => (1000, 0),
            WorldEvent::FailedDispense => (1001, 0),
            WorldEvent::DispenserShoots => (1002, 0),
            WorldEvent::EnderEyeLaunches => (1003, 0),
            WorldEvent::FireworkShots => (1004, 0),
            WorldEvent::IronDoorOpens => (1005, 0),
            WorldEvent::WoodenDoorOpens => (1006, 0),
            WorldEvent::WoodenTrapdoorOpens => (1007, 0),
            WorldEvent::FenceGateOpens => (1008, 0),
            WorldEvent::FireExtinguishes => (1009, 0),
            WorldEvent::PlayRecord { record_id } => (1010, *record_id),
            WorldEvent::IronDoorCloses => (1011, 0),
            WorldEvent::WoodenDoorCloses => (1012, 0),
            WorldEvent::WoodenTrapDoorCloses => (1013, 0),
            WorldEvent::FenceGateCloses => (1014, 0),
            WorldEvent::GhastWarns => (1015, 0),
            WorldEvent::GhastShoots => (1016, 0),
            WorldEvent::EnderDragonShoots => (1017, 0),
            WorldEvent::BlazeShoots => (1018, 0),
            WorldEvent::ZombieAttacksWoodenDoor => (1019, 0),
            WorldEvent::ZombieAttacksIronDoor => (1020, 0),
            WorldEvent::ZombieBreaksWoodenDoor => (1021, 0),
            WorldEvent::WitherBreaksBlock => (1022, 0),
            WorldEvent::WitherSpawns => (1023, 0),
            WorldEvent::WitherShoots => (1024, 0),
            WorldEvent::BatTakesOff => (1025, 0),
            WorldEvent::ZombieInfects => (1026, 0),
            WorldEvent::ZombieVillagerConverts => (1027, 0),
            WorldEvent::EnderDragonDeath => (1028, 0),
            WorldEvent::AnvilDestroy => (1029, 0),
            WorldEvent::AnvilUse => (1030, 0),
            WorldEvent::AnvilLand => (1031, 0),
            WorldEvent::PortalTravel => (1032, 0),
            WorldEvent::ChorusFlowerGrown => (1033, 0),
            WorldEvent::ChorusFlowerDeath => (1034, 0),
            WorldEvent::BrewingStandBrew => (1035, 0),
            WorldEvent::IronTrapdoorOpens => (1036, 0),
            WorldEvent::IronTrapdoorCloses => (1037, 0),
            WorldEvent::OverworldEndPortalCreates => (1038, 0),
            WorldEvent::PhantomBites => (1039, 0),
            WorldEvent::ZombieConvertsToDrowned => (1040, 0),
            WorldEvent::HuskConvertsToZombie => (1041, 0),
            WorldEvent::GrindstoneUsed => (1042, 0),
            WorldEvent::BookPageTurned => (1043, 0),
            WorldEvent::ComposterComposts => (1500, 0),
            WorldEvent::LavaConvertsBlock => (1501, 0),
            WorldEvent::RedstoneTorchBurnsOut => (1502, 0),
            WorldEvent::EnderEyePlace => (1503, 0),
            WorldEvent::Smoke { direction } => (2000, (*direction) as i32),
            WorldEvent::BlockBreak { block_state } => (2001, *block_state),
            WorldEvent::SplashPotion { color } => (2002, *color),
            WorldEvent::EyeOfEnderBreak => (2003, 0),
            WorldEvent::MobSpawn => (2004, 0),
            WorldEvent::BonemealParticles { amount } => (2005, *amount),
            WorldEvent::DragonBreath => (2006, 0),
            WorldEvent::InstantSplashPotion { color } => (2007, *color),
            WorldEvent::EnderDragonDestroysBlock => (2008, 0),
            WorldEvent::WetSpongeVaporizesInNether => (2009, 0),
            WorldEvent::EndGatewaySpawn => (3000, 0),
            WorldEvent::EnderDragonGrowl => (3001, 0),
            WorldEvent::ElectricSpark => (3002, 0),
            WorldEvent::CopperApplyWax => (3003, 0),
            WorldEvent::CopperRemoveWax => (3004, 0),
            WorldEvent::CopperScrapeOxidation => (3005, 0),
        }
    }
}

#[derive(ProtocolPacket, Clone, Copy, Debug)]
#[bp(id = 0x21, state = Play, bound = Client)]
pub struct WorldEventPS2C {
    pub event: WorldEvent,
    pub location: Vector3D<i32>,
    pub disable_relative_volume: bool,
}

impl ProtocolSize for WorldEventPS2C {
    const SIZE: Range<u32> = add_protocol_sizes_ty!(i32, i32, Vector3D<i32>, bool);
}

impl ProtocolWritable for WorldEventPS2C {
    fn write<W: ProtocolWriter>(&self, writer: &mut W) -> anyhow::Result<()> {
        let (event, event_data) = self.event.get_id_value();
        event.write(writer)?;
        BlockPosition::write_variant(&self.location, writer)?;
        event_data.write(writer)?;
        self.disable_relative_volume.write(writer)
    }
}

impl<'a> ProtocolReadable<'a> for WorldEventPS2C {
    fn read<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<Self> {
        let event_id = i32::read(cursor)?;
        let location = BlockPosition::read_variant(cursor)?;
        let event_data = i32::read(cursor)?;
        let disable_relative_volume = bool::read(cursor)?;
        Ok(Self {
            event: WorldEvent::new(event_id, event_data)
                .ok_or_else(|| ProtocolError::Any(anyhow::Error::msg("Bad world event id")))?,
            location,
            disable_relative_volume,
        })
    }
}

#[repr(u8)]
#[derive(ProtocolSize, Clone, Copy, Debug, PartialEq)]
#[bp(variant = VarInt, ty = i32)]
pub enum Particle<'a> {
    AmbientEntityEffect,
    AngryVillager,
    Block {
        #[bp(variant = VarInt)]
        block_state: i32
    },
    BlockMarker {
        #[bp(variant = VarInt)]
        block_state: i32
    },
    Bubble,
    Cloud,
    Crit,
    DamageIndicator,
    DragonBreath,
    DrippingLava,
    FallingLava,
    LandingLava,
    DrippingWater,
    FallingWater,
    Dust {
        red: f32,
        green: f32,
        blue: f32,
        scale: f32,
    },
    DustColorTransition {
        from_red: f32,
        from_green: f32,
        from_blue: f32,
        scale: f32,
        to_red: f32,
        to_green: f32,
        to_blue: f32,
    },
    Effect,
    ElderGuardian,
    EnchantedHit,
    Enchant,
    EndRod,
    EntityEffect,
    ExplosionEmitter,
    Explosion,
    FallingDust {
        #[bp(variant = VarInt)]
        block_state: i32
    },
    Firework,
    Fishing,
    Flame,
    SoulFireFlame,
    Soul,
    Flash,
    HappyVillager,
    Composter,
    Heart,
    InstantEffect,
    Item {
        slot: Option<Slot<'a>>
    },
    Vibration {
        variant: VibrationVariant<'a>,
        ticks: i32,
    },
    ItemSlime,
    ItemSnowball,
    LargeSmoke,
    Lava,
    Mycelium,
    Note,
    Poof,
    Portal,
    Rain,
    Smoke,
    Sneeze,
    Spit,
    SquidInk,
    SweepAttack,
    TotemOfUndying,
    Underwater,
    Splash,
    Witch,
    BubblePop,
    CurrentDown,
    BubbleColumnUp,
    Nautilus,
    Dolphin,
    CampfireCosySmoke,
    CampfireSignalSmoke,
    DrippingHoney,
    FallingHoney,
    LandingHoney,
    FallingNectar,
    FallingSporeBlossom,
    Ash,
    CrimsonSpore,
    WarpedSpore,
    SporeBlossomAir,
    DrippingObsidianTear,
    FallingObsidianTear,
    LandingObsidianTear,
    ReversePortal,
    WhiteAsh,
    SmallFlame,
    Snowflake,
    DrippingDripstoneLava,
    FallingDripstoneLava,
    DrippingDripstoneWater,
    FallingDripstoneWater,
    GlowSquidInk,
    Glow,
    WaxOn,
    WaxOff,
    ElectricSpark,
    Scrape,
}

impl<'a> Particle<'a> {
    pub fn read<C: ProtocolCursor<'a>>(id: i32, cursor: &mut C) -> ProtocolResult<Self> {
        Ok(match id {
            2 => Self::Block { block_state: VarInt::read_variant(cursor)? },
            3 => Self::BlockMarker { block_state: VarInt::read_variant(cursor)? },
            14 => Self::Dust {
                red: f32::read(cursor)?,
                green: f32::read(cursor)?,
                blue: f32::read(cursor)?,
                scale: f32::read(cursor)?.clamp(0.01, 4f32),
            },
            15 => Self::DustColorTransition {
                from_red: f32::read(cursor)?,
                from_green: f32::read(cursor)?,
                from_blue: f32::read(cursor)?,
                scale: f32::read(cursor)?.clamp(0.01, 4f32),
                to_red: f32::read(cursor)?,
                to_green: f32::read(cursor)?,
                to_blue: f32::read(cursor)?,
            },
            35 => Self::Item { slot: Option::read(cursor)? },
            36 => Self::Vibration {
                variant: match <&'a str>::read(cursor)? {
                    "minecraft:block" => VibrationVariant::Block { position: BlockPosition::read_variant(cursor)? },
                    "minecraft:entity" => VibrationVariant::Entity {
                        entity_id: VarInt::read_variant(cursor)?,
                        entity_eye_height: f32::read(cursor)?,
                    },
                    other => VibrationVariant::Other { source_type: other },
                },
                ticks: i32::read(cursor)?,
            },
            0..=87 => unsafe {
                std::mem::transmute({
                    let mut arr = MaybeUninit::<[u8; std::mem::size_of::<Self>()]>::uninit().assume_init();
                    arr[0] = id as u8;
                    arr
                })
            },
            _ => Err(ProtocolError::Any(anyhow::Error::msg("Bad particle id")))?,
        })
    }

    pub const fn get_id(&self) -> i32 {
        (unsafe { (&*(self as *const Self as *const () as *const [u8; std::mem::size_of::<Self>()]))[0] }) as i32
    }

    pub fn write_data<W: ProtocolWriter>(&self, writer: &mut W) -> anyhow::Result<()> {
        match self {
            Self::Block { block_state } => VarInt::write_variant(block_state, writer),
            Self::BlockMarker { block_state } => VarInt::write_variant(block_state, writer),
            Self::Dust { red, green, blue, scale, } => {
                red.write(writer)?;
                green.write(writer)?;
                blue.write(writer)?;
                scale.write(writer)
            }
            Self::DustColorTransition {
                from_red, from_green, from_blue, scale,
                to_red, to_green, to_blue,
            } => {
                from_red.write(writer)?;
                from_green.write(writer)?;
                from_blue.write(writer)?;
                scale.write(writer)?;
                to_red.write(writer)?;
                to_green.write(writer)?;
                to_blue.write(writer)
            },
            Self::Item { slot } => slot.write(writer),
            Self::Vibration { variant, ticks } => {
                match variant {
                    VibrationVariant::Block { position } => {
                        "minecraft:block".write(writer)?;
                        BlockPosition::write_variant(position, writer)?
                    },
                    VibrationVariant::Entity { entity_id, entity_eye_height } => {
                        "minecraft:entity".write(writer)?;
                        VarInt::write_variant(entity_id, writer)?;
                        entity_eye_height.write(writer)?
                    }
                    VibrationVariant::Other { source_type } => source_type.write(writer)?,
                };
                ticks.write(writer)
            },
            _ => Ok(())
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum VibrationVariant<'a> {
    Block {
        position: Vector3D<i32>
    },
    Entity {
        entity_id: i32,
        entity_eye_height: f32,
    },
    Other {
        source_type: &'a str,
    },
}

impl<'a> ProtocolSize for VibrationVariant<'a> {
    const SIZE: Range<u32> = add_protocol_sizes_ty!(&str).start..add_protocol_sizes_ty!(&str, Vector3D<i32>).end;
}

#[derive(ProtocolPacket, Clone, Copy, Debug)]
#[bp(id = 0x22, state = Play, bound = Client)]
pub struct ParticlePS2C<'a> {
    pub particle: Particle<'a>,
    pub long_distance: bool,
    pub position: Vector3D<f64>,
    pub offset: Vector3D<f32>,
    pub max_speed: f32,
    pub particle_count: i32,
}

impl<'a> ProtocolSize for ParticlePS2C<'a> {
    const SIZE: Range<u32> = add_protocol_sizes_ty!(Particle, bool, Vector3D<f64>, Vector3D<f32>, f32, i32);
}

impl<'a> ProtocolWritable for ParticlePS2C<'a> {
    fn write<W: ProtocolWriter>(&self, writer: &mut W) -> anyhow::Result<()> {
        VarInt::write_variant(&self.particle.get_id(), writer)?;
        self.long_distance.write(writer)?;
        self.position.write(writer)?;
        self.offset.write(writer)?;
        self.max_speed.write(writer)?;
        self.particle_count.write(writer)?;
        self.particle.write_data(writer)
    }
}

impl<'a> ProtocolReadable<'a> for ParticlePS2C<'a> {
    fn read<C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<Self> {
        let particle_id = VarInt::read_variant(cursor)?;
        Ok(Self {
            long_distance: bool::read(cursor)?,
            position: Vector3D::read(cursor)?,
            offset: Vector3D::read(cursor)?,
            max_speed: f32::read(cursor)?,
            particle_count: i32::read(cursor)?,
            particle: Particle::read(particle_id, cursor)?,
        })
    }
}

#[derive(ProtocolAll, ProtocolPacket, Clone, Debug)]
#[bp(id = 0x23, state = Play, bound = Client)]
pub struct UpdateLightPS2C<'a> {
    pub chunk: Vector2D<i32>,
    pub light_data: LightData<'a>,
}

#[derive(ProtocolAll, Clone, Copy, Debug)]
#[bp(ty = i8)]
pub enum PreviousLoginGameMode {
    #[bp(value = -1)]
    None,
    Survival,
    Creative,
    Adventure,
    Spectator,
}

#[derive(ProtocolAll, Clone, Copy, Debug)]
#[bp(ty = u8)]
pub enum LoginGameMode {
    Survival,
    Creative,
    Adventure,
    Spectator
}

#[derive(ProtocolAll, Clone, Debug)]
pub struct LoginDeathLocation<'a> {
    pub dimension_name: Identifier<'a>,
    #[bp(variant = BlockPosition)]
    pub location: Vector3D<i32>,
}

#[derive(ProtocolAll, ProtocolPacket, Clone, Debug)]
#[bp(id = 0x24, state = Play, bound = Client)]
pub struct LoginPS2C<'a> {
    pub entity_id: i32,
    pub is_hardcore: bool,
    pub game_mode: LoginGameMode,
    pub previous_game_mode: PreviousLoginGameMode,
    #[bp(variant = "LengthProvidedArray<i32, VarInt, Identifier<'a>, Identifier<'a>>")]
    pub dimensions: Cow<'a, [Identifier<'a>]>,
    #[bp(variant = NbtBytes)]
    pub registry_codec: &'a [u8],
    pub dimension_type: Identifier<'a>,
    pub dimension_name: Identifier<'a>,
    pub hashed_seed: i64,
    #[bp(variant = VarInt)]
    pub max_players: i32,
    #[bp(variant = VarInt)]
    pub view_distance: i32,
    #[bp(variant = VarInt)]
    pub simulation_distance: i32,
    pub reduced_debug_info: bool,
    pub enable_respawn_screen: bool,
    pub is_debug: bool,
    pub is_flat: bool,
    pub death_location: Option<LoginDeathLocation<'a>>,
}

#[derive(BirdNBT, Clone, Debug)]
pub struct LoginRegistryCodec {
    a: i8,
    b: i16,
    c: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gap_compact_longs_reader_test() {
        let mut compact_longs_reader = unsafe {
            GapCompactLongsReader::<_, 19>::new(
                vec![0b111111111_001111111_000011111_000000111_000000001_0; 3].into_iter(),
                9,
            ).unwrap()
        };
        for i in 0..3 {
            assert_eq!(compact_longs_reader.next(), Some(0b1));
            assert_eq!(compact_longs_reader.next(), Some(0b111));
            assert_eq!(compact_longs_reader.next(), Some(0b11111));
            assert_eq!(compact_longs_reader.next(), Some(0b1111111));
            assert_eq!(compact_longs_reader.next(), Some(0b111111111));
            if i == 2 {
                assert_eq!(compact_longs_reader.next(), None);
            } else {
                assert_eq!(compact_longs_reader.next(), Some(0b0));
                assert_eq!(compact_longs_reader.next(), Some(0b0));
            }
        }
    }

    #[test]
    fn gap_compact_longs_writer_test() {
        let mut vec = Vec::new();
        let mut compact_longs_writer = unsafe { GapCompactLongsWriter::new(&mut vec, 9) };
        unsafe {
            for i in 0..3 {
                compact_longs_writer.write(0b1).unwrap();
                compact_longs_writer.write(0b111).unwrap();
                compact_longs_writer.write(0b11111).unwrap();
                compact_longs_writer.write(0b1111111).unwrap();
                compact_longs_writer.write(0b111111111).unwrap();
                if i != 2 {
                    compact_longs_writer.write(0b0).unwrap();
                    compact_longs_writer.write(0b0).unwrap();
                }
            }
        }
        compact_longs_writer.finish().unwrap();
        let mut res_vec = Vec::new();
        for _ in 0..3 {
            0b111111111_001111111_000011111_000000111_000000001_0_u64.write(&mut res_vec).unwrap();
        }
        assert_eq!(vec, res_vec);
    }

    #[test]
    fn gap_compact_longs_length_test() {
        unsafe {
            assert_eq!(compact_longs_array_length(11, 15), 3);
            assert_eq!(compact_longs_array_length(12, 15), 3);
            assert_eq!(compact_longs_array_length(13, 15), 4);
            assert_eq!(compact_longs_array_length(14, 15), 4);
            assert_eq!(compact_longs_array_length(15, 15), 4);
            assert_eq!(compact_longs_array_length(16, 15), 4);
        }
    }

    #[test]
    fn bit_set_test() {
        let mut owned_bit_set = OwnedBitSet::new();
        owned_bit_set.set(0);
        owned_bit_set.set(3);
        {
            let bit_set = owned_bit_set.get_bit_set();
            assert_eq!(bit_set.get(0), Some(true));
            assert_eq!(bit_set.get(3), Some(true));
            assert_eq!(bit_set.get(1), Some(false));
            assert_eq!(bit_set.get(64), None);
            let mut iter = bit_set.long_iter();
            assert_eq!(iter.next(), Some(0b1001));
            assert_eq!(iter.next(), None);
        }
        owned_bit_set.set(64);
        owned_bit_set.set(65);
        owned_bit_set.set(67);
        {
            let bit_set = owned_bit_set.get_bit_set();
            assert_eq!(bit_set.get(0), Some(true));
            assert_eq!(bit_set.get(3), Some(true));
            assert_eq!(bit_set.get(1), Some(false));
            assert_eq!(bit_set.get(64), Some(true));
            assert_eq!(bit_set.get(65), Some(true));
            assert_eq!(bit_set.get(67), Some(true));
            assert_eq!(bit_set.get(66), Some(false));
            let mut iter = bit_set.long_iter();
            assert_eq!(iter.next(), Some(0b1001));
            assert_eq!(iter.next(), Some(0b1011));
            assert_eq!(iter.next(), None);
            let mut raw = Vec::new();
            bit_set.long_iter().for_each(|n| n.write(&mut raw).unwrap());
            let bit_set = unsafe { BitSet::new_raw(&raw) };
            for i in 0..127 {
                assert_eq!(
                    bit_set.get(i),
                    Some(match i {
                        0 | 3 | 64 | 65 | 67 => true,
                        _ => false,
                    })
                )
            }
            assert_eq!(bit_set.get(128), None);
        }
    }

    #[test]
    fn light_array_test() {
        unsafe {
            let mut owned_light_array = OwnedLightArray::new();
            assert_eq!(owned_light_array.is_empty(), true);
            owned_light_array.set(Vector3D::new(0, 0, 0), 15);
            owned_light_array.set(Vector3D::new(1, 1, 1), 13);
            owned_light_array.set(Vector3D::new(2, 1, 1), 11);
            {
                let light_array = owned_light_array.as_light_array();
                assert_eq!(light_array.get(Vector3D::new(0, 0, 0)), 15);
                assert_eq!(light_array.get(Vector3D::new(1, 1, 1)), 13);
                assert_eq!(light_array.get(Vector3D::new(2, 1, 1)), 11);
                assert_eq!(light_array.get(Vector3D::new(2, 2, 1)), 0);
                assert_eq!(light_array.get_bytes()[0], 15);
            }
            owned_light_array.set(Vector3D::new(0, 0, 0), 0);
            owned_light_array.set(Vector3D::new(1, 1, 1), 0);
            owned_light_array.set(Vector3D::new(2, 1, 1), 0);
            assert_eq!(owned_light_array.is_empty(), true);
        }
    }

    #[test]
    fn particle_test() {
        let mut empty_slice = [].as_slice();
        let mut zero_slice = [0].as_slice();
        assert_eq!(Particle::read(83, &mut empty_slice).unwrap(), Particle::Glow);
        assert_eq!(Particle::read(37, &mut empty_slice).unwrap(), Particle::ItemSlime);
        assert_eq!(Particle::read(2, &mut zero_slice).unwrap(), Particle::Block { block_state: 0 });
        assert_eq!(Particle::Glow.get_id(), 83);
        assert_eq!(Particle::ItemSlime.get_id(), 37);
        assert_eq!(Particle::Block { block_state: 2 }.get_id(), 2);
    }
}
