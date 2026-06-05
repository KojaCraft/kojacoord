use bytes::{Buf, BufMut, Bytes, BytesMut};

use crate::codec::{Decode, Encode, PacketId};
use crate::error::ProtocolError;
use crate::types::slot::LegacySlot;
use crate::types::VarInt;

pub use packets::{
    ClientboundAwardStats, ClientboundBlockAction, ClientboundBlockDestroyStage,
    ClientboundBlockEntityData, ClientboundBlockUpdate, ClientboundChangeDifficulty,
    ClientboundChatMessage, ClientboundCommandSuggestions, ClientboundContainerClose,
    ClientboundContainerSetContent, ClientboundContainerSetProperty, ClientboundContainerSetSlot,
    ClientboundDisconnect, ClientboundEntityAnimation, ClientboundEntityEvent,
    ClientboundExplosion, ClientboundForgetLevelChunk, ClientboundGameEvent, ClientboundJoinGame,
    ClientboundKeepAlive, ClientboundLevelChunkWithLight, ClientboundLevelEvent,
    ClientboundLevelParticles, ClientboundMapItemData, ClientboundMoveEntityPos,
    ClientboundMoveEntityPosRot, ClientboundMoveEntityRot, ClientboundOpenScreen,
    ClientboundOpenSignEditor, ClientboundPlayerAbilities, ClientboundPlayerCombatEnd,
    ClientboundPlayerCombatEnter, ClientboundPlayerCombatKill, ClientboundPlayerInfoUpdate,
    ClientboundPlayerPosition, ClientboundPluginMessage, ClientboundRemoveEntities,
    ClientboundRemoveEntityEffect, ClientboundResetScore, ClientboundResourcePackPush,
    ClientboundRespawn, ClientboundRotateHead, ClientboundSectionBlocksUpdate,
    ClientboundSetEntityLink, ClientboundSetEntityMotion, ClientboundSetEquipment,
    ClientboundSetExperience, ClientboundSetHealth, ClientboundSetHeldItem,
    ClientboundSetScoreboardObjective, ClientboundSetScoreboardScore, ClientboundSetTime,
    ClientboundSound, ClientboundSpawnEntity, ClientboundSpawnExperienceOrb, ClientboundSpawnMob,
    ClientboundSpawnPlayer, ClientboundTabList, ClientboundTakeItemEntity,
    ClientboundTeleportEntity, ClientboundUpdateAttributes, ClientboundUpdateEffects,
    InteractAction, ServerboundAnimation, ServerboundChatMessage, ServerboundClickWindow,
    ServerboundClickWindowButton, ServerboundClientSettings, ServerboundClientStatus,
    ServerboundCloseWindow, ServerboundCommandSuggestion, ServerboundConfirmTransaction,
    ServerboundCreativeInventoryAction, ServerboundEnchantItem, ServerboundEntityAction,
    ServerboundHeldItemChange, ServerboundInteract, ServerboundKeepAlive, ServerboundMovePlayerPos,
    ServerboundMovePlayerPosRot, ServerboundMovePlayerRot, ServerboundMovePlayerStatusOnly,
    ServerboundPlayerAbilities, ServerboundPlayerAction, ServerboundPlayerBlockPlacement,
    ServerboundPluginMessage, ServerboundResourcePackStatus, ServerboundSteerVehicle,
    ServerboundUpdateSign,
};

mod packets {
    use super::*;

    fn encode_str(s: &str, dst: &mut BytesMut) -> Result<(), ProtocolError> {
        let bytes = s.as_bytes();
        VarInt(bytes.len() as i32).encode(dst)?;
        dst.put_slice(bytes);
        Ok(())
    }

