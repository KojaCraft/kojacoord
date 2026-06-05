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
    ClientboundExplosion, ClientboundForgetLevelChunk, ClientboundGameEvent,
    ClientboundInitializeBorder, ClientboundJoinGame, ClientboundKeepAlive,
    ClientboundLevelChunkWithLight, ClientboundLevelEvent, ClientboundLevelParticles,
    ClientboundLoginPlay, ClientboundMapItemData, ClientboundMoveEntityPos,
    ClientboundMoveEntityPosRot, ClientboundMoveEntityRot, ClientboundOpenScreen,
    ClientboundOpenSignEditor, ClientboundPlayerAbilities, ClientboundPlayerCombatEnd,
    ClientboundPlayerCombatEnter, ClientboundPlayerCombatKill, ClientboundPlayerInfoUpdate,
    ClientboundPlayerPosition, ClientboundPluginMessage, ClientboundRemoveEntities,
    ClientboundRemoveEntityEffect, ClientboundResetScore, ClientboundResourcePackPush,
    ClientboundRespawn, ClientboundRotateHead, ClientboundSectionBlocksUpdate,
    ClientboundSetBorderCenter, ClientboundSetBorderLerpSize, ClientboundSetBorderSize,
    ClientboundSetBorderWarningDelay, ClientboundSetBorderWarningDistance, ClientboundSetCamera,
    ClientboundSetEntityLink, ClientboundSetEntityMotion, ClientboundSetEquipment,
    ClientboundSetExperience, ClientboundSetHealth, ClientboundSetHeldItem,
    ClientboundSetScoreboardObjective, ClientboundSetScoreboardScore, ClientboundSetTime,
    ClientboundSound, ClientboundSpawnEntity, ClientboundSpawnExperienceOrb,
    ClientboundSpawnPlayer, ClientboundTabList, ClientboundTakeItemEntity,
    ClientboundTeleportEntity, ClientboundUpdateAttributes, ClientboundUpdateEffects,
    InteractAction, ServerboundAnimation, ServerboundChatMessage, ServerboundClickWindow,
    ServerboundClickWindowButton, ServerboundClientSettings, ServerboundClientStatus,
    ServerboundCloseWindow, ServerboundCommandSuggestion, ServerboundConfirmTransaction,
    ServerboundCreativeInventoryAction, ServerboundEnchantItem, ServerboundEntityAction,
    ServerboundHeldItemChange, ServerboundInteract, ServerboundKeepAlive, ServerboundMovePlayerPos,
    ServerboundMovePlayerPosRot, ServerboundMovePlayerRot, ServerboundMovePlayerStatusOnly,
    ServerboundPlayerAbilities, ServerboundPlayerAction, ServerboundPlayerBlockPlacement,
    ServerboundPluginMessage, ServerboundResourcePackStatus, ServerboundSpectate,
    ServerboundSteerVehicle, ServerboundUpdateSign,
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

    // LoginPlay is an alias for JoinGame used by the proxy layer — kept as raw.
    raw_packet!(ClientboundLoginPlay, 0x01);

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

    // ── Chat (0x02 / 0x01) ────────────────────────────────────────────────────

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

    // ── PluginMessage (0x3F / 0x17) ───────────────────────────────────────────

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
            dst.put_slice(&self.data);
            Ok(())
        }
    }

    impl Decode for ClientboundPluginMessage {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let channel = decode_str(src, "ClientboundPluginMessage channel")?;
            let len = src.remaining();
            let mut data = vec![0u8; len];
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
            dst.put_slice(&self.data);
            Ok(())
        }
    }

    impl Decode for ServerboundPluginMessage {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let channel = decode_str(src, "ServerboundPluginMessage channel")?;
            let len = src.remaining();
            let mut data = vec![0u8; len];
            src.copy_to_slice(&mut data);
            Ok(Self { channel, data })
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

    // ── SetHeldItem (0x09) ────────────────────────────────────────────────────

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

    // ── Interact (0x02) ───────────────────────────────────────────────────────

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
    pub struct ServerboundMovePlayerStatusOnly {
        pub on_ground: bool,
    }

    impl PacketId for ServerboundMovePlayerStatusOnly {
        fn packet_id(_ver: u32) -> u8 {
            0x03
        }
    }

    impl Encode for ServerboundMovePlayerStatusOnly {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_u8(self.on_ground as u8);
            Ok(())
        }
    }

    impl Decode for ServerboundMovePlayerStatusOnly {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 1)?;
            Ok(Self {
                on_ground: src.get_u8() != 0,
            })
        }
    }

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
            let sound_name = decode_str(src, "MinecraftNamedSound context")?;
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

    // ── Animation (0x0A) — zero-byte serverbound packet ───────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ServerboundAnimation;

    impl PacketId for ServerboundAnimation {
        fn packet_id(_ver: u32) -> u8 {
            0x0A
        }
    }

    impl Encode for ServerboundAnimation {
        fn encode(&self, _dst: &mut BytesMut) -> Result<(), ProtocolError> {
            Ok(())
        }
    }

    impl Decode for ServerboundAnimation {
        fn decode(_src: &mut Bytes) -> Result<Self, ProtocolError> {
            Ok(Self)
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

    // ── Opaque raw stubs ──────────────────────────────────────────────────────

    raw_packet!(ClientboundSpawnEntity, 0x0E);
    raw_packet!(ClientboundSpawnExperienceOrb, 0x11);
    raw_packet!(ClientboundSpawnPlayer, 0x0C);
    raw_packet!(ClientboundEntityAnimation, 0x0B);
    raw_packet!(ClientboundAwardStats, 0x37);
    raw_packet!(ClientboundBlockDestroyStage, 0x25);
    raw_packet!(ClientboundBlockEntityData, 0x35);
    raw_packet!(ClientboundBlockAction, 0x24);
    raw_packet!(ClientboundBlockUpdate, 0x23);
    raw_packet!(ClientboundChangeDifficulty, 0x41);
    raw_packet!(ClientboundCommandSuggestions, 0x3A);
    raw_packet!(ClientboundContainerClose, 0x2E);
    raw_packet!(ClientboundContainerSetProperty, 0x31);

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

    raw_packet!(ClientboundEntityEvent, 0x1A);
    raw_packet!(ClientboundExplosion, 0x27);
    raw_packet!(ClientboundForgetLevelChunk, 0x26);
    raw_packet!(ClientboundGameEvent, 0x2B);
    raw_packet!(ClientboundInitializeBorder, 0x44);
    raw_packet!(ClientboundLevelChunkWithLight, 0x21);
    raw_packet!(ClientboundLevelEvent, 0x28);
    raw_packet!(ClientboundLevelParticles, 0x2A);
    raw_packet!(ClientboundMapItemData, 0x34);
    raw_packet!(ClientboundMoveEntityPos, 0x15);
    raw_packet!(ClientboundMoveEntityPosRot, 0x17);
    raw_packet!(ClientboundMoveEntityRot, 0x16);
    raw_packet!(ClientboundOpenScreen, 0x2D);
    raw_packet!(ClientboundOpenSignEditor, 0x36);
    raw_packet!(ClientboundPlayerCombatEnd, 0x42);
    raw_packet!(ClientboundPlayerCombatEnter, 0x42);
    raw_packet!(ClientboundPlayerCombatKill, 0x42);
    raw_packet!(ClientboundPlayerInfoUpdate, 0x38);
    raw_packet!(ClientboundRemoveEntities, 0x13);
    raw_packet!(ClientboundRemoveEntityEffect, 0x1E);
    raw_packet!(ClientboundResetScore, 0x3B);
    raw_packet!(ClientboundResourcePackPush, 0x48);
    raw_packet!(ClientboundRotateHead, 0x19);
    raw_packet!(ClientboundSectionBlocksUpdate, 0x22);
    raw_packet!(ClientboundSetBorderCenter, 0x44);
    raw_packet!(ClientboundSetBorderLerpSize, 0x44);
    raw_packet!(ClientboundSetBorderSize, 0x44);
    raw_packet!(ClientboundSetBorderWarningDelay, 0x44);
    raw_packet!(ClientboundSetBorderWarningDistance, 0x44);
    raw_packet!(ClientboundSetCamera, 0x43);
    raw_packet!(ClientboundSetEntityLink, 0x1B);
    raw_packet!(ClientboundSetEntityMotion, 0x12);
    raw_packet!(ClientboundSetEquipment, 0x04);
    raw_packet!(ClientboundSetExperience, 0x1F);
    raw_packet!(ClientboundSetHealth, 0x06);
    raw_packet!(ClientboundSetScoreboardObjective, 0x3B);
    raw_packet!(ClientboundSetScoreboardScore, 0x3C);
    raw_packet!(ClientboundSetTime, 0x03);
    raw_packet!(ClientboundTabList, 0x47);
    raw_packet!(ClientboundTakeItemEntity, 0x0D);
    raw_packet!(ClientboundTeleportEntity, 0x18);
    raw_packet!(ClientboundUpdateAttributes, 0x20);
    raw_packet!(ClientboundUpdateEffects, 0x1D);

    raw_packet!(ServerboundPlayerBlockPlacement, 0x08);
    raw_packet!(ServerboundClientSettings, 0x15);
    raw_packet!(ServerboundClickWindowButton, 0x11);
    raw_packet!(ServerboundClickWindow, 0x0E);
    raw_packet!(ServerboundCloseWindow, 0x0D);
    raw_packet!(ServerboundCommandSuggestion, 0x14);
    raw_packet!(ServerboundConfirmTransaction, 0x0F);
    raw_packet!(ServerboundCreativeInventoryAction, 0x10);
    raw_packet!(ServerboundEnchantItem, 0x11);
    raw_packet!(ServerboundEntityAction, 0x0B);
    raw_packet!(ServerboundHeldItemChange, 0x09);
    raw_packet!(ServerboundPlayerAbilities, 0x13);
    raw_packet!(ServerboundPlayerAction, 0x07);
    raw_packet!(ServerboundResourcePackStatus, 0x19);
    raw_packet!(ServerboundClientStatus, 0x16);
    raw_packet!(ServerboundSpectate, 0x18);
    raw_packet!(ServerboundSteerVehicle, 0x0C);
    raw_packet!(ServerboundUpdateSign, 0x12);
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
                x: 100.0,
                head_y: 65.62,
                z: -50.0,
                yaw: 0.0,
                pitch: 0.0,
                flags: 0,
            }
        );
    }

    #[test]
    fn keepalive_roundtrip() {
        roundtrip!(
            ClientboundKeepAlive,
            ClientboundKeepAlive {
                keep_alive_id: VarInt(12345)
            }
        );
        roundtrip!(
            ServerboundKeepAlive,
            ServerboundKeepAlive {
                keep_alive_id: VarInt(12345)
            }
        );
    }

    #[test]
    fn chat_roundtrip() {
        roundtrip!(
            ClientboundChatMessage,
            ClientboundChatMessage {
                json_message: r#"{"text":"hello"}"#.to_string(),
                position: 0,
            }
        );
        roundtrip!(
            ServerboundChatMessage,
            ServerboundChatMessage {
                message: "/help".to_string()
            }
        );
    }

    #[test]
    fn move_pos_roundtrip() {
        roundtrip!(
            ServerboundMovePlayerPos,
            ServerboundMovePlayerPos {
                x: 1.0,
                feet_y: 64.0,
                z: -3.5,
                on_ground: true,
            }
        );
    }

    #[test]
    fn move_rot_roundtrip() {
        roundtrip!(
            ServerboundMovePlayerRot,
            ServerboundMovePlayerRot {
                yaw: 45.0,
                pitch: 0.0,
                on_ground: true,
            }
        );
    }

    #[test]
    fn move_pos_rot_roundtrip() {
        roundtrip!(
            ServerboundMovePlayerPosRot,
            ServerboundMovePlayerPosRot {
                x: 10.0,
                feet_y: 65.0,
                z: 20.0,
                yaw: 90.0,
                pitch: -30.0,
                on_ground: false,
            }
        );
    }

    #[test]
    fn move_status_only_roundtrip() {
        roundtrip!(
            ServerboundMovePlayerStatusOnly,
            ServerboundMovePlayerStatusOnly { on_ground: true }
        );
    }

    #[test]
    fn interact_roundtrip() {
        roundtrip!(
            ServerboundInteract,
            ServerboundInteract {
                entity_id: VarInt(42),
                action: InteractAction::Attack,
            }
        );
        roundtrip!(
            ServerboundInteract,
            ServerboundInteract {
                entity_id: VarInt(7),
                action: InteractAction::Interact,
            }
        );
        roundtrip!(
            ServerboundInteract,
            ServerboundInteract {
                entity_id: VarInt(5),
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
                data: b"vanilla".to_vec(),
            }
        );
    }

    #[test]
    fn disconnect_roundtrip() {
        roundtrip!(
            ClientboundDisconnect,
            ClientboundDisconnect {
                reason: r#"{"text":"You are banned"}"#.to_string(),
            }
        );
    }

    #[test]
    fn set_held_item_roundtrip() {
        roundtrip!(ClientboundSetHeldItem, ClientboundSetHeldItem { slot: 3 });
    }

    #[test]
    fn animation_roundtrip() {
        roundtrip!(ServerboundAnimation, ServerboundAnimation);
    }

    #[test]
    fn packet_ids() {
        assert_eq!(ClientboundJoinGame::packet_id(47), 0x01);
        assert_eq!(ClientboundLoginPlay::packet_id(47), 0x01);
        assert_eq!(ClientboundRespawn::packet_id(47), 0x07);
        assert_eq!(ClientboundPlayerPosition::packet_id(47), 0x08);
        assert_eq!(ClientboundKeepAlive::packet_id(47), 0x00);
        assert_eq!(ServerboundKeepAlive::packet_id(47), 0x00);
        assert_eq!(ClientboundChatMessage::packet_id(47), 0x02);
        assert_eq!(ServerboundChatMessage::packet_id(47), 0x01);
        assert_eq!(ServerboundMovePlayerPos::packet_id(47), 0x04);
        assert_eq!(ServerboundMovePlayerRot::packet_id(47), 0x05);
        assert_eq!(ServerboundMovePlayerPosRot::packet_id(47), 0x06);
        assert_eq!(ServerboundMovePlayerStatusOnly::packet_id(47), 0x03);
        assert_eq!(ServerboundInteract::packet_id(47), 0x02);
        assert_eq!(ClientboundPluginMessage::packet_id(47), 0x3F);
        assert_eq!(ServerboundPluginMessage::packet_id(47), 0x17);
        assert_eq!(ClientboundDisconnect::packet_id(47), 0x40);
        assert_eq!(ClientboundSetHeldItem::packet_id(47), 0x09);
        assert_eq!(ServerboundAnimation::packet_id(47), 0x0A);
        assert_eq!(ServerboundPlayerBlockPlacement::packet_id(47), 0x08);
        assert_eq!(ServerboundClientSettings::packet_id(47), 0x15);
    }
}