    fn decode_str(src: &mut Bytes, ctx: &'static str) -> Result<String, ProtocolError> {
        let len = VarInt::decode(src)?.0 as usize;
        if src.remaining() < len {
            return Err(ProtocolError::Io(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                format!("Missing bytes for {ctx}"),
            )));
        }
        let mut b = vec![0u8; len];
        src.copy_to_slice(&mut b);
        String::from_utf8(b).map_err(|_| {
            ProtocolError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Invalid UTF-8 in {ctx}"),
            ))
        })
    }

    fn need(src: &Bytes, n: usize) -> Result<(), ProtocolError> {
        if src.remaining() < n {
            Err(ProtocolError::Io(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                format!("Need {n} bytes, have {}", src.remaining()),
            )))
        } else {
            Ok(())
        }
    }

    // ── Raw opaque packet macro ───────────────────────────────────────────────

    macro_rules! raw_packet {
        ($name:ident, $id:expr) => {
            #[derive(Debug, Clone, PartialEq)]
            pub struct $name {
                pub raw: Vec<u8>,
            }
            impl PacketId for $name {
                fn packet_id(_ver: u32) -> u8 {
                    $id
                }
            }
            impl Encode for $name {
                fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
                    dst.put_slice(&self.raw);
                    Ok(())
                }
            }
            impl Decode for $name {
                fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
                    let len = src.remaining();
                    let mut raw = vec![0u8; len];
                    src.copy_to_slice(&mut raw);
                    Ok(Self { raw })
                }
            }
        };
    }

    // ── KeepAlive (0x00 / 0x00) ───────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundKeepAlive {
        pub keep_alive_id: VarInt,
    }

    impl PacketId for ClientboundKeepAlive {
        fn packet_id(_ver: u32) -> u8 {
            0x00
        }
    }

    impl Encode for ClientboundKeepAlive {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.keep_alive_id.encode(dst)
        }
    }

    impl Decode for ClientboundKeepAlive {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            Ok(Self {
                keep_alive_id: VarInt::decode(src)?,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ServerboundKeepAlive {
        pub keep_alive_id: VarInt,
    }

    impl PacketId for ServerboundKeepAlive {
        fn packet_id(_ver: u32) -> u8 {
            0x00
        }
    }

    impl Encode for ServerboundKeepAlive {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.keep_alive_id.encode(dst)
        }
    }

    impl Decode for ServerboundKeepAlive {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            Ok(Self {
                keep_alive_id: VarInt::decode(src)?,
            })
        }
    }

    // ── JoinGame (0x01) ───────────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundJoinGame {
        pub entity_id: i32,
        pub game_mode: u8,
        pub dimension: i8,
        pub difficulty: u8,
        pub max_players: u8,
        pub level_type: String,
        pub reduced_debug_info: bool,
    }

    impl PacketId for ClientboundJoinGame {
        fn packet_id(_ver: u32) -> u8 {
            0x01
        }
    }

    impl Encode for ClientboundJoinGame {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_i32(self.entity_id);
            dst.put_u8(self.game_mode);
            dst.put_i8(self.dimension);
            dst.put_u8(self.difficulty);
            dst.put_u8(self.max_players);
            encode_str(&self.level_type, dst)?;
            dst.put_u8(self.reduced_debug_info as u8);
            Ok(())
        }
    }

    impl Decode for ClientboundJoinGame {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 4 + 1 + 1 + 1 + 1)?;
            let entity_id = src.get_i32();
            let game_mode = src.get_u8();
            let dimension = src.get_i8();
            let difficulty = src.get_u8();
            let max_players = src.get_u8();
            let level_type = decode_str(src, "ClientboundJoinGame level_type")?;
            need(src, 1)?;
            let reduced_debug_info = src.get_u8() != 0;
            Ok(Self {
                entity_id,
                game_mode,
                dimension,
                difficulty,
                max_players,
                level_type,
                reduced_debug_info,
            })
        }
    }

    // ── ChatMessage (0x02 / 0x01) ─────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundChatMessage {
        pub json_message: String,
        pub position: u8,
    }

    impl PacketId for ClientboundChatMessage {
        fn packet_id(_ver: u32) -> u8 {
            0x02
        }
    }

    impl Encode for ClientboundChatMessage {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            encode_str(&self.json_message, dst)?;
            dst.put_u8(self.position);
            Ok(())
        }
    }

    impl Decode for ClientboundChatMessage {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let json_message = decode_str(src, "ClientboundChatMessage json_message")?;
            need(src, 1)?;
            Ok(Self {
                json_message,
                position: src.get_u8(),
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ServerboundChatMessage {
        pub message: String,
    }

    impl PacketId for ServerboundChatMessage {
        fn packet_id(_ver: u32) -> u8 {
            0x01
        }
    }

    impl Encode for ServerboundChatMessage {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            encode_str(&self.message, dst)
        }
    }

    impl Decode for ServerboundChatMessage {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            Ok(Self {
                message: decode_str(src, "ServerboundChatMessage message")?,
            })
        }
    }

    // ── Respawn (0x07) ────────────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundRespawn {
        pub dimension: i32,
        pub difficulty: u8,
        pub game_mode: u8,
        pub level_type: String,
    }

    impl PacketId for ClientboundRespawn {
        fn packet_id(_ver: u32) -> u8 {
            0x07
        }
    }

    impl Encode for ClientboundRespawn {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_i32(self.dimension);
            dst.put_u8(self.difficulty);
            dst.put_u8(self.game_mode);
            encode_str(&self.level_type, dst)
        }
    }

    impl Decode for ClientboundRespawn {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 4 + 1 + 1)?;
            let dimension = src.get_i32();
            let difficulty = src.get_u8();
            let game_mode = src.get_u8();
            let level_type = decode_str(src, "ClientboundRespawn level_type")?;
            Ok(Self {
                dimension,
                difficulty,
                game_mode,
                level_type,
            })
        }
    }

    // ── PlayerPosition (0x08) ─────────────────────────────────────────────────
    // 1.7.10 uses head_y instead of feet_y and has no teleport_id.

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundPlayerPosition {
        pub x: f64,
        pub head_y: f64,
        pub z: f64,
        pub yaw: f32,
        pub pitch: f32,
        pub flags: u8,
    }

    impl PacketId for ClientboundPlayerPosition {
        fn packet_id(_ver: u32) -> u8 {
            0x08
        }
    }

    impl Encode for ClientboundPlayerPosition {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_f64(self.x);
            dst.put_f64(self.head_y);
            dst.put_f64(self.z);
            dst.put_f32(self.yaw);
            dst.put_f32(self.pitch);
            dst.put_u8(self.flags);
            Ok(())
        }
    }

    impl Decode for ClientboundPlayerPosition {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 8 + 8 + 8 + 4 + 4 + 1)?;
            Ok(Self {
                x: src.get_f64(),
                head_y: src.get_f64(),
                z: src.get_f64(),
                yaw: src.get_f32(),
                pitch: src.get_f32(),
                flags: src.get_u8(),
            })
        }
    }

    // ── Disconnect (0x40) ─────────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundDisconnect {
        pub reason: String,
    }

    impl PacketId for ClientboundDisconnect {
        fn packet_id(_ver: u32) -> u8 {
            0x40
        }
    }

    impl Encode for ClientboundDisconnect {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            encode_str(&self.reason, dst)
        }
    }

    impl Decode for ClientboundDisconnect {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            Ok(Self {
                reason: decode_str(src, "ClientboundDisconnect reason")?,
            })
        }
    }

    // ── PluginMessage (0x3F / 0x17) ───────────────────────────────────────────
    // 1.7.10 uses a signed i16 length prefix for the data payload, not VarInt.

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundPluginMessage {
        pub channel: String,
        pub data: Vec<u8>,
    }

    impl PacketId for ClientboundPluginMessage {
        fn packet_id(_ver: u32) -> u8 {
            0x3F
        }
    }

    impl Encode for ClientboundPluginMessage {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            encode_str(&self.channel, dst)?;
            dst.put_i16(self.data.len() as i16);
            dst.put_slice(&self.data);
            Ok(())
        }
    }

    impl Decode for ClientboundPluginMessage {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let channel = decode_str(src, "ClientboundPluginMessage channel")?;
            need(src, 2)?;
            let data_len = src.get_i16() as usize;
            need(src, data_len)?;
            let mut data = vec![0u8; data_len];
            src.copy_to_slice(&mut data);
            Ok(Self { channel, data })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ServerboundPluginMessage {
        pub channel: String,
        pub data: Vec<u8>,
    }

    impl PacketId for ServerboundPluginMessage {
        fn packet_id(_ver: u32) -> u8 {
            0x17
        }
    }

    impl Encode for ServerboundPluginMessage {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            encode_str(&self.channel, dst)?;
            dst.put_i16(self.data.len() as i16);
            dst.put_slice(&self.data);
            Ok(())
        }
    }

    impl Decode for ServerboundPluginMessage {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let channel = decode_str(src, "ServerboundPluginMessage channel")?;
            need(src, 2)?;
            let data_len = src.get_i16() as usize;
            need(src, data_len)?;
            let mut data = vec![0u8; data_len];
            src.copy_to_slice(&mut data);
            Ok(Self { channel, data })
        }
    }

    // ── Interact (0x02) ───────────────────────────────────────────────────────
    // 1.7.10: no hand field in Interact / InteractAt variants.

    #[derive(Debug, Clone, PartialEq)]
    pub enum InteractAction {
        Interact,
        Attack,
        InteractAt {
            target_x: f32,
            target_y: f32,
            target_z: f32,
        },
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ServerboundInteract {
        pub entity_id: VarInt,
        pub action: InteractAction,
    }

    impl PacketId for ServerboundInteract {
        fn packet_id(_ver: u32) -> u8 {
            0x02
        }
    }

    impl Encode for ServerboundInteract {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.entity_id.encode(dst)?;
            match &self.action {
                InteractAction::Interact => {
                    VarInt(0).encode(dst)?;
                },
                InteractAction::Attack => {
                    VarInt(1).encode(dst)?;
                },
                InteractAction::InteractAt {
                    target_x,
                    target_y,
                    target_z,
                } => {
                    VarInt(2).encode(dst)?;
                    dst.put_f32(*target_x);
                    dst.put_f32(*target_y);
                    dst.put_f32(*target_z);
                },
            }
            Ok(())
        }
    }

    impl Decode for ServerboundInteract {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let entity_id = VarInt::decode(src)?;
            let action = match VarInt::decode(src)?.0 {
                0 => InteractAction::Interact,
                1 => InteractAction::Attack,
                2 => {
                    need(src, 4 + 4 + 4)?;
                    InteractAction::InteractAt {
                        target_x: src.get_f32(),
                        target_y: src.get_f32(),
                        target_z: src.get_f32(),
                    }
                },
                _ => {
                    return Err(ProtocolError::Io(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "Unknown ServerboundInteract action type",
                    )))
                },
            };
            Ok(Self { entity_id, action })
        }
    }

    // ── Movement (0x03–0x06) ──────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ServerboundMovePlayerPos {
        pub x: f64,
        pub feet_y: f64,
        pub z: f64,
        pub on_ground: bool,
    }

    impl PacketId for ServerboundMovePlayerPos {
        fn packet_id(_ver: u32) -> u8 {
            0x04
        }
    }

    impl Encode for ServerboundMovePlayerPos {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_f64(self.x);
            dst.put_f64(self.feet_y);
            dst.put_f64(self.z);
            dst.put_u8(self.on_ground as u8);
            Ok(())
        }
    }

    impl Decode for ServerboundMovePlayerPos {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 8 + 8 + 8 + 1)?;
            Ok(Self {
                x: src.get_f64(),
                feet_y: src.get_f64(),
                z: src.get_f64(),
                on_ground: src.get_u8() != 0,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ServerboundMovePlayerRot {
        pub yaw: f32,
        pub pitch: f32,
        pub on_ground: bool,
    }

    impl PacketId for ServerboundMovePlayerRot {
        fn packet_id(_ver: u32) -> u8 {
            0x05
        }
    }

    impl Encode for ServerboundMovePlayerRot {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_f32(self.yaw);
            dst.put_f32(self.pitch);
            dst.put_u8(self.on_ground as u8);
            Ok(())
        }
    }

    impl Decode for ServerboundMovePlayerRot {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 4 + 4 + 1)?;
            Ok(Self {
                yaw: src.get_f32(),
                pitch: src.get_f32(),
                on_ground: src.get_u8() != 0,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ServerboundMovePlayerPosRot {
        pub x: f64,
        pub feet_y: f64,
        pub z: f64,
        pub yaw: f32,
        pub pitch: f32,
        pub on_ground: bool,
    }

    impl PacketId for ServerboundMovePlayerPosRot {
        fn packet_id(_ver: u32) -> u8 {
            0x06
        }
    }

    impl Encode for ServerboundMovePlayerPosRot {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_f64(self.x);
            dst.put_f64(self.feet_y);
            dst.put_f64(self.z);
            dst.put_f32(self.yaw);
            dst.put_f32(self.pitch);
            dst.put_u8(self.on_ground as u8);
            Ok(())
        }
    }

    impl Decode for ServerboundMovePlayerPosRot {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 8 + 8 + 8 + 4 + 4 + 1)?;
            Ok(Self {
                x: src.get_f64(),
                feet_y: src.get_f64(),
                z: src.get_f64(),
                yaw: src.get_f32(),
                pitch: src.get_f32(),
                on_ground: src.get_u8() != 0,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundSpawnMob {
        pub entity_id: VarInt,
        pub kind: u8, // Wither ID is strictly 64 in Minecraft 1.7.10
        pub x: i32,   // (Absolute X Coordinate) * 32
        pub y: i32,   // (Absolute Y Coordinate) * 32
        pub z: i32,   // (Absolute Z Coordinate) * 32
        pub pitch: u8,
        pub yaw: u8,
        pub head_pitch: u8,
        pub velocity_x: i16,
        pub velocity_y: i16,
        pub velocity_z: i16,
        pub metadata: Vec<u8>, // Raw 1.7.10 DataWatcher Metadata sequence terminating with 0x7F
    }

    impl PacketId for ClientboundSpawnMob {
        fn packet_id(_ver: u32) -> u8 {
            0x0F // ID for SpawnMob in Protocol 5 (1.7.10)
        }
    }

    impl Encode for ClientboundSpawnMob {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.entity_id.encode(dst)?;
            dst.put_u8(self.kind);
            dst.put_i32(self.x);
            dst.put_i32(self.y);
            dst.put_i32(self.z);
            dst.put_u8(self.pitch);
            dst.put_u8(self.yaw);
            dst.put_u8(self.head_pitch);
            dst.put_i16(self.velocity_x);
            dst.put_i16(self.velocity_y);
            dst.put_i16(self.velocity_z);

            // Write trailing 1.7.10 style metadata fields directly
            dst.put_slice(&self.metadata);
            Ok(())
        }
    }

    // ── Set Equipment (0x04) ─────────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundSetEquipment {
        pub entity_id: i32,
        pub slot: i16,
        pub item: LegacySlot,
    }

    impl PacketId for ClientboundSetEquipment {
        fn packet_id(_ver: u32) -> u8 {
            0x04
        }
    }

    impl Encode for ClientboundSetEquipment {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_i32(self.entity_id);
            dst.put_i16(self.slot);
            self.item.encode(dst)?;
            Ok(())
        }
    }

    impl Decode for ClientboundSetEquipment {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 4 + 2)?;
            let entity_id = src.get_i32();
            let slot = src.get_i16();
            let item = LegacySlot::decode(src)?;
            Ok(Self {
                entity_id,
                slot,
                item,
            })
        }
    }

    // ── Entity Animation (0x0B) ───────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundEntityAnimation {
        pub entity_id: i32,
        pub animation: i8,
    }

    impl PacketId for ClientboundEntityAnimation {
        fn packet_id(_ver: u32) -> u8 {
            0x0B
        }
    }

    impl Encode for ClientboundEntityAnimation {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_i32(self.entity_id);
            dst.put_i8(self.animation);
            Ok(())
        }
    }

    impl Decode for ClientboundEntityAnimation {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 4 + 1)?;
            Ok(Self {
                entity_id: src.get_i32(),
                animation: src.get_i8(),
            })
        }
    }

    // ── Take Item Entity (0x0D) ───────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundTakeItemEntity {
        pub collected_entity_id: i32,
        pub collector_entity_id: i32,
    }

    impl PacketId for ClientboundTakeItemEntity {
        fn packet_id(_ver: u32) -> u8 {
            0x0D
        }
    }

    impl Encode for ClientboundTakeItemEntity {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_i32(self.collected_entity_id);
            dst.put_i32(self.collector_entity_id);
            Ok(())
        }
    }

    impl Decode for ClientboundTakeItemEntity {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 4 + 4)?;
            Ok(Self {
                collected_entity_id: src.get_i32(),
                collector_entity_id: src.get_i32(),
            })
        }
    }

    // ── Spawn Experience Orb (0x11) ────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundSpawnExperienceOrb {
        pub entity_id: i32,
        pub x: i32,
        pub y: i32,
        pub z: i32,
        pub count: i16,
    }

    impl PacketId for ClientboundSpawnExperienceOrb {
        fn packet_id(_ver: u32) -> u8 {
            0x11
        }
    }

    impl Encode for ClientboundSpawnExperienceOrb {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_i32(self.entity_id);
            dst.put_i32(self.x);
            dst.put_i32(self.y);
            dst.put_i32(self.z);
            dst.put_i16(self.count);
            Ok(())
        }
    }

    impl Decode for ClientboundSpawnExperienceOrb {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 4 + 4 + 4 + 4 + 2)?;
            Ok(Self {
                entity_id: src.get_i32(),
                x: src.get_i32(),
                y: src.get_i32(),
                z: src.get_i32(),
                count: src.get_i16(),
            })
        }
    }

    // ── Set Entity Motion (0x12) ───────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundSetEntityMotion {
        pub entity_id: i32,
        pub mot_x: i16,
        pub mot_y: i16,
        pub mot_z: i16,
    }

    impl PacketId for ClientboundSetEntityMotion {
        fn packet_id(_ver: u32) -> u8 {
            0x12
        }
    }

    impl Encode for ClientboundSetEntityMotion {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_i32(self.entity_id);
            dst.put_i16(self.mot_x);
            dst.put_i16(self.mot_y);
            dst.put_i16(self.mot_z);
            Ok(())
        }
    }

    impl Decode for ClientboundSetEntityMotion {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 4 + 2 + 2 + 2)?;
            Ok(Self {
                entity_id: src.get_i32(),
                mot_x: src.get_i16(),
                mot_y: src.get_i16(),
                mot_z: src.get_i16(),
            })
        }
    }

    // ── Remove Entities (0x13) ─────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundRemoveEntities {
        pub entity_ids: Vec<i32>,
    }

    impl PacketId for ClientboundRemoveEntities {
        fn packet_id(_ver: u32) -> u8 {
            0x13
        }
    }

    impl Encode for ClientboundRemoveEntities {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            VarInt(self.entity_ids.len() as i32).encode(dst)?;
            for id in &self.entity_ids {
                dst.put_i32(*id);
            }
            Ok(())
        }
    }

    impl Decode for ClientboundRemoveEntities {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let count = VarInt::decode(src)?.0 as usize;
            need(src, count * 4)?;
            let mut entity_ids = Vec::with_capacity(count);
            for _ in 0..count {
                entity_ids.push(src.get_i32());
            }
            Ok(Self { entity_ids })
        }
    }

    // ── Move Entity Pos (0x15) ───────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundMoveEntityPos {
        pub entity_id: i32,
        pub dx: i8,
        pub dy: i8,
        pub dz: i8,
    }

    impl PacketId for ClientboundMoveEntityPos {
        fn packet_id(_ver: u32) -> u8 {
            0x15
        }
    }

    impl Encode for ClientboundMoveEntityPos {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_i32(self.entity_id);
            dst.put_i8(self.dx);
            dst.put_i8(self.dy);
            dst.put_i8(self.dz);
            Ok(())
        }
    }

    impl Decode for ClientboundMoveEntityPos {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 4 + 1 + 1 + 1)?;
            Ok(Self {
                entity_id: src.get_i32(),
                dx: src.get_i8(),
                dy: src.get_i8(),
                dz: src.get_i8(),
            })
        }
    }

    // ── Move Entity Rot (0x16) ───────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundMoveEntityRot {
        pub entity_id: i32,
        pub yaw: i8,
        pub pitch: i8,
    }

    impl PacketId for ClientboundMoveEntityRot {
        fn packet_id(_ver: u32) -> u8 {
            0x16
        }
    }

    impl Encode for ClientboundMoveEntityRot {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_i32(self.entity_id);
            dst.put_i8(self.yaw);
            dst.put_i8(self.pitch);
            Ok(())
        }
    }

    impl Decode for ClientboundMoveEntityRot {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 4 + 1 + 1)?;
            Ok(Self {
                entity_id: src.get_i32(),
                yaw: src.get_i8(),
                pitch: src.get_i8(),
            })
        }
    }

    // ── Move Entity Pos Rot (0x17) ───────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundMoveEntityPosRot {
        pub entity_id: i32,
        pub dx: i8,
        pub dy: i8,
        pub dz: i8,
        pub yaw: i8,
        pub pitch: i8,
    }

    impl PacketId for ClientboundMoveEntityPosRot {
        fn packet_id(_ver: u32) -> u8 {
            0x17
        }
    }

    impl Encode for ClientboundMoveEntityPosRot {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_i32(self.entity_id);
            dst.put_i8(self.dx);
            dst.put_i8(self.dy);
            dst.put_i8(self.dz);
            dst.put_i8(self.yaw);
            dst.put_i8(self.pitch);
            Ok(())
        }
    }

    impl Decode for ClientboundMoveEntityPosRot {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 4 + 1 + 1 + 1 + 1 + 1)?;
            Ok(Self {
                entity_id: src.get_i32(),
                dx: src.get_i8(),
                dy: src.get_i8(),
                dz: src.get_i8(),
                yaw: src.get_i8(),
                pitch: src.get_i8(),
            })
        }
    }

    // ── Teleport Entity (0x18) ───────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundTeleportEntity {
        pub entity_id: i32,
        pub x: i32,
        pub y: i32,
        pub z: i32,
        pub yaw: i8,
        pub pitch: i8,
    }

    impl PacketId for ClientboundTeleportEntity {
        fn packet_id(_ver: u32) -> u8 {
            0x18
        }
    }

    impl Encode for ClientboundTeleportEntity {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_i32(self.entity_id);
            dst.put_i32(self.x);
            dst.put_i32(self.y);
            dst.put_i32(self.z);
            dst.put_i8(self.yaw);
            dst.put_i8(self.pitch);
            Ok(())
        }
    }

    impl Decode for ClientboundTeleportEntity {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 4 + 4 + 4 + 4 + 1 + 1)?;
            Ok(Self {
                entity_id: src.get_i32(),
                x: src.get_i32(),
                y: src.get_i32(),
                z: src.get_i32(),
                yaw: src.get_i8(),
                pitch: src.get_i8(),
            })
        }
    }

    // ── Rotate Head (0x19) ───────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundRotateHead {
        pub entity_id: i32,
        pub head_yaw: i8,
    }

    impl PacketId for ClientboundRotateHead {
        fn packet_id(_ver: u32) -> u8 {
            0x19
        }
    }

    impl Encode for ClientboundRotateHead {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_i32(self.entity_id);
            dst.put_i8(self.head_yaw);
            Ok(())
        }
    }

    impl Decode for ClientboundRotateHead {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 4 + 1)?;
            Ok(Self {
                entity_id: src.get_i32(),
                head_yaw: src.get_i8(),
            })
        }
    }

    // ── Entity Event (0x1A) ─────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundEntityEvent {
        pub entity_id: i32,
        pub event_id: i8,
    }

    impl PacketId for ClientboundEntityEvent {
        fn packet_id(_ver: u32) -> u8 {
            0x1A
        }
    }

    impl Encode for ClientboundEntityEvent {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_i32(self.entity_id);
            dst.put_i8(self.event_id);
            Ok(())
        }
    }

    impl Decode for ClientboundEntityEvent {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 4 + 1)?;
            Ok(Self {
                entity_id: src.get_i32(),
                event_id: src.get_i8(),
            })
        }
    }

    // ── Set Entity Link (0x1B) ───────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundSetEntityLink {
        pub entity_id: i32,
        pub vehicle_id: i32,
    }

    impl PacketId for ClientboundSetEntityLink {
        fn packet_id(_ver: u32) -> u8 {
            0x1B
        }
    }

    impl Encode for ClientboundSetEntityLink {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_i32(self.entity_id);
            dst.put_i32(self.vehicle_id);
            Ok(())
        }
    }

    impl Decode for ClientboundSetEntityLink {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 4 + 4)?;
            Ok(Self {
                entity_id: src.get_i32(),
                vehicle_id: src.get_i32(),
            })
        }
    }

    // ── Update Effects (0x1D) ───────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundUpdateEffects {
        pub entity_id: i32,
        pub effect_id: i8,
        pub amplifier: i8,
        pub duration: i16,
    }

    impl PacketId for ClientboundUpdateEffects {
        fn packet_id(_ver: u32) -> u8 {
            0x1D
        }
    }

    impl Encode for ClientboundUpdateEffects {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_i32(self.entity_id);
            dst.put_i8(self.effect_id);
            dst.put_i8(self.amplifier);
            dst.put_i16(self.duration);
            Ok(())
        }
    }

    impl Decode for ClientboundUpdateEffects {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 4 + 1 + 1 + 2)?;
            Ok(Self {
                entity_id: src.get_i32(),
                effect_id: src.get_i8(),
                amplifier: src.get_i8(),
                duration: src.get_i16(),
            })
        }
    }

    // ── Remove Entity Effect (0x1E) ───────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundRemoveEntityEffect {
        pub entity_id: i32,
        pub effect_id: i8,
    }

    impl PacketId for ClientboundRemoveEntityEffect {
        fn packet_id(_ver: u32) -> u8 {
            0x1E
        }
    }

    impl Encode for ClientboundRemoveEntityEffect {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_i32(self.entity_id);
            dst.put_i8(self.effect_id);
            Ok(())
        }
    }

    impl Decode for ClientboundRemoveEntityEffect {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 4 + 1)?;
            Ok(Self {
                entity_id: src.get_i32(),
                effect_id: src.get_i8(),
            })
        }
    }

    // ── Update Attributes (0x20) ───────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundUpdateAttributes {
        pub entity_id: i32,
        pub data: Vec<u8>,
    }

    impl PacketId for ClientboundUpdateAttributes {
        fn packet_id(_ver: u32) -> u8 {
            0x20
        }
    }

    impl Encode for ClientboundUpdateAttributes {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_i32(self.entity_id);
            dst.extend_from_slice(&self.data);
            Ok(())
        }
    }

    impl Decode for ClientboundUpdateAttributes {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 4)?;
            let entity_id = src.get_i32();
            let data = src.to_vec();
            Ok(Self { entity_id, data })
        }
    }

    // ── Level Chunk With Light (0x21) ───────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundLevelChunkWithLight {
        pub chunk_x: i32,
        pub chunk_z: i32,
        pub continuous: bool,
        pub primary_bitmap: i16,
        pub add_bitmap: i16,
        pub compressed_data: Vec<u8>,
    }

    impl PacketId for ClientboundLevelChunkWithLight {
        fn packet_id(_ver: u32) -> u8 {
            0x21
        }
    }

    impl Encode for ClientboundLevelChunkWithLight {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_i32(self.chunk_x);
            dst.put_i32(self.chunk_z);
            dst.put_u8(self.continuous as u8);
            dst.put_i16(self.primary_bitmap);
            dst.put_i16(self.add_bitmap);
            dst.put_i32(self.compressed_data.len() as i32);
            dst.extend_from_slice(&self.compressed_data);
            Ok(())
        }
    }

    impl Decode for ClientboundLevelChunkWithLight {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 4 + 4 + 1 + 2 + 2 + 4)?;
            let chunk_x = src.get_i32();
            let chunk_z = src.get_i32();
            let continuous = src.get_u8() != 0;
            let primary_bitmap = src.get_i16();
            let add_bitmap = src.get_i16();
            let data_len = src.get_i32() as usize;
            if src.remaining() < data_len {
                return Err(ProtocolError::UnexpectedEof);
            }
            let compressed_data = src.copy_to_bytes(data_len).to_vec();
            Ok(Self {
                chunk_x,
                chunk_z,
                continuous,
                primary_bitmap,
                add_bitmap,
                compressed_data,
            })
        }
    }

    // ── Section Blocks Update (0x22) ─────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundSectionBlocksUpdate {
        pub chunk_x: i32,
        pub chunk_z: i32,
        pub record_count: i16,
        pub data: Vec<u8>,
    }

    impl PacketId for ClientboundSectionBlocksUpdate {
        fn packet_id(_ver: u32) -> u8 {
            0x22
        }
    }

    impl Encode for ClientboundSectionBlocksUpdate {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_i32(self.chunk_x);
            dst.put_i32(self.chunk_z);
            dst.put_i16(self.record_count);
            dst.extend_from_slice(&self.data);
            Ok(())
        }
    }

    impl Decode for ClientboundSectionBlocksUpdate {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 4 + 4 + 2)?;
            let chunk_x = src.get_i32();
            let chunk_z = src.get_i32();
            let record_count = src.get_i16();
            let data = src.to_vec();
            Ok(Self {
                chunk_x,
                chunk_z,
                record_count,
                data,
            })
        }
    }

    // ── Block Action (0x24) ─────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundBlockAction {
        pub x: i32,
        pub y: i16,
        pub z: i32,
        pub action_id: i8,
        pub action_param: i8,
        pub block_type: i32,
    }

    impl PacketId for ClientboundBlockAction {
        fn packet_id(_ver: u32) -> u8 {
            0x24
        }
    }

    impl Encode for ClientboundBlockAction {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_i32(self.x);
            dst.put_i16(self.y);
            dst.put_i32(self.z);
            dst.put_i8(self.action_id);
            dst.put_i8(self.action_param);
            VarInt(self.block_type).encode(dst)?;
            Ok(())
        }
    }

    impl Decode for ClientboundBlockAction {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 4 + 2 + 4 + 1 + 1)?;
            Ok(Self {
                x: src.get_i32(),
                y: src.get_i16(),
                z: src.get_i32(),
                action_id: src.get_i8(),
                action_param: src.get_i8(),
                block_type: VarInt::decode(src)?.0 as i32,
            })
        }
    }

    // ── Block Destroy Stage (0x25) ─────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundBlockDestroyStage {
        pub x: i32,
        pub y: i8,
        pub z: i32,
        pub stage: i8,
    }

    impl PacketId for ClientboundBlockDestroyStage {
        fn packet_id(_ver: u32) -> u8 {
            0x25
        }
    }

    impl Encode for ClientboundBlockDestroyStage {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_i32(self.x);
            dst.put_i8(self.y);
            dst.put_i32(self.z);
            dst.put_i8(self.stage);
            Ok(())
        }
    }

    impl Decode for ClientboundBlockDestroyStage {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 4 + 1 + 4 + 1)?;
            Ok(Self {
                x: src.get_i32(),
                y: src.get_i8(),
                z: src.get_i32(),
                stage: src.get_i8(),
            })
        }
    }

    // ── Forget Level Chunk (0x26) ───────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundForgetLevelChunk {
        pub chunk_x: i32,
        pub chunk_z: i32,
    }

    impl PacketId for ClientboundForgetLevelChunk {
        fn packet_id(_ver: u32) -> u8 {
            0x26
        }
    }

    impl Encode for ClientboundForgetLevelChunk {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_i32(self.chunk_x);
            dst.put_i32(self.chunk_z);
            Ok(())
        }
    }

    impl Decode for ClientboundForgetLevelChunk {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 4 + 4)?;
            Ok(Self {
                chunk_x: src.get_i32(),
                chunk_z: src.get_i32(),
            })
        }
    }

    // ── Explosion (0x27) ─────────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundExplosion {
        pub x: f32,
        pub y: f32,
        pub z: f32,
        pub radius: f32,
        pub record_count: i32,
        pub records: Vec<u8>,
        pub player_motion_x: f32,
        pub player_motion_y: f32,
        pub player_motion_z: f32,
    }

    impl PacketId for ClientboundExplosion {
        fn packet_id(_ver: u32) -> u8 {
            0x27
        }
    }

    impl Encode for ClientboundExplosion {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_f32(self.x);
            dst.put_f32(self.y);
            dst.put_f32(self.z);
            dst.put_f32(self.radius);
            dst.put_i32(self.record_count);
            dst.extend_from_slice(&self.records);
            dst.put_f32(self.player_motion_x);
            dst.put_f32(self.player_motion_y);
            dst.put_f32(self.player_motion_z);
            Ok(())
        }
    }

    impl Decode for ClientboundExplosion {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 4 + 4 + 4 + 4 + 4 + 4 + 4 + 4)?;
            let x = src.get_f32();
            let y = src.get_f32();
            let z = src.get_f32();
            let radius = src.get_f32();
            let record_count = src.get_i32();
            let records_len = record_count as usize * 3;
            if src.remaining() < records_len + 12 {
                return Err(ProtocolError::UnexpectedEof);
            }
            let mut records = vec![0u8; records_len];
            src.copy_to_slice(&mut records);
            let player_motion_x = src.get_f32();
            let player_motion_y = src.get_f32();
            let player_motion_z = src.get_f32();
            Ok(Self {
                x,
                y,
                z,
                radius,
                record_count,
                records,
                player_motion_x,
                player_motion_y,
                player_motion_z,
            })
        }
    }

    // ── Level Event (0x28) ─────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundLevelEvent {
        pub event_id: i32,
        pub x: i32,
        pub y: i32,
        pub z: i32,
        pub data: i32,
    }

    impl PacketId for ClientboundLevelEvent {
        fn packet_id(_ver: u32) -> u8 {
            0x28
        }
    }

    impl Encode for ClientboundLevelEvent {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_i32(self.event_id);
            dst.put_i32(self.x);
            dst.put_i32(self.y);
            dst.put_i32(self.z);
            dst.put_i32(self.data);
            Ok(())
        }
    }

    impl Decode for ClientboundLevelEvent {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 4 + 4 + 4 + 4 + 4)?;
            Ok(Self {
                event_id: src.get_i32(),
                x: src.get_i32(),
                y: src.get_i32(),
                z: src.get_i32(),
                data: src.get_i32(),
            })
        }
    }

    // ── Level Particles (0x2A) ───────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundLevelParticles {
        pub particle_id: i32,
        pub x: f32,
        pub y: f32,
        pub z: f32,
        pub offset_x: f32,
        pub offset_y: f32,
        pub offset_z: f32,
        pub particle_speed: f32,
        pub particle_count: i32,
        pub data: Vec<u8>,
    }

    impl PacketId for ClientboundLevelParticles {
        fn packet_id(_ver: u32) -> u8 {
            0x2A
        }
    }

    impl Encode for ClientboundLevelParticles {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_i32(self.particle_id);
            dst.put_f32(self.x);
            dst.put_f32(self.y);
            dst.put_f32(self.z);
            dst.put_f32(self.offset_x);
            dst.put_f32(self.offset_y);
            dst.put_f32(self.offset_z);
            dst.put_f32(self.particle_speed);
            dst.put_i32(self.particle_count);
            dst.extend_from_slice(&self.data);
            Ok(())
        }
    }

    impl Decode for ClientboundLevelParticles {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 4 + 4 + 4 + 4 + 4 + 4 + 4 + 4 + 4)?;
            let particle_id = src.get_i32();
            let x = src.get_f32();
            let y = src.get_f32();
            let z = src.get_f32();
            let offset_x = src.get_f32();
            let offset_y = src.get_f32();
            let offset_z = src.get_f32();
            let particle_speed = src.get_f32();
            let particle_count = src.get_i32();
            let data = src.to_vec();
            Ok(Self {
                particle_id,
                x,
                y,
                z,
                offset_x,
                offset_y,
                offset_z,
                particle_speed,
                particle_count,
                data,
            })
        }
    }

    // ── Game Event (0x2B) ─────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundGameEvent {
        pub event: u8,
        pub data: f32,
    }

    impl PacketId for ClientboundGameEvent {
        fn packet_id(_ver: u32) -> u8 {
            0x2B
        }
    }

    impl Encode for ClientboundGameEvent {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_u8(self.event);
            dst.put_f32(self.data);
            Ok(())
        }
    }

    impl Decode for ClientboundGameEvent {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 1 + 4)?;
            Ok(Self {
                event: src.get_u8(),
                data: src.get_f32(),
            })
        }
    }

    // ── Open Screen (0x2D) ─────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundOpenScreen {
        pub window_id: i8,
        pub window_type: String,
        pub window_title: String,
        pub slots_count: i8,
    }

    impl PacketId for ClientboundOpenScreen {
        fn packet_id(_ver: u32) -> u8 {
            0x2D
        }
    }

    impl Encode for ClientboundOpenScreen {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_i8(self.window_id);
            encode_str(&self.window_type, dst)?;
            encode_str(&self.window_title, dst)?;
            dst.put_i8(self.slots_count);
            Ok(())
        }
    }

    impl Decode for ClientboundOpenScreen {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let window_id = src.get_i8();
            let window_type = decode_str(src, "ClientboundOpenScreen window_type")?;
            let window_title = decode_str(src, "ClientboundOpenScreen window_title")?;
            need(src, 1)?;
            let slots_count = src.get_i8();
            Ok(Self {
                window_id,
                window_type,
                window_title,
                slots_count,
            })
        }
    }

    // ── Container Close (0x2E) ─────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundContainerClose {
        pub window_id: i8,
    }

    impl PacketId for ClientboundContainerClose {
        fn packet_id(_ver: u32) -> u8 {
            0x2E
        }
    }

    impl Encode for ClientboundContainerClose {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_i8(self.window_id);
            Ok(())
        }
    }

    impl Decode for ClientboundContainerClose {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 1)?;
            Ok(Self {
                window_id: src.get_i8(),
            })
        }
    }

    // ── Container Set Property (0x31) ───────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundContainerSetProperty {
        pub window_id: i8,
        pub property: i16,
        pub value: i16,
    }

    impl PacketId for ClientboundContainerSetProperty {
        fn packet_id(_ver: u32) -> u8 {
            0x31
        }
    }

    impl Encode for ClientboundContainerSetProperty {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_i8(self.window_id);
            dst.put_i16(self.property);
            dst.put_i16(self.value);
            Ok(())
        }
    }

    impl Decode for ClientboundContainerSetProperty {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 1 + 2 + 2)?;
            Ok(Self {
                window_id: src.get_i8(),
                property: src.get_i16(),
                value: src.get_i16(),
            })
        }
    }

    // ── Set Time (0x03) ───────────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundSetTime {
        pub age: i64,
        pub time: i64,
    }

    impl PacketId for ClientboundSetTime {
        fn packet_id(_ver: u32) -> u8 {
            0x03
        }
    }

    impl Encode for ClientboundSetTime {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_i64(self.age);
            dst.put_i64(self.time);
            Ok(())
        }
    }

    impl Decode for ClientboundSetTime {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 8 + 8)?;
            Ok(Self {
                age: src.get_i64(),
                time: src.get_i64(),
            })
        }
    }

    // ── Set Health (0x06) ───────────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundSetHealth {
        pub health: f32,
        pub food: i16,
        pub food_saturation: f32,
    }

    impl PacketId for ClientboundSetHealth {
        fn packet_id(_ver: u32) -> u8 {
            0x06
        }
    }

    impl Encode for ClientboundSetHealth {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_f32(self.health);
            dst.put_i16(self.food);
            dst.put_f32(self.food_saturation);
            Ok(())
        }
    }

    impl Decode for ClientboundSetHealth {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 4 + 2 + 4)?;
            Ok(Self {
                health: src.get_f32(),
                food: src.get_i16(),
                food_saturation: src.get_f32(),
            })
        }
    }

    // ── Spawn Player (0x0C) ─────────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundSpawnPlayer {
        pub entity_id: i32,
        pub player_uuid: uuid::Uuid,
        pub username: String,
        pub x: i32,
        pub y: i32,
        pub z: i32,
        pub yaw: i8,
        pub pitch: i8,
        pub current_item: i16,
    }

    impl PacketId for ClientboundSpawnPlayer {
        fn packet_id(_ver: u32) -> u8 {
            0x0C
        }
    }

    impl Encode for ClientboundSpawnPlayer {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_i32(self.entity_id);
            let (hi, lo) = self.player_uuid.as_u64_pair();
            dst.put_i64(hi as i64);
            dst.put_i64(lo as i64);
            encode_str(&self.username, dst)?;
            dst.put_i32(self.x);
            dst.put_i32(self.y);
            dst.put_i32(self.z);
            dst.put_i8(self.yaw);
            dst.put_i8(self.pitch);
            dst.put_i16(self.current_item);
            Ok(())
        }
    }

    impl Decode for ClientboundSpawnPlayer {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 4 + 8 + 8)?;
            let entity_id = src.get_i32();
            let hi = src.get_i64() as u64;
            let lo = src.get_i64() as u64;
            let player_uuid = uuid::Uuid::from_u64_pair(hi, lo);
            let username = decode_str(src, "ClientboundSpawnPlayer username")?;
            need(src, 4 + 4 + 4 + 1 + 1 + 2)?;
            Ok(Self {
                entity_id,
                player_uuid,
                username,
                x: src.get_i32(),
                y: src.get_i32(),
                z: src.get_i32(),
                yaw: src.get_i8(),
                pitch: src.get_i8(),
                current_item: src.get_i16(),
            })
        }
    }

    // ── Spawn Entity (0x0E) ─────────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundSpawnEntity {
        pub entity_id: i32,
        pub entity_type: i8,
        pub x: i32,
        pub y: i32,
        pub z: i32,
        pub yaw: i8,
        pub pitch: i8,
        pub head_pitch: i8,
        pub velocity_x: i16,
        pub velocity_y: i16,
        pub velocity_z: i16,
    }

    impl PacketId for ClientboundSpawnEntity {
        fn packet_id(_ver: u32) -> u8 {
            0x0E
        }
    }

    impl Encode for ClientboundSpawnEntity {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_i32(self.entity_id);
            dst.put_i8(self.entity_type);
            dst.put_i32(self.x);
            dst.put_i32(self.y);
            dst.put_i32(self.z);
            dst.put_i8(self.yaw);
            dst.put_i8(self.pitch);
            dst.put_i8(self.head_pitch);
            dst.put_i16(self.velocity_x);
            dst.put_i16(self.velocity_y);
            dst.put_i16(self.velocity_z);
            Ok(())
        }
    }

    impl Decode for ClientboundSpawnEntity {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 4 + 1 + 4 + 4 + 4 + 1 + 1 + 1 + 2 + 2 + 2)?;
            Ok(Self {
                entity_id: src.get_i32(),
                entity_type: src.get_i8(),
                x: src.get_i32(),
                y: src.get_i32(),
                z: src.get_i32(),
                yaw: src.get_i8(),
                pitch: src.get_i8(),
                head_pitch: src.get_i8(),
                velocity_x: src.get_i16(),
                velocity_y: src.get_i16(),
                velocity_z: src.get_i16(),
            })
        }
    }

    // ── Set Experience (0x1F) ───────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundSetExperience {
        pub experience_bar: f32,
        pub level: i16,
        pub total_experience: i16,
    }

    impl PacketId for ClientboundSetExperience {
        fn packet_id(_ver: u32) -> u8 {
            0x1F
        }
    }

    impl Encode for ClientboundSetExperience {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_f32(self.experience_bar);
            dst.put_i16(self.level);
            dst.put_i16(self.total_experience);
            Ok(())
        }
    }

    impl Decode for ClientboundSetExperience {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 4 + 2 + 2)?;
            Ok(Self {
                experience_bar: src.get_f32(),
                level: src.get_i16(),
                total_experience: src.get_i16(),
            })
        }
    }

    // ── Block Update (0x23) ───────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundBlockUpdate {
        pub x: i32,
        pub y: i8,
        pub z: i32,
        pub block_id: i32,
        pub block_metadata: i8,
    }

    impl PacketId for ClientboundBlockUpdate {
        fn packet_id(_ver: u32) -> u8 {
            0x23
        }
    }

    impl Encode for ClientboundBlockUpdate {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_i32(self.x);
            dst.put_i8(self.y);
            dst.put_i32(self.z);
            VarInt(self.block_id).encode(dst)?;
            dst.put_i8(self.block_metadata);
            Ok(())
        }
    }

    impl Decode for ClientboundBlockUpdate {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 4 + 1 + 4)?;
            Ok(Self {
                x: src.get_i32(),
                y: src.get_i8(),
                z: src.get_i32(),
                block_id: VarInt::decode(src)?.0 as i32,
                block_metadata: src.get_i8(),
            })
        }
    }

    // ── Set Held Item (0x09) ─────────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundSetHeldItem {
        pub slot: u8,
    }

    impl PacketId for ClientboundSetHeldItem {
        fn packet_id(_ver: u32) -> u8 {
            0x09
        }
    }

    impl Encode for ClientboundSetHeldItem {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_u8(self.slot);
            Ok(())
        }
    }

    impl Decode for ClientboundSetHeldItem {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 1)?;
            Ok(Self { slot: src.get_u8() })
        }
    }

    // ── Named Sound Effect (0x29) ────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundSound {
        pub sound_name: String,
        pub x: i32,
        pub y: i32,
        pub z: i32,
        pub volume: f32,
        pub pitch: u8,
    }

    impl PacketId for ClientboundSound {
        fn packet_id(_ver: u32) -> u8 {
            0x29
        }
    }

    impl Encode for ClientboundSound {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            encode_str(&self.sound_name, dst)?;
            dst.put_i32(self.x);
            dst.put_i32(self.y);
            dst.put_i32(self.z);
            dst.put_f32(self.volume);
            dst.put_u8(self.pitch);
            Ok(())
        }
    }

    impl Decode for ClientboundSound {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let sound_name = decode_str(src, "ClientboundSound sound_name")?;
            need(src, 4 + 4 + 4 + 4 + 1)?;
            Ok(Self {
                sound_name,
                x: src.get_i32(),
                y: src.get_i32(),
                z: src.get_i32(),
                volume: src.get_f32(),
                pitch: src.get_u8(),
            })
        }
    }

    // ── Player Abilities (0x39) ───────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundPlayerAbilities {
        pub flags: u8,
        pub flying_speed: f32,
        pub field_of_view_modifier: f32,
    }

    impl PacketId for ClientboundPlayerAbilities {
        fn packet_id(_ver: u32) -> u8 {
            0x39
        }
    }

    impl Encode for ClientboundPlayerAbilities {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_u8(self.flags);
            dst.put_f32(self.flying_speed);
            dst.put_f32(self.field_of_view_modifier);
            Ok(())
        }
    }

    impl Decode for ClientboundPlayerAbilities {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 1 + 4 + 4)?;
            Ok(Self {
                flags: src.get_u8(),
                flying_speed: src.get_f32(),
                field_of_view_modifier: src.get_f32(),
            })
        }
    }

    // ── ContainerSetContent (0x30) ────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundContainerSetContent {
        pub window_id: u8,
        pub slots: Vec<LegacySlot>,
        pub carried_item: LegacySlot,
    }

    impl PacketId for ClientboundContainerSetContent {
        fn packet_id(_ver: u32) -> u8 {
            0x30
        }
    }

    impl Encode for ClientboundContainerSetContent {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_u8(self.window_id);
            dst.put_i16(self.slots.len() as i16);
            for slot in &self.slots {
                slot.encode(dst)?;
            }
            self.carried_item.encode(dst)?;
            Ok(())
        }
    }

    impl Decode for ClientboundContainerSetContent {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 1 + 2)?;
            let window_id = src.get_u8();
            let count = src.get_i16() as usize;
            let mut slots = Vec::with_capacity(count);
            for _ in 0..count {
                slots.push(LegacySlot::decode(src)?);
            }
            let carried_item = LegacySlot::decode(src)?;
            Ok(Self {
                window_id,
                slots,
                carried_item,
            })
        }
    }

    // ── ContainerSetSlot (0x2F) ───────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundContainerSetSlot {
        pub window_id: i8,
        pub slot: i16,
        pub slot_data: LegacySlot,
    }

    impl PacketId for ClientboundContainerSetSlot {
        fn packet_id(_ver: u32) -> u8 {
            0x2F
        }
    }

    impl Encode for ClientboundContainerSetSlot {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_i8(self.window_id);
            dst.put_i16(self.slot);
            self.slot_data.encode(dst)?;
            Ok(())
        }
    }

    impl Decode for ClientboundContainerSetSlot {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 1 + 2)?;
            let window_id = src.get_i8();
            let slot = src.get_i16();
            let slot_data = LegacySlot::decode(src)?;
            Ok(Self {
                window_id,
                slot,
                slot_data,
            })
        }
    }

    raw_packet!(ClientboundMapItemData, 0x34);
    raw_packet!(ClientboundBlockEntityData, 0x35);
    raw_packet!(ClientboundAwardStats, 0x37);
    raw_packet!(ClientboundCommandSuggestions, 0x3A);
    raw_packet!(ClientboundSetScoreboardObjective, 0x3B);
    raw_packet!(ClientboundResetScore, 0x3B);
    raw_packet!(ClientboundSetScoreboardScore, 0x3C);
    raw_packet!(ClientboundChangeDifficulty, 0x41);
    raw_packet!(ClientboundPlayerCombatEnd, 0x42);
    raw_packet!(ClientboundPlayerCombatEnter, 0x42);
    raw_packet!(ClientboundPlayerCombatKill, 0x42);

    // ── Open Sign Editor (0x36) ─────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundOpenSignEditor {
        pub x: i32,
        pub y: i32,
        pub z: i32,
    }

    impl PacketId for ClientboundOpenSignEditor {
        fn packet_id(_ver: u32) -> u8 {
            0x36
        }
    }

    impl Encode for ClientboundOpenSignEditor {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_i32(self.x);
            dst.put_i32(self.y);
            dst.put_i32(self.z);
            Ok(())
        }
    }

    impl Decode for ClientboundOpenSignEditor {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 4 + 4 + 4)?;
            Ok(Self {
                x: src.get_i32(),
                y: src.get_i32(),
                z: src.get_i32(),
            })
        }
    }

    // ── Player Info Update (0x38) ─────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundPlayerInfoUpdate {
        pub action: i8,
        pub data: Vec<u8>,
    }

    impl PacketId for ClientboundPlayerInfoUpdate {
        fn packet_id(_ver: u32) -> u8 {
            0x38
        }
    }

    impl Encode for ClientboundPlayerInfoUpdate {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_i8(self.action);
            dst.extend_from_slice(&self.data);
            Ok(())
        }
    }

    impl Decode for ClientboundPlayerInfoUpdate {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 1)?;
            let action = src.get_i8();
            let data = src.to_vec();
            Ok(Self { action, data })
        }
    }

    // ── Tab List (0x47) ─────────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundTabList {
        pub header: String,
        pub footer: String,
    }

    impl PacketId for ClientboundTabList {
        fn packet_id(_ver: u32) -> u8 {
            0x47
        }
    }

    impl Encode for ClientboundTabList {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            encode_str(&self.header, dst)?;
            encode_str(&self.footer, dst)?;
            Ok(())
        }
    }

    impl Decode for ClientboundTabList {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let header = decode_str(src, "ClientboundTabList header")?;
            let footer = decode_str(src, "ClientboundTabList footer")?;
            Ok(Self { header, footer })
        }
    }

    // ── Resource Pack Push (0x48) ───────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundResourcePackPush {
        pub url: String,
        pub hash: String,
    }

    impl PacketId for ClientboundResourcePackPush {
        fn packet_id(_ver: u32) -> u8 {
            0x48
        }
    }

    impl Encode for ClientboundResourcePackPush {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            encode_str(&self.url, dst)?;
            encode_str(&self.hash, dst)?;
            Ok(())
        }
    }

    impl Decode for ClientboundResourcePackPush {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let url = decode_str(src, "ClientboundResourcePackPush url")?;
            let hash = decode_str(src, "ClientboundResourcePackPush hash")?;
            Ok(Self { url, hash })
        }
    }

    raw_packet!(ServerboundMovePlayerStatusOnly, 0x03);
    raw_packet!(ServerboundPlayerAction, 0x07);
    raw_packet!(ServerboundPlayerBlockPlacement, 0x08);
    raw_packet!(ServerboundHeldItemChange, 0x09);
    raw_packet!(ServerboundAnimation, 0x0A);
    raw_packet!(ServerboundEntityAction, 0x0B);
    raw_packet!(ServerboundSteerVehicle, 0x0C);
    raw_packet!(ServerboundCloseWindow, 0x0D);
    raw_packet!(ServerboundClickWindow, 0x0E);
    raw_packet!(ServerboundConfirmTransaction, 0x0F);
    raw_packet!(ServerboundCreativeInventoryAction, 0x10);
    raw_packet!(ServerboundEnchantItem, 0x11);
    raw_packet!(ServerboundClickWindowButton, 0x11);
    raw_packet!(ServerboundUpdateSign, 0x12);
    raw_packet!(ServerboundPlayerAbilities, 0x13);
    raw_packet!(ServerboundCommandSuggestion, 0x14);
    raw_packet!(ServerboundClientSettings, 0x15);
    raw_packet!(ServerboundClientStatus, 0x16);
    raw_packet!(ServerboundResourcePackStatus, 0x19);
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! roundtrip {
        ($T:ty, $val:expr) => {{
            let v = $val;
            let mut buf = BytesMut::new();
            v.encode(&mut buf).unwrap();
            let mut b = buf.freeze();
            assert_eq!(<$T>::decode(&mut b).unwrap(), v);
        }};
    }

    #[test]
    fn keepalive_roundtrip() {
        roundtrip!(
            ClientboundKeepAlive,
            ClientboundKeepAlive {
                keep_alive_id: VarInt(42)
            }
        );
        roundtrip!(
            ServerboundKeepAlive,
            ServerboundKeepAlive {
                keep_alive_id: VarInt(42)
            }
        );
    }

    #[test]
    fn join_game_roundtrip() {
        roundtrip!(
            ClientboundJoinGame,
            ClientboundJoinGame {
                entity_id: 1,
                game_mode: 0,
                dimension: 0,
                difficulty: 1,
                max_players: 20,
                level_type: "default".to_string(),
                reduced_debug_info: false,
            }
        );
    }

    #[test]
    fn chat_roundtrip() {
        roundtrip!(
            ClientboundChatMessage,
            ClientboundChatMessage {
                json_message: r#"{"text":"hi"}"#.to_string(),
                position: 0,
            }
        );
        roundtrip!(
            ServerboundChatMessage,
            ServerboundChatMessage {
                message: "hello".to_string()
            }
        );
    }

    #[test]
    fn respawn_roundtrip() {
        roundtrip!(
            ClientboundRespawn,
            ClientboundRespawn {
                dimension: -1,
                difficulty: 2,
                game_mode: 0,
                level_type: "default".to_string(),
            }
        );
    }

    #[test]
    fn player_position_roundtrip() {
        roundtrip!(
            ClientboundPlayerPosition,
            ClientboundPlayerPosition {
                x: 10.0,
                head_y: 66.62,
                z: -5.0,
                yaw: 0.0,
                pitch: 0.0,
                flags: 0,
            }
        );
    }

    #[test]
    fn disconnect_roundtrip() {
        roundtrip!(
            ClientboundDisconnect,
            ClientboundDisconnect {
                reason: r#"{"text":"bye"}"#.to_string(),
            }
        );
    }

    #[test]
    fn movement_roundtrip() {
        roundtrip!(
            ServerboundMovePlayerPos,
            ServerboundMovePlayerPos {
                x: 5.0,
                feet_y: 64.0,
                z: -2.0,
                on_ground: true,
            }
        );
        roundtrip!(
            ServerboundMovePlayerRot,
            ServerboundMovePlayerRot {
                yaw: 90.0,
                pitch: -10.0,
                on_ground: true,
            }
        );
        roundtrip!(
            ServerboundMovePlayerPosRot,
            ServerboundMovePlayerPosRot {
                x: 1.0,
                feet_y: 65.0,
                z: 2.0,
                yaw: 180.0,
                pitch: 45.0,
                on_ground: false,
            }
        );
    }

    #[test]
    fn interact_roundtrip() {
        roundtrip!(
            ServerboundInteract,
            ServerboundInteract {
                entity_id: VarInt(1),
                action: InteractAction::Attack,
            }
        );
        roundtrip!(
            ServerboundInteract,
            ServerboundInteract {
                entity_id: VarInt(2),
                action: InteractAction::Interact,
            }
        );
        roundtrip!(
            ServerboundInteract,
            ServerboundInteract {
                entity_id: VarInt(3),
                action: InteractAction::InteractAt {
                    target_x: 0.5,
                    target_y: 1.0,
                    target_z: 0.5
                },
            }
        );
    }

    #[test]
    fn plugin_message_roundtrip() {
        roundtrip!(
            ClientboundPluginMessage,
            ClientboundPluginMessage {
                channel: "MC|Brand".to_string(),
                data: b"vanilla".to_vec(),
            }
        );
        roundtrip!(
            ServerboundPluginMessage,
            ServerboundPluginMessage {
                channel: "MC|Brand".to_string(),
                data: b"client".to_vec(),
            }
        );
    }

    #[test]
    fn packet_ids() {
        assert_eq!(ClientboundKeepAlive::packet_id(5), 0x00);
        assert_eq!(ClientboundJoinGame::packet_id(5), 0x01);
        assert_eq!(ClientboundChatMessage::packet_id(5), 0x02);
        assert_eq!(ClientboundRespawn::packet_id(5), 0x07);
        assert_eq!(ClientboundPlayerPosition::packet_id(5), 0x08);
        assert_eq!(ClientboundDisconnect::packet_id(5), 0x40);
        assert_eq!(ClientboundPluginMessage::packet_id(5), 0x3F);
        assert_eq!(ServerboundKeepAlive::packet_id(5), 0x00);
        assert_eq!(ServerboundChatMessage::packet_id(5), 0x01);
        assert_eq!(ServerboundInteract::packet_id(5), 0x02);
        assert_eq!(ServerboundMovePlayerPos::packet_id(5), 0x04);
        assert_eq!(ServerboundMovePlayerRot::packet_id(5), 0x05);
        assert_eq!(ServerboundMovePlayerPosRot::packet_id(5), 0x06);
        assert_eq!(ServerboundPluginMessage::packet_id(5), 0x17);
    }
}
