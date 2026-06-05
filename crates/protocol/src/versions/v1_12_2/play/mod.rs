use crate::codec::{Decode, Encode, PacketId};
use crate::error::ProtocolError;
use crate::types::slot::LegacySlot;
use crate::types::VarInt;
use bytes::{Buf, BufMut, Bytes, BytesMut};

pub use packets::{
    ClientboundAwardStats, ClientboundBlockAction, ClientboundBlockDestroyStage,
    ClientboundBlockEntityData, ClientboundBlockUpdate, ClientboundBossBar,
    ClientboundChangeDifficulty, ClientboundChatMessage, ClientboundCommandSuggestions,
    ClientboundContainerClose, ClientboundContainerSetContent, ClientboundContainerSetProperty,
    ClientboundContainerSetSlot, ClientboundCooldown, ClientboundDisconnect,
    ClientboundEntityAnimation, ClientboundEntityEvent, ClientboundExplosion,
    ClientboundForgetLevelChunk, ClientboundGameEvent, ClientboundHorseScreenOpen,
    ClientboundInitializeBorder, ClientboundJoinGame, ClientboundKeepAlive,
    ClientboundLevelChunkWithLight, ClientboundLevelEvent, ClientboundLevelParticles,
    ClientboundMapItemData, ClientboundMoveEntityPos, ClientboundMoveEntityPosRot,
    ClientboundMoveEntityRot, ClientboundMoveVehicle, ClientboundOpenScreen,
    ClientboundOpenSignEditor, ClientboundPlaceGhostRecipe, ClientboundPlayerAbilities,
    ClientboundPlayerCombatEnd, ClientboundPlayerCombatEnter, ClientboundPlayerCombatKill,
    ClientboundPlayerInfoUpdate, ClientboundPlayerPosition, ClientboundPluginMessage,
    ClientboundRecipes, ClientboundRemoveEntities, ClientboundRemoveEntityEffect,
    ClientboundResetScore, ClientboundResourcePackPush, ClientboundRespawn, ClientboundRotateHead,
    ClientboundSectionBlocksUpdate, ClientboundSelectAdvancementsTab, ClientboundSetBorderCenter,
    ClientboundSetBorderLerpSize, ClientboundSetBorderSize, ClientboundSetBorderWarningDelay,
    ClientboundSetBorderWarningDistance, ClientboundSetCamera, ClientboundSetCarriedItem,
    ClientboundSetEntityLink, ClientboundSetEntityMotion, ClientboundSetEquipment,
    ClientboundSetExperience, ClientboundSetHealth, ClientboundSetHeldItem,
    ClientboundSetScoreboardObjective, ClientboundSetScoreboardScore, ClientboundSetTime,
    ClientboundSound, ClientboundSpawnEntity, ClientboundSpawnExperienceOrb,
    ClientboundSpawnGlobalEntity, ClientboundSpawnMob, ClientboundSpawnPainting,
    ClientboundSpawnPlayer, ClientboundStopSound, ClientboundTabList, ClientboundTakeItemEntity,
    ClientboundTeleportEntity, ClientboundUpdateAdvancements, ClientboundUpdateAttributes,
    ClientboundUpdateEffects, InteractAction, SPacketChunkData, ServerboundAnimation,
    ServerboundChatMessage, ServerboundClickWindow, ServerboundClickWindowButton,
    ServerboundClientSettings, ServerboundClientStatus, ServerboundCloseWindow,
    ServerboundCommandSuggestion, ServerboundConfirmTransaction,
    ServerboundCreativeInventoryAction, ServerboundCustomPayload, ServerboundEnchantItem,
    ServerboundEntityAction, ServerboundHeldItemChange, ServerboundInteract, ServerboundKeepAlive,
    ServerboundMovePlayerPos, ServerboundMovePlayerPosRot, ServerboundMovePlayerRot,
    ServerboundMovePlayerStatusOnly, ServerboundPickItem, ServerboundPlaceRecipe,
    ServerboundPlayerAbilities, ServerboundPlayerAction, ServerboundPlayerBlockPlacement,
    ServerboundPluginMessage, ServerboundRecipeBookChangeSettings, ServerboundRecipeBookSeenRecipe,
    ServerboundResourcePackStatus, ServerboundSelectTrade, ServerboundSetBeaconEffect,
    ServerboundSetStructureBlock, ServerboundSpectate, ServerboundSteerBoat,
    ServerboundSteerVehicle, ServerboundTeleportConfirm, ServerboundUpdateCommandBlock,
    ServerboundUpdateCommandBlockMinecart, ServerboundUpdateSign, ServerboundUseItem,
    ServerboundVehicleMove,
};

mod packets {
    use crate::types::Nbt;
    use uuid::Uuid;

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

    // ── JoinGame (0x23) ───────────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundJoinGame {
        pub entity_id: i32,
        pub gamemode: u8,
        pub dimension: i32, // 1.12.2 uses an i32 (-1: Nether, 0: Overworld, 1: End)
        pub difficulty: u8,
        pub max_players: u8,
        pub level_type: String, // e.g., "default", "flat"
        pub reduced_debug_info: bool,
    }

    impl PacketId for ClientboundJoinGame {
        fn packet_id(_ver: u32) -> u8 {
            0x23
        }
    }

    impl Encode for ClientboundJoinGame {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            // 1. Entity ID (4 bytes)
            dst.put_i32(self.entity_id);

            // 2. Gamemode (1 byte)
            dst.put_u8(self.gamemode);

            // 3. Dimension (4 bytes)
            dst.put_i32(self.dimension);

            // 4. Difficulty (1 byte)
            dst.put_u8(self.difficulty);

            // 5. Max Players (1 byte - ignored by modern clients but strictly read)
            dst.put_u8(self.max_players);

            // 6. Level Type (String -> VarInt Length + UTF-8 payload)
            let level_bytes = self.level_type.as_bytes();
            VarInt(level_bytes.len() as i32).encode(dst)?;
            dst.put_slice(level_bytes);

            // 7. Reduced Debug Info (1 byte boolean)
            dst.put_u8(self.reduced_debug_info as u8);

            Ok(())
        }
    }

    impl Decode for ClientboundJoinGame {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            // Base static size check: 4 (EID) + 1 (GM) + 4 (Dim) + 1 (Diff) + 1 (MaxP) = 11 bytes
            if src.remaining() < 11 {
                return Err(ProtocolError::Io(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "Missing initial static data for ClientboundJoinGame",
                )));
            }

            let entity_id = src.get_i32();
            let gamemode = src.get_u8();
            let dimension = src.get_i32();
            let difficulty = src.get_u8();
            let max_players = src.get_u8();

            // Decode Level Type String
            let level_len = VarInt::decode(src)?.0 as usize;
            if src.remaining() < level_len {
                return Err(ProtocolError::Io(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "Missing level_type string payload inside ClientboundJoinGame",
                )));
            }
            let mut level_bytes = vec![0u8; level_len];
            src.copy_to_slice(&mut level_bytes);
            let level_type = String::from_utf8(level_bytes).map_err(|_| {
                ProtocolError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Invalid UTF-8 sequence for level_type string",
                ))
            })?;

            // Decode trailing boolean
            if src.remaining() < 1 {
                return Err(ProtocolError::Io(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "Missing reduced_debug_info flag inside ClientboundJoinGame",
                )));
            }
            let reduced_debug_info = src.get_u8() != 0;

            Ok(Self {
                entity_id,
                gamemode,
                dimension,
                difficulty,
                max_players,
                level_type,
                reduced_debug_info,
            })
        }
    }

    // ── Respawn (0x35) ────────────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundRespawn {
        pub dimension: i32,
        pub difficulty: u8,
        pub game_mode: u8,
        pub level_type: String,
    }

    impl PacketId for ClientboundRespawn {
        fn packet_id(_ver: u32) -> u8 {
            0x35
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

    // ── PlayerPosition (0x2F) ─────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundPlayerPosition {
        pub x: f64,
        pub y: f64,
        pub z: f64,
        pub yaw: f32,
        pub pitch: f32,
        pub flags: u8,
        pub teleport_id: VarInt,
    }

    impl PacketId for ClientboundPlayerPosition {
        fn packet_id(_ver: u32) -> u8 {
            0x2F
        }
    }

    impl Encode for ClientboundPlayerPosition {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_f64(self.x);
            dst.put_f64(self.y);
            dst.put_f64(self.z);
            dst.put_f32(self.yaw);
            dst.put_f32(self.pitch);
            dst.put_u8(self.flags);
            self.teleport_id.encode(dst)
        }
    }

    impl Decode for ClientboundPlayerPosition {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 8 + 8 + 8 + 4 + 4 + 1)?;
            let x = src.get_f64();
            let y = src.get_f64();
            let z = src.get_f64();
            let yaw = src.get_f32();
            let pitch = src.get_f32();
            let flags = src.get_u8();
            let teleport_id = VarInt::decode(src)?;
            Ok(Self {
                x,
                y,
                z,
                yaw,
                pitch,
                flags,
                teleport_id,
            })
        }
    }

    // ── SpawnEntity (0x00) ────────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundSpawnEntity {
        pub entity_id: VarInt,
        pub object_uuid: Uuid,
        pub kind: u8,
        pub x: f64,
        pub y: f64,
        pub z: f64,
        pub yaw: u8,
        pub pitch: u8,
        pub data: i32,
        pub velocity_x: i16,
        pub velocity_y: i16,
        pub velocity_z: i16,
    }

    impl PacketId for ClientboundSpawnEntity {
        fn packet_id(_ver: u32) -> u8 {
            0x00
        }
    }

    impl Encode for ClientboundSpawnEntity {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.entity_id.encode(dst)?;
            dst.put_slice(self.object_uuid.as_bytes());
            dst.put_u8(self.kind);
            dst.put_f64(self.x);
            dst.put_f64(self.y);
            dst.put_f64(self.z);
            dst.put_u8(self.yaw);
            dst.put_u8(self.pitch);
            dst.put_i32(self.data);
            dst.put_i16(self.velocity_x);
            dst.put_i16(self.velocity_y);
            dst.put_i16(self.velocity_z);
            Ok(())
        }
    }

    impl Decode for ClientboundSpawnEntity {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let entity_id = VarInt::decode(src)?;
            // 16B UUID + 1B kind + 24B xyz + 2B yaw+pitch + 4B data + 6B velocity = 53
            need(src, 53)?;
            let mut uuid_bytes = [0u8; 16];
            src.copy_to_slice(&mut uuid_bytes);
            let object_uuid = Uuid::from_bytes(uuid_bytes);
            let kind = src.get_u8();
            let x = src.get_f64();
            let y = src.get_f64();
            let z = src.get_f64();
            let yaw = src.get_u8();
            let pitch = src.get_u8();
            let data = src.get_i32();
            let velocity_x = src.get_i16();
            let velocity_y = src.get_i16();
            let velocity_z = src.get_i16();
            Ok(Self {
                entity_id,
                object_uuid,
                kind,
                x,
                y,
                z,
                yaw,
                pitch,
                data,
                velocity_x,
                velocity_y,
                velocity_z,
            })
        }
    }

    // ── KeepAlive (0x1F / 0x0B) ──────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundKeepAlive {
        pub keep_alive_id: i64,
    }

    impl PacketId for ClientboundKeepAlive {
        fn packet_id(_ver: u32) -> u8 {
            0x1F
        }
    }

    impl Encode for ClientboundKeepAlive {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_i64(self.keep_alive_id);
            Ok(())
        }
    }

    impl Decode for ClientboundKeepAlive {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 8)?;
            Ok(Self {
                keep_alive_id: src.get_i64(),
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ServerboundKeepAlive {
        pub keep_alive_id: i64,
    }

    impl PacketId for ServerboundKeepAlive {
        fn packet_id(_ver: u32) -> u8 {
            0x0B
        }
    }

    impl Encode for ServerboundKeepAlive {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_i64(self.keep_alive_id);
            Ok(())
        }
    }

    impl Decode for ServerboundKeepAlive {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 8)?;
            Ok(Self {
                keep_alive_id: src.get_i64(),
            })
        }
    }

    // ── Chat (0x0F / 0x02) ────────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundChatMessage {
        pub json_message: String,
        pub position: u8,
    }

    impl PacketId for ClientboundChatMessage {
        fn packet_id(_ver: u32) -> u8 {
            0x0F
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
            let position = src.get_u8();
            Ok(Self {
                json_message,
                position,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ServerboundChatMessage {
        pub message: String,
    }

    impl PacketId for ServerboundChatMessage {
        fn packet_id(_ver: u32) -> u8 {
            0x02
        }
    }

    impl Encode for ServerboundChatMessage {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            encode_str(&self.message, dst)
        }
    }

    impl Decode for ServerboundChatMessage {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let message = decode_str(src, "ServerboundChatMessage message")?;
            Ok(Self { message })
        }
    }

    // ── Movement ──────────────────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ServerboundMovePlayerPos {
        pub x: f64,
        pub feet_y: f64,
        pub z: f64,
        pub on_ground: bool,
    }

    impl PacketId for ServerboundMovePlayerPos {
        fn packet_id(_ver: u32) -> u8 {
            0x0C
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
            0x0E
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
            0x0D
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

    // ── Interact (0x0A) ───────────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub enum InteractAction {
        Interact {
            hand: VarInt,
        },
        Attack,
        InteractAt {
            target_x: f32,
            target_y: f32,
            target_z: f32,
            hand: VarInt,
        },
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ServerboundInteract {
        pub entity_id: VarInt,
        pub action: InteractAction,
    }

    impl PacketId for ServerboundInteract {
        fn packet_id(_ver: u32) -> u8 {
            0x0A
        }
    }

    impl Encode for ServerboundInteract {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.entity_id.encode(dst)?;
            match &self.action {
                InteractAction::Interact { hand } => {
                    VarInt(0).encode(dst)?;
                    hand.encode(dst)?;
                },
                InteractAction::Attack => {
                    VarInt(1).encode(dst)?;
                },
                InteractAction::InteractAt {
                    target_x,
                    target_y,
                    target_z,
                    hand,
                } => {
                    VarInt(2).encode(dst)?;
                    dst.put_f32(*target_x);
                    dst.put_f32(*target_y);
                    dst.put_f32(*target_z);
                    hand.encode(dst)?;
                },
            }
            Ok(())
        }
    }

    impl Decode for ServerboundInteract {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let entity_id = VarInt::decode(src)?;
            let action = match VarInt::decode(src)?.0 {
                0 => InteractAction::Interact {
                    hand: VarInt::decode(src)?,
                },
                1 => InteractAction::Attack,
                2 => {
                    need(src, 4 + 4 + 4)?;
                    InteractAction::InteractAt {
                        target_x: src.get_f32(),
                        target_y: src.get_f32(),
                        target_z: src.get_f32(),
                        hand: VarInt::decode(src)?,
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

    // ── PluginMessage (0x18 / 0x09) ───────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundPluginMessage {
        pub channel: String,
        pub data: Vec<u8>,
    }

    impl PacketId for ClientboundPluginMessage {
        fn packet_id(_ver: u32) -> u8 {
            0x18
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
            0x09
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

    // ── Disconnect (0x1A) ─────────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundDisconnect {
        pub reason: String,
    }

    impl PacketId for ClientboundDisconnect {
        fn packet_id(_ver: u32) -> u8 {
            0x1A
        }
    }

    impl Encode for ClientboundDisconnect {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            encode_str(&self.reason, dst)
        }
    }

    impl Decode for ClientboundDisconnect {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let reason = decode_str(src, "ClientboundDisconnect reason")?;
            Ok(Self { reason })
        }
    }

    // ── PlayerAbilities (0x2C / 0x12) ─────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundPlayerAbilities {
        pub flags: u8,
        pub flying_speed: f32,
        pub walking_speed: f32,
    }

    impl PacketId for ClientboundPlayerAbilities {
        fn packet_id(_ver: u32) -> u8 {
            0x2C
        }
    }

    impl Encode for ClientboundPlayerAbilities {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_u8(self.flags);
            dst.put_f32(self.flying_speed);
            dst.put_f32(self.walking_speed);
            Ok(())
        }
    }

    impl Decode for ClientboundPlayerAbilities {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 1 + 4 + 4)?;
            Ok(Self {
                flags: src.get_u8(),
                flying_speed: src.get_f32(),
                walking_speed: src.get_f32(),
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ServerboundPlayerAbilities {
        pub flags: u8,
        pub flying_speed: f32,
        pub walking_speed: f32,
    }

    impl PacketId for ServerboundPlayerAbilities {
        fn packet_id(_ver: u32) -> u8 {
            0x12
        }
    }

    impl Encode for ServerboundPlayerAbilities {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_u8(self.flags);
            dst.put_f32(self.flying_speed);
            dst.put_f32(self.walking_speed);
            Ok(())
        }
    }

    impl Decode for ServerboundPlayerAbilities {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 1 + 4 + 4)?;
            Ok(Self {
                flags: src.get_u8(),
                flying_speed: src.get_f32(),
                walking_speed: src.get_f32(),
            })
        }
    }

    // ── SetCarriedItem (0x3A) ─────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundSetCarriedItem {
        pub slot: i8,
    }

    impl PacketId for ClientboundSetCarriedItem {
        fn packet_id(_ver: u32) -> u8 {
            0x3A
        }
    }

    impl Encode for ClientboundSetCarriedItem {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_i8(self.slot);
            Ok(())
        }
    }

    impl Decode for ClientboundSetCarriedItem {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 1)?;
            Ok(Self { slot: src.get_i8() })
        }
    }

    // ── BossBar (0x0C) ────────────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub enum BossBarAction {
        Add {
            title: String,
            health: f32,
            color: VarInt,
            division: VarInt,
            flags: u8,
        },
        Remove,
        UpdateHealth {
            health: f32,
        },
        UpdateTitle {
            title: String,
        },
        UpdateStyle {
            color: VarInt,
            division: VarInt,
        },
        UpdateFlags {
            flags: u8,
        },
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundBossBar {
        pub uuid: Uuid,
        pub action: BossBarAction,
    }

    impl PacketId for ClientboundBossBar {
        fn packet_id(_ver: u32) -> u8 {
            0x0C
        }
    }

    impl Encode for ClientboundBossBar {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_slice(self.uuid.as_bytes());
            let action_id: i32 = match &self.action {
                BossBarAction::Add { .. } => 0,
                BossBarAction::Remove => 1,
                BossBarAction::UpdateHealth { .. } => 2,
                BossBarAction::UpdateTitle { .. } => 3,
                BossBarAction::UpdateStyle { .. } => 4,
                BossBarAction::UpdateFlags { .. } => 5,
            };
            VarInt(action_id).encode(dst)?;
            match &self.action {
                BossBarAction::Add {
                    title,
                    health,
                    color,
                    division,
                    flags,
                } => {
                    encode_str(title, dst)?;
                    dst.put_f32(*health);
                    color.encode(dst)?;
                    division.encode(dst)?;
                    dst.put_u8(*flags);
                },
                BossBarAction::Remove => {},
                BossBarAction::UpdateHealth { health } => {
                    dst.put_f32(*health);
                },
                BossBarAction::UpdateTitle { title } => {
                    encode_str(title, dst)?;
                },
                BossBarAction::UpdateStyle { color, division } => {
                    color.encode(dst)?;
                    division.encode(dst)?;
                },
                BossBarAction::UpdateFlags { flags } => {
                    dst.put_u8(*flags);
                },
            }
            Ok(())
        }
    }

    impl Decode for ClientboundBossBar {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 16)?;
            let mut b = [0u8; 16];
            src.copy_to_slice(&mut b);
            let uuid = Uuid::from_bytes(b);
            let action = match VarInt::decode(src)?.0 {
                0 => {
                    let title = decode_str(src, "BossBar Add title")?;
                    need(src, 4)?;
                    let health = src.get_f32();
                    let color = VarInt::decode(src)?;
                    let division = VarInt::decode(src)?;
                    need(src, 1)?;
                    let flags = src.get_u8();
                    BossBarAction::Add {
                        title,
                        health,
                        color,
                        division,
                        flags,
                    }
                },
                1 => BossBarAction::Remove,
                2 => {
                    need(src, 4)?;
                    BossBarAction::UpdateHealth {
                        health: src.get_f32(),
                    }
                },
                3 => BossBarAction::UpdateTitle {
                    title: decode_str(src, "BossBar UpdateTitle")?,
                },
                4 => BossBarAction::UpdateStyle {
                    color: VarInt::decode(src)?,
                    division: VarInt::decode(src)?,
                },
                5 => {
                    need(src, 1)?;
                    BossBarAction::UpdateFlags {
                        flags: src.get_u8(),
                    }
                },
                _ => {
                    return Err(ProtocolError::Io(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "Unknown BossBar action id",
                    )))
                },
            };
            Ok(Self { uuid, action })
        }
    }

    // ── SpawnExperienceOrb (0x01) ─────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundSpawnExperienceOrb {
        pub entity_id: VarInt,
        pub x: f64,
        pub y: f64,
        pub z: f64,
        pub count: i16,
    }

    impl PacketId for ClientboundSpawnExperienceOrb {
        fn packet_id(_ver: u32) -> u8 {
            0x01
        }
    }

    impl Encode for ClientboundSpawnExperienceOrb {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.entity_id.encode(dst)?;
            dst.put_f64(self.x);
            dst.put_f64(self.y);
            dst.put_f64(self.z);
            dst.put_i16(self.count);
            Ok(())
        }
    }

    impl Decode for ClientboundSpawnExperienceOrb {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let entity_id = VarInt::decode(src)?;
            need(src, 8 + 8 + 8 + 2)?;
            Ok(Self {
                entity_id,
                x: src.get_f64(),
                y: src.get_f64(),
                z: src.get_f64(),
                count: src.get_i16(),
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundSpawnPainting {
        pub entity_id: VarInt,
        pub uuid: Uuid,
        pub title: String, // Název obrazu (např. "Alban", "Aztec") max 13 znaků
        pub position: u64, // Minecraft pozice x/y/z zabalená do u64
        pub direction: u8, // 0: jih, 1: západ, 2: sever, 3: východ
    }

    impl PacketId for ClientboundSpawnPainting {
        fn packet_id(_ver: u32) -> u8 {
            0x04
        }
    }

    impl Encode for ClientboundSpawnPainting {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.entity_id.encode(dst)?;
            dst.put_slice(self.uuid.as_bytes());

            let title_bytes = self.title.as_bytes();
            VarInt(title_bytes.len() as i32).encode(dst)?;
            dst.put_slice(title_bytes);

            dst.put_u64(self.position); // Pozice (Big Endian)
            dst.put_u8(self.direction);
            Ok(())
        }
    }

    impl Decode for ClientboundSpawnPainting {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let entity_id = VarInt::decode(src)?;

            if src.remaining() < 16 {
                return Err(ProtocolError::UnexpectedEof);
            }
            let mut uuid_bytes = [0u8; 16];
            src.copy_to_slice(&mut uuid_bytes);
            let uuid = Uuid::from_bytes(uuid_bytes);

            let title_len = VarInt::decode(src)?.0 as usize;
            if src.remaining() < title_len {
                return Err(ProtocolError::UnexpectedEof);
            }
            let mut title_bytes = vec![0u8; title_len];
            src.copy_to_slice(&mut title_bytes);
            let title = String::from_utf8(title_bytes).map_err(|_| ProtocolError::UnexpectedEof)?;

            if src.remaining() < 9 {
                return Err(ProtocolError::UnexpectedEof);
            } // 8B pozice + 1B směr
            let position = src.get_u64();
            let direction = src.get_u8();

            Ok(Self {
                entity_id,
                uuid,
                title,
                position,
                direction,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundSpawnMob {
        pub entity_id: VarInt,
        pub uuid: Uuid,
        pub kind: VarInt, // ID typu moba (v 1.12.2 vyjádřeno jako VarInt)
        pub x: f64,
        pub y: f64,
        pub z: f64,
        pub yaw: u8,
        pub pitch: u8,
        pub head_pitch: u8,
        pub velocity_x: i16,
        pub velocity_y: i16,
        pub velocity_z: i16,
    }

    impl PacketId for ClientboundSpawnMob {
        fn packet_id(_ver: u32) -> u8 {
            0x03
        }
    }

    impl Encode for ClientboundSpawnMob {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.entity_id.encode(dst)?;
            dst.put_slice(self.uuid.as_bytes());
            self.kind.encode(dst)?;
            dst.put_f64(self.x);
            dst.put_f64(self.y);
            dst.put_f64(self.z);
            dst.put_u8(self.yaw);
            dst.put_u8(self.pitch);
            dst.put_u8(self.head_pitch);
            dst.put_i16(self.velocity_x);
            dst.put_i16(self.velocity_y);
            dst.put_i16(self.velocity_z);

            // Konec paketu v 1.12.2 tvoří pole metadat (Entity Metadata) ukončené bajtem 0xFF.
            // Zapisujeme prázdný DataWatcher terminátor, abychom nekorumpovali stream.
            dst.put_u8(0xFF);
            Ok(())
        }
    }

    impl Decode for ClientboundSpawnMob {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let entity_id = VarInt::decode(src)?;

            // Pevná velikost bloku dat: 16B (UUID) + dynamický VarInt pro typ (provádíme check po přečtení)
            if src.remaining() < 16 {
                return Err(ProtocolError::UnexpectedEof);
            }
            let mut uuid_bytes = [0u8; 16];
            src.copy_to_slice(&mut uuid_bytes);
            let uuid = Uuid::from_bytes(uuid_bytes);

            let kind = VarInt::decode(src)?;

            // Zbytek fixních polí: 3x f64 (24B) + 3x u8 (3B) + 3x i16 (6B) = 33 bajtů
            if src.remaining() < 33 {
                return Err(ProtocolError::UnexpectedEof);
            }
            let x = src.get_f64();
            let y = src.get_f64();
            let z = src.get_f64();
            let yaw = src.get_u8();
            let pitch = src.get_u8();
            let head_pitch = src.get_u8();
            let velocity_x = src.get_i16();
            let velocity_y = src.get_i16();
            let velocity_z = src.get_i16();

            // Odr things: Zbývající bajty jsou DataWatcher metadata, která končí hodnotou 0xFF.
            // Proxy je bezpečně přeskočí, aby vyčistila buffer pro Netty.
            let remaining = src.remaining();
            src.advance(remaining);

            Ok(Self {
                entity_id,
                uuid,
                kind,
                x,
                y,
                z,
                yaw,
                pitch,
                head_pitch,
                velocity_x,
                velocity_y,
                velocity_z,
            })
        }
    }

    // ── SpawnPlayer (0x05) ────────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundSpawnPlayer {
        pub entity_id: VarInt,
        pub player_uuid: Uuid,
        pub x: f64,
        pub y: f64,
        pub z: f64,
        pub yaw: u8,
        pub pitch: u8,
    }

    impl PacketId for ClientboundSpawnPlayer {
        fn packet_id(_ver: u32) -> u8 {
            0x05
        }
    }

    impl Encode for ClientboundSpawnPlayer {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.entity_id.encode(dst)?;
            dst.put_slice(self.player_uuid.as_bytes());
            dst.put_f64(self.x);
            dst.put_f64(self.y);
            dst.put_f64(self.z);
            dst.put_u8(self.yaw);
            dst.put_u8(self.pitch);
            Ok(())
        }
    }

    impl Decode for ClientboundSpawnPlayer {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let entity_id = VarInt::decode(src)?;
            // 16B UUID + 24B xyz + 2B yaw+pitch = 42
            need(src, 42)?;
            let mut b = [0u8; 16];
            src.copy_to_slice(&mut b);
            let player_uuid = Uuid::from_bytes(b);
            Ok(Self {
                entity_id,
                player_uuid,
                x: src.get_f64(),
                y: src.get_f64(),
                z: src.get_f64(),
                yaw: src.get_u8(),
                pitch: src.get_u8(),
            })
        }
    }

    // ── EntityAnimation (0x06) ────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundEntityAnimation {
        pub entity_id: VarInt,
        pub animation: u8,
    }

    impl PacketId for ClientboundEntityAnimation {
        fn packet_id(_ver: u32) -> u8 {
            0x06
        }
    }

    impl Encode for ClientboundEntityAnimation {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.entity_id.encode(dst)?;
            dst.put_u8(self.animation);
            Ok(())
        }
    }

    impl Decode for ClientboundEntityAnimation {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let entity_id = VarInt::decode(src)?;
            need(src, 1)?;
            Ok(Self {
                entity_id,
                animation: src.get_u8(),
            })
        }
    }

    // ── AwardStats (0x07) ─────────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundAwardStats {
        pub entries: Vec<(String, VarInt)>,
    }

    impl PacketId for ClientboundAwardStats {
        fn packet_id(_ver: u32) -> u8 {
            0x07
        }
    }

    impl Encode for ClientboundAwardStats {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            VarInt(self.entries.len() as i32).encode(dst)?;
            for (name, value) in &self.entries {
                encode_str(name, dst)?;
                value.encode(dst)?;
            }
            Ok(())
        }
    }

    impl Decode for ClientboundAwardStats {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let count = VarInt::decode(src)?.0 as usize;
            let mut entries = Vec::with_capacity(count);
            for _ in 0..count {
                let name = decode_str(src, "ClientboundAwardStats entry name")?;
                let value = VarInt::decode(src)?;
                entries.push((name, value));
            }
            Ok(Self { entries })
        }
    }

    // ── BlockDestroyStage (0x08) ──────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundBlockDestroyStage {
        pub entity_id: VarInt,
        pub location: u64,
        pub destroy_stage: u8,
    }

    impl PacketId for ClientboundBlockDestroyStage {
        fn packet_id(_ver: u32) -> u8 {
            0x08
        }
    }

    impl Encode for ClientboundBlockDestroyStage {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.entity_id.encode(dst)?;
            dst.put_u64(self.location);
            dst.put_u8(self.destroy_stage);
            Ok(())
        }
    }

    impl Decode for ClientboundBlockDestroyStage {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let entity_id = VarInt::decode(src)?;
            need(src, 8 + 1)?;
            Ok(Self {
                entity_id,
                location: src.get_u64(),
                destroy_stage: src.get_u8(),
            })
        }
    }

    // ── BlockEntityData (0x09) ────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundBlockEntityData {
        pub location: u64,
        pub action: u8,
        pub nbt: Vec<u8>,
    }

    impl PacketId for ClientboundBlockEntityData {
        fn packet_id(_ver: u32) -> u8 {
            0x09
        }
    }

    impl Encode for ClientboundBlockEntityData {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_u64(self.location);
            dst.put_u8(self.action);
            dst.put_slice(&self.nbt);
            Ok(())
        }
    }

    impl Decode for ClientboundBlockEntityData {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 8 + 1)?;
            let location = src.get_u64();
            let action = src.get_u8();
            let len = src.remaining();
            let mut nbt = vec![0u8; len];
            src.copy_to_slice(&mut nbt);
            Ok(Self {
                location,
                action,
                nbt,
            })
        }
    }

    // ── BlockAction (0x0A) ────────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundBlockAction {
        pub location: u64,
        pub action_id: u8,
        pub action_param: u8,
        pub block_type: VarInt,
    }

    impl PacketId for ClientboundBlockAction {
        fn packet_id(_ver: u32) -> u8 {
            0x0A
        }
    }

    impl Encode for ClientboundBlockAction {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_u64(self.location);
            dst.put_u8(self.action_id);
            dst.put_u8(self.action_param);
            self.block_type.encode(dst)
        }
    }

    impl Decode for ClientboundBlockAction {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 8 + 1 + 1)?;
            let location = src.get_u64();
            let action_id = src.get_u8();
            let action_param = src.get_u8();
            let block_type = VarInt::decode(src)?;
            Ok(Self {
                location,
                action_id,
                action_param,
                block_type,
            })
        }
    }

    // ── BlockUpdate (0x0B) ────────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundBlockUpdate {
        pub location: u64,
        pub block_id: VarInt,
    }

    impl PacketId for ClientboundBlockUpdate {
        fn packet_id(_ver: u32) -> u8 {
            0x0B
        }
    }

    impl Encode for ClientboundBlockUpdate {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_u64(self.location);
            self.block_id.encode(dst)
        }
    }

    impl Decode for ClientboundBlockUpdate {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 8)?;
            let location = src.get_u64();
            let block_id = VarInt::decode(src)?;
            Ok(Self { location, block_id })
        }
    }

    // ── ChangeDifficulty (0x0D) ───────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundChangeDifficulty {
        pub difficulty: u8,
    }

    impl PacketId for ClientboundChangeDifficulty {
        fn packet_id(_ver: u32) -> u8 {
            0x0D
        }
    }

    impl Encode for ClientboundChangeDifficulty {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_u8(self.difficulty);
            Ok(())
        }
    }

    impl Decode for ClientboundChangeDifficulty {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 1)?;
            Ok(Self {
                difficulty: src.get_u8(),
            })
        }
    }

    // ── CommandSuggestions (0x0E) ─────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundCommandSuggestions {
        pub matches: Vec<String>,
    }

    impl PacketId for ClientboundCommandSuggestions {
        fn packet_id(_ver: u32) -> u8 {
            0x0E
        }
    }

    impl Encode for ClientboundCommandSuggestions {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            VarInt(self.matches.len() as i32).encode(dst)?;
            for m in &self.matches {
                encode_str(m, dst)?;
            }
            Ok(())
        }
    }

    impl Decode for ClientboundCommandSuggestions {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let count = VarInt::decode(src)?.0 as usize;
            let mut matches = Vec::with_capacity(count);
            for _ in 0..count {
                matches.push(decode_str(src, "ClientboundCommandSuggestions match")?);
            }
            Ok(Self { matches })
        }
    }

    // ── SectionBlocksUpdate (0x10) ────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundSectionBlocksUpdate {
        pub chunk_x: i32,
        pub chunk_z: i32,
        pub records: Vec<(u8, u8, VarInt)>,
    }

    impl PacketId for ClientboundSectionBlocksUpdate {
        fn packet_id(_ver: u32) -> u8 {
            0x10
        }
    }

    impl Encode for ClientboundSectionBlocksUpdate {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_i32(self.chunk_x);
            dst.put_i32(self.chunk_z);
            VarInt(self.records.len() as i32).encode(dst)?;
            for (h, y, id) in &self.records {
                dst.put_u8(*h);
                dst.put_u8(*y);
                id.encode(dst)?;
            }
            Ok(())
        }
    }

    impl Decode for ClientboundSectionBlocksUpdate {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 4 + 4)?;
            let chunk_x = src.get_i32();
            let chunk_z = src.get_i32();
            let count = VarInt::decode(src)?.0 as usize;
            let mut records = Vec::with_capacity(count);
            for _ in 0..count {
                need(src, 2)?;
                let h = src.get_u8();
                let y = src.get_u8();
                let id = VarInt::decode(src)?;
                records.push((h, y, id));
            }
            Ok(Self {
                chunk_x,
                chunk_z,
                records,
            })
        }
    }

    // ── ContainerClose (0x12) ─────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundContainerClose {
        pub window_id: u8,
    }

    impl PacketId for ClientboundContainerClose {
        fn packet_id(_ver: u32) -> u8 {
            0x12
        }
    }

    impl Encode for ClientboundContainerClose {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_u8(self.window_id);
            Ok(())
        }
    }

    impl Decode for ClientboundContainerClose {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 1)?;
            Ok(Self {
                window_id: src.get_u8(),
            })
        }
    }

    // ── OpenScreen (0x13) ─────────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundOpenScreen {
        pub window_id: u8,
        pub window_type: String,
        pub window_title: String,
        pub slot_count: u8,
    }

    impl PacketId for ClientboundOpenScreen {
        fn packet_id(_ver: u32) -> u8 {
            0x13
        }
    }

    impl Encode for ClientboundOpenScreen {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_u8(self.window_id);
            encode_str(&self.window_type, dst)?;
            encode_str(&self.window_title, dst)?;
            dst.put_u8(self.slot_count);
            Ok(())
        }
    }

    impl Decode for ClientboundOpenScreen {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 1)?;
            let window_id = src.get_u8();
            let window_type = decode_str(src, "ClientboundOpenScreen window_type")?;
            let window_title = decode_str(src, "ClientboundOpenScreen window_title")?;
            need(src, 1)?;
            let slot_count = src.get_u8();
            Ok(Self {
                window_id,
                window_type,
                window_title,
                slot_count,
            })
        }
    }

    // ── ContainerSetContent (0x14) ────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundContainerSetContent {
        pub window_id: u8,
        pub slots: Vec<LegacySlot>,
        pub carried_item: LegacySlot,
    }

    impl PacketId for ClientboundContainerSetContent {
        fn packet_id(_ver: u32) -> u8 {
            0x14
        }
    }

    impl Encode for ClientboundContainerSetContent {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_u8(self.window_id);
            VarInt(self.slots.len() as i32).encode(dst)?;
            for slot in &self.slots {
                slot.encode(dst)?;
            }
            self.carried_item.encode(dst)?;
            Ok(())
        }
    }

    impl Decode for ClientboundContainerSetContent {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 1)?;
            let window_id = src.get_u8();
            let count = VarInt::decode(src)?.0 as usize;
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

    // ── ContainerSetProperty (0x15) ───────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundContainerSetProperty {
        pub window_id: u8,
        pub property: i16,
        pub value: i16,
    }

    impl PacketId for ClientboundContainerSetProperty {
        fn packet_id(_ver: u32) -> u8 {
            0x15
        }
    }

    impl Encode for ClientboundContainerSetProperty {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_u8(self.window_id);
            dst.put_i16(self.property);
            dst.put_i16(self.value);
            Ok(())
        }
    }

    impl Decode for ClientboundContainerSetProperty {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 1 + 2 + 2)?;
            Ok(Self {
                window_id: src.get_u8(),
                property: src.get_i16(),
                value: src.get_i16(),
            })
        }
    }

    // ── ContainerSetSlot (0x16) ───────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundContainerSetSlot {
        pub window_id: i8,
        pub slot: i16,
        pub slot_data: LegacySlot,
    }

    impl PacketId for ClientboundContainerSetSlot {
        fn packet_id(_ver: u32) -> u8 {
            0x16
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

    // ── Cooldown (0x17) ───────────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundCooldown {
        pub item_id: VarInt,
        pub cooldown_ticks: VarInt,
    }

    impl PacketId for ClientboundCooldown {
        fn packet_id(_ver: u32) -> u8 {
            0x17
        }
    }

    impl Encode for ClientboundCooldown {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.item_id.encode(dst)?;
            self.cooldown_ticks.encode(dst)
        }
    }

    impl Decode for ClientboundCooldown {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            Ok(Self {
                item_id: VarInt::decode(src)?,
                cooldown_ticks: VarInt::decode(src)?,
            })
        }
    }

    // ── EntityEvent (0x1B) ────────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundEntityEvent {
        pub entity_id: i32,
        pub entity_status: u8,
    }

    impl PacketId for ClientboundEntityEvent {
        fn packet_id(_ver: u32) -> u8 {
            0x1B
        }
    }

    impl Encode for ClientboundEntityEvent {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_i32(self.entity_id);
            dst.put_u8(self.entity_status);
            Ok(())
        }
    }

    impl Decode for ClientboundEntityEvent {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 4 + 1)?;
            Ok(Self {
                entity_id: src.get_i32(),
                entity_status: src.get_u8(),
            })
        }
    }

    // ── Explosion (0x1C) ──────────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundExplosion {
        pub x: f32,
        pub y: f32,
        pub z: f32,
        pub radius: f32,
        pub records: Vec<(i8, i8, i8)>,
        pub player_motion_x: f32,
        pub player_motion_y: f32,
        pub player_motion_z: f32,
    }

    impl PacketId for ClientboundExplosion {
        fn packet_id(_ver: u32) -> u8 {
            0x1C
        }
    }

    impl Encode for ClientboundExplosion {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_f32(self.x);
            dst.put_f32(self.y);
            dst.put_f32(self.z);
            dst.put_f32(self.radius);
            dst.put_i32(self.records.len() as i32);
            for (rx, ry, rz) in &self.records {
                dst.put_i8(*rx);
                dst.put_i8(*ry);
                dst.put_i8(*rz);
            }
            dst.put_f32(self.player_motion_x);
            dst.put_f32(self.player_motion_y);
            dst.put_f32(self.player_motion_z);
            Ok(())
        }
    }

    impl Decode for ClientboundExplosion {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 4 + 4 + 4 + 4 + 4)?;
            let x = src.get_f32();
            let y = src.get_f32();
            let z = src.get_f32();
            let radius = src.get_f32();
            let count = src.get_i32() as usize;
            need(src, count * 3)?;
            let mut records = Vec::with_capacity(count);
            for _ in 0..count {
                records.push((src.get_i8(), src.get_i8(), src.get_i8()));
            }
            need(src, 4 + 4 + 4)?;
            Ok(Self {
                x,
                y,
                z,
                radius,
                records,
                player_motion_x: src.get_f32(),
                player_motion_y: src.get_f32(),
                player_motion_z: src.get_f32(),
            })
        }
    }

    // ── ForgetLevelChunk (0x1D) ───────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundForgetLevelChunk {
        pub chunk_x: i32,
        pub chunk_z: i32,
    }

    impl PacketId for ClientboundForgetLevelChunk {
        fn packet_id(_ver: u32) -> u8 {
            0x1D
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

    // ── GameEvent (0x1E) ──────────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundGameEvent {
        pub reason: u8,
        pub value: f32,
    }

    impl PacketId for ClientboundGameEvent {
        fn packet_id(_ver: u32) -> u8 {
            0x1E
        }
    }

    impl Encode for ClientboundGameEvent {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_u8(self.reason);
            dst.put_f32(self.value);
            Ok(())
        }
    }

    impl Decode for ClientboundGameEvent {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 1 + 4)?;
            Ok(Self {
                reason: src.get_u8(),
                value: src.get_f32(),
            })
        }
    }

    // ── LevelChunkWithLight (0x20) - SPacketChunkData for 1.12.2 ──────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundLevelChunkWithLight {
        pub chunk_x: i32,
        pub chunk_z: i32,
        pub full_chunk: bool,
        pub primary_bit_mask: VarInt,
        pub data: Vec<u8>,
        pub tile_entities: Vec<Nbt>, // NBT compound tags for tile entities
    }

    // Type alias for 1.12.2 compatibility
    pub type SPacketChunkData = ClientboundLevelChunkWithLight;

    impl PacketId for ClientboundLevelChunkWithLight {
        fn packet_id(_ver: u32) -> u8 {
            0x20
        }
    }

    impl Encode for ClientboundLevelChunkWithLight {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_i32(self.chunk_x);
            dst.put_i32(self.chunk_z);
            dst.put_u8(self.full_chunk as u8);
            self.primary_bit_mask.encode(dst)?;
            VarInt(self.data.len() as i32).encode(dst)?;
            dst.put_slice(&self.data);
            // Tile entity count (varint) followed by NBT compound tags
            VarInt(self.tile_entities.len() as i32).encode(dst)?;
            for tile_entity in &self.tile_entities {
                tile_entity.encode(dst)?;
            }
            Ok(())
        }
    }

    impl Decode for ClientboundLevelChunkWithLight {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 4 + 4 + 1)?;
            let chunk_x = src.get_i32();
            let chunk_z = src.get_i32();
            let full_chunk = src.get_u8() != 0;
            let primary_bit_mask = VarInt::decode(src)?;
            let len = VarInt::decode(src)?.0 as usize;
            need(src, len)?;
            let mut data = vec![0u8; len];
            src.copy_to_slice(&mut data);
            // Read tile entity count (varint) followed by NBT compound tags
            let tile_count = VarInt::decode(src)?.0 as usize;
            let mut tile_entities = Vec::with_capacity(tile_count);
            for _ in 0..tile_count {
                tile_entities.push(Nbt::decode(src)?);
            }
            Ok(Self {
                chunk_x,
                chunk_z,
                full_chunk,
                primary_bit_mask,
                data,
                tile_entities,
            })
        }
    }

    // ── LevelEvent (0x21) ─────────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundLevelEvent {
        pub effect_id: i32,
        pub location: u64,
        pub data: i32,
        pub disable_relative_volume: bool,
    }

    impl PacketId for ClientboundLevelEvent {
        fn packet_id(_ver: u32) -> u8 {
            0x21
        }
    }

    impl Encode for ClientboundLevelEvent {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_i32(self.effect_id);
            dst.put_u64(self.location);
            dst.put_i32(self.data);
            dst.put_u8(self.disable_relative_volume as u8);
            Ok(())
        }
    }

    impl Decode for ClientboundLevelEvent {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 4 + 8 + 4 + 1)?;
            Ok(Self {
                effect_id: src.get_i32(),
                location: src.get_u64(),
                data: src.get_i32(),
                disable_relative_volume: src.get_u8() != 0,
            })
        }
    }

    // ── LevelParticles (0x22) ─────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundLevelParticles {
        pub particle_id: i32,
        pub long_distance: bool,
        pub x: f32,
        pub y: f32,
        pub z: f32,
        pub offset_x: f32,
        pub offset_y: f32,
        pub offset_z: f32,
        pub particle_data: f32,
        pub particle_count: i32,
        pub data: Vec<VarInt>,
    }

    impl PacketId for ClientboundLevelParticles {
        fn packet_id(_ver: u32) -> u8 {
            0x22
        }
    }

    impl Encode for ClientboundLevelParticles {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            // Write the base static payload
            dst.put_i32(self.particle_id);
            dst.put_u8(self.long_distance as u8);
            dst.put_f32(self.x);
            dst.put_f32(self.y);
            dst.put_f32(self.z);
            dst.put_f32(self.offset_x);
            dst.put_f32(self.offset_y);
            dst.put_f32(self.offset_z);
            dst.put_f32(self.particle_data);
            dst.put_i32(self.particle_count);

            // ENFORCE EXACT SPECIFICATION LENGTHS FOR 1.12.2 WIRE TRAFFIC
            match self.particle_id {
                36 => {
                    // iconcrack requires EXACTLY 2 VarInts (Item ID, Metadata)
                    for d in self.data.iter().take(2) {
                        d.encode(dst)?;
                    }
                },
                37 | 38 => {
                    // blockcrack / blockdust requires EXACTLY 1 VarInt (Combined Block State)
                    if let Some(d) = self.data.first() {
                        d.encode(dst)?;
                    }
                },
                _ => {
                    // ANY OTHER PARTICLE MUST NOT WRITE TRAILING DATA
                    // If self.data contains residual values from a modern backend server, ignore them!
                },
            }
            Ok(())
        }
    }

    impl Decode for ClientboundLevelParticles {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            // 4 + 1 + 3*4 + 3*4 + 4 + 4 = 37
            need(src, 37)?;
            let particle_id = src.get_i32();
            let long_distance = src.get_u8() != 0;
            let x = src.get_f32();
            let y = src.get_f32();
            let z = src.get_f32();
            let offset_x = src.get_f32();
            let offset_y = src.get_f32();
            let offset_z = src.get_f32();
            let particle_data = src.get_f32();
            let particle_count = src.get_i32();
            let expected = match particle_id {
                36 => 2,
                37 | 38 => 1,
                _ => 0,
            };
            let mut data = Vec::with_capacity(expected);
            for _ in 0..expected {
                data.push(VarInt::decode(src)?);
            }
            Ok(Self {
                particle_id,
                long_distance,
                x,
                y,
                z,
                offset_x,
                offset_y,
                offset_z,
                particle_data,
                particle_count,
                data,
            })
        }
    }

    // ── MapItemData (0x24) ────────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundMapItemData {
        pub map_id: VarInt,
        pub scale: u8,
        pub tracking_position: bool,
        pub icons: Vec<(u8, i8, i8)>,
        pub columns: u8,
        pub data: Vec<u8>,
    }

    impl PacketId for ClientboundMapItemData {
        fn packet_id(_ver: u32) -> u8 {
            0x24
        }
    }

    impl Encode for ClientboundMapItemData {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.map_id.encode(dst)?;
            dst.put_u8(self.scale);
            dst.put_u8(self.tracking_position as u8);
            VarInt(self.icons.len() as i32).encode(dst)?;
            for (dir_type, x, z) in &self.icons {
                dst.put_u8(*dir_type);
                dst.put_i8(*x);
                dst.put_i8(*z);
            }
            dst.put_u8(self.columns);
            if self.columns > 0 {
                dst.put_u8(self.data.len() as u8); // rows
                dst.put_i8(0);
                dst.put_i8(0); // x, z offset
                VarInt(self.data.len() as i32).encode(dst)?;
                dst.put_slice(&self.data);
            }
            Ok(())
        }
    }

    impl Decode for ClientboundMapItemData {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let map_id = VarInt::decode(src)?;
            need(src, 1 + 1)?;
            let scale = src.get_u8();
            let tracking_position = src.get_u8() != 0;
            let icon_count = VarInt::decode(src)?.0 as usize;
            let mut icons = Vec::with_capacity(icon_count);
            for _ in 0..icon_count {
                need(src, 3)?;
                icons.push((src.get_u8(), src.get_i8(), src.get_i8()));
            }
            need(src, 1)?;
            let columns = src.get_u8();
            let data = if columns > 0 {
                need(src, 1 + 1 + 1)?;
                let _rows = src.get_u8();
                let _x = src.get_i8();
                let _z = src.get_i8();
                let len = VarInt::decode(src)?.0 as usize;
                need(src, len)?;
                let mut d = vec![0u8; len];
                src.copy_to_slice(&mut d);
                d
            } else {
                vec![]
            };
            Ok(Self {
                map_id,
                scale,
                tracking_position,
                icons,
                columns,
                data,
            })
        }
    }

    // ── MoveEntityPos (0x25) ──────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundMoveEntityPos {
        pub entity_id: VarInt,
        pub delta_x: i16,
        pub delta_y: i16,
        pub delta_z: i16,
        pub on_ground: bool,
    }

    impl PacketId for ClientboundMoveEntityPos {
        fn packet_id(_ver: u32) -> u8 {
            0x25
        }
    }

    impl Encode for ClientboundMoveEntityPos {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.entity_id.encode(dst)?;
            dst.put_i16(self.delta_x);
            dst.put_i16(self.delta_y);
            dst.put_i16(self.delta_z);
            dst.put_u8(self.on_ground as u8);
            Ok(())
        }
    }

    impl Decode for ClientboundMoveEntityPos {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let entity_id = VarInt::decode(src)?;
            need(src, 2 + 2 + 2 + 1)?;
            Ok(Self {
                entity_id,
                delta_x: src.get_i16(),
                delta_y: src.get_i16(),
                delta_z: src.get_i16(),
                on_ground: src.get_u8() != 0,
            })
        }
    }

    // ── MoveEntityPosRot (0x26) ───────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundMoveEntityPosRot {
        pub entity_id: VarInt,
        pub delta_x: i16,
        pub delta_y: i16,
        pub delta_z: i16,
        pub yaw: u8,
        pub pitch: u8,
        pub on_ground: bool,
    }

    impl PacketId for ClientboundMoveEntityPosRot {
        fn packet_id(_ver: u32) -> u8 {
            0x26
        }
    }

    impl Encode for ClientboundMoveEntityPosRot {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.entity_id.encode(dst)?;
            dst.put_i16(self.delta_x);
            dst.put_i16(self.delta_y);
            dst.put_i16(self.delta_z);
            dst.put_u8(self.yaw);
            dst.put_u8(self.pitch);
            dst.put_u8(self.on_ground as u8);
            Ok(())
        }
    }

    impl Decode for ClientboundMoveEntityPosRot {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let entity_id = VarInt::decode(src)?;
            need(src, 2 + 2 + 2 + 1 + 1 + 1)?;
            Ok(Self {
                entity_id,
                delta_x: src.get_i16(),
                delta_y: src.get_i16(),
                delta_z: src.get_i16(),
                yaw: src.get_u8(),
                pitch: src.get_u8(),
                on_ground: src.get_u8() != 0,
            })
        }
    }

    // ── MoveEntityRot (0x27) ──────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundMoveEntityRot {
        pub entity_id: VarInt,
        pub yaw: u8,
        pub pitch: u8,
        pub on_ground: bool,
    }

    impl PacketId for ClientboundMoveEntityRot {
        fn packet_id(_ver: u32) -> u8 {
            0x27
        }
    }

    impl Encode for ClientboundMoveEntityRot {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.entity_id.encode(dst)?;
            dst.put_u8(self.yaw);
            dst.put_u8(self.pitch);
            dst.put_u8(self.on_ground as u8);
            Ok(())
        }
    }

    impl Decode for ClientboundMoveEntityRot {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let entity_id = VarInt::decode(src)?;
            need(src, 1 + 1 + 1)?;
            Ok(Self {
                entity_id,
                yaw: src.get_u8(),
                pitch: src.get_u8(),
                on_ground: src.get_u8() != 0,
            })
        }
    }

    // ── MoveVehicle (0x29) ────────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundMoveVehicle {
        pub x: f64,
        pub y: f64,
        pub z: f64,
        pub yaw: f32,
        pub pitch: f32,
    }

    impl PacketId for ClientboundMoveVehicle {
        fn packet_id(_ver: u32) -> u8 {
            0x29
        }
    }

    impl Encode for ClientboundMoveVehicle {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_f64(self.x);
            dst.put_f64(self.y);
            dst.put_f64(self.z);
            dst.put_f32(self.yaw);
            dst.put_f32(self.pitch);
            Ok(())
        }
    }

    impl Decode for ClientboundMoveVehicle {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 8 + 8 + 8 + 4 + 4)?;
            Ok(Self {
                x: src.get_f64(),
                y: src.get_f64(),
                z: src.get_f64(),
                yaw: src.get_f32(),
                pitch: src.get_f32(),
            })
        }
    }

    // ── OpenSignEditor (0x2A) ─────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundOpenSignEditor {
        pub location: u64,
    }

    impl PacketId for ClientboundOpenSignEditor {
        fn packet_id(_ver: u32) -> u8 {
            0x2A
        }
    }

    impl Encode for ClientboundOpenSignEditor {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_u64(self.location);
            Ok(())
        }
    }

    impl Decode for ClientboundOpenSignEditor {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 8)?;
            Ok(Self {
                location: src.get_u64(),
            })
        }
    }

    // ── PlayerInfoUpdate (0x2E) — opaque raw ─────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundPlayerInfoUpdate {
        pub raw: Vec<u8>,
    }

    impl PacketId for ClientboundPlayerInfoUpdate {
        fn packet_id(_ver: u32) -> u8 {
            0x2E
        }
    }

    impl Encode for ClientboundPlayerInfoUpdate {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_slice(&self.raw);
            Ok(())
        }
    }

    impl Decode for ClientboundPlayerInfoUpdate {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let len = src.remaining();
            let mut raw = vec![0u8; len];
            src.copy_to_slice(&mut raw);
            Ok(Self { raw })
        }
    }

    // ── Recipes (0x31) — opaque raw ───────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundRecipes {
        pub raw: Vec<u8>,
    }

    impl PacketId for ClientboundRecipes {
        fn packet_id(_ver: u32) -> u8 {
            0x31
        }
    }

    impl Encode for ClientboundRecipes {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_slice(&self.raw);
            Ok(())
        }
    }

    impl Decode for ClientboundRecipes {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let len = src.remaining();
            let mut raw = vec![0u8; len];
            src.copy_to_slice(&mut raw);
            Ok(Self { raw })
        }
    }

    // ── RemoveEntities (0x32) ─────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundRemoveEntities {
        pub entity_ids: Vec<VarInt>,
    }

    impl PacketId for ClientboundRemoveEntities {
        fn packet_id(_ver: u32) -> u8 {
            0x32
        }
    }

    impl Encode for ClientboundRemoveEntities {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            VarInt(self.entity_ids.len() as i32).encode(dst)?;
            for id in &self.entity_ids {
                id.encode(dst)?;
            }
            Ok(())
        }
    }

    impl Decode for ClientboundRemoveEntities {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let count = VarInt::decode(src)?.0 as usize;
            let mut entity_ids = Vec::with_capacity(count);
            for _ in 0..count {
                entity_ids.push(VarInt::decode(src)?);
            }
            Ok(Self { entity_ids })
        }
    }

    // ── RemoveEntityEffect (0x33) ─────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundRemoveEntityEffect {
        pub entity_id: VarInt,
        pub effect_id: u8,
    }

    impl PacketId for ClientboundRemoveEntityEffect {
        fn packet_id(_ver: u32) -> u8 {
            0x33
        }
    }

    impl Encode for ClientboundRemoveEntityEffect {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.entity_id.encode(dst)?;
            dst.put_u8(self.effect_id);
            Ok(())
        }
    }

    impl Decode for ClientboundRemoveEntityEffect {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let entity_id = VarInt::decode(src)?;
            need(src, 1)?;
            Ok(Self {
                entity_id,
                effect_id: src.get_u8(),
            })
        }
    }

    // ── ResourcePackPush (0x34) ───────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundResourcePackPush {
        pub url: String,
        pub hash: String,
    }

    impl PacketId for ClientboundResourcePackPush {
        fn packet_id(_ver: u32) -> u8 {
            0x34
        }
    }

    impl Encode for ClientboundResourcePackPush {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            encode_str(&self.url, dst)?;
            encode_str(&self.hash, dst)
        }
    }

    impl Decode for ClientboundResourcePackPush {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let url = decode_str(src, "ClientboundResourcePackPush url")?;
            let hash = decode_str(src, "ClientboundResourcePackPush hash")?;
            Ok(Self { url, hash })
        }
    }

    // ── RotateHead (0x36) ─────────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundRotateHead {
        pub entity_id: VarInt,
        pub head_yaw: u8,
    }

    impl PacketId for ClientboundRotateHead {
        fn packet_id(_ver: u32) -> u8 {
            0x36
        }
    }

    impl Encode for ClientboundRotateHead {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.entity_id.encode(dst)?;
            dst.put_u8(self.head_yaw);
            Ok(())
        }
    }

    impl Decode for ClientboundRotateHead {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let entity_id = VarInt::decode(src)?;
            need(src, 1)?;
            Ok(Self {
                entity_id,
                head_yaw: src.get_u8(),
            })
        }
    }

    // ── SelectAdvancementsTab (0x37) ──────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundSelectAdvancementsTab {
        pub has_id: bool,
        pub tab_id: Option<String>,
    }

    impl PacketId for ClientboundSelectAdvancementsTab {
        fn packet_id(_ver: u32) -> u8 {
            0x37
        }
    }

    impl Encode for ClientboundSelectAdvancementsTab {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_u8(self.has_id as u8);
            if let Some(id) = &self.tab_id {
                encode_str(id, dst)?;
            }
            Ok(())
        }
    }

    impl Decode for ClientboundSelectAdvancementsTab {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 1)?;
            let has_id = src.get_u8() != 0;
            let tab_id = if has_id {
                Some(decode_str(src, "ClientboundSelectAdvancementsTab tab_id")?)
            } else {
                None
            };
            Ok(Self { has_id, tab_id })
        }
    }

    // ── SetCamera (0x39) ──────────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundSetCamera {
        pub camera_id: VarInt,
    }

    impl PacketId for ClientboundSetCamera {
        fn packet_id(_ver: u32) -> u8 {
            0x39
        }
    }

    impl Encode for ClientboundSetCamera {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.camera_id.encode(dst)
        }
    }

    impl Decode for ClientboundSetCamera {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            Ok(Self {
                camera_id: VarInt::decode(src)?,
            })
        }
    }

    // ── SetHeldItem (0x3A) ────────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundSetHeldItem {
        pub slot: u8,
    }

    impl PacketId for ClientboundSetHeldItem {
        fn packet_id(_ver: u32) -> u8 {
            0x3A
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

    // ── SetEntityLink (0x3D) ──────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundSetEntityLink {
        pub attached_entity_id: i32,
        pub holding_entity_id: i32,
    }

    impl PacketId for ClientboundSetEntityLink {
        fn packet_id(_ver: u32) -> u8 {
            0x3D
        }
    }

    impl Encode for ClientboundSetEntityLink {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_i32(self.attached_entity_id);
            dst.put_i32(self.holding_entity_id);
            Ok(())
        }
    }

    impl Decode for ClientboundSetEntityLink {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 4 + 4)?;
            Ok(Self {
                attached_entity_id: src.get_i32(),
                holding_entity_id: src.get_i32(),
            })
        }
    }

    // ── SetEntityMotion (0x3E) ────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundSetEntityMotion {
        pub entity_id: VarInt,
        pub velocity_x: i16,
        pub velocity_y: i16,
        pub velocity_z: i16,
    }

    impl PacketId for ClientboundSetEntityMotion {
        fn packet_id(_ver: u32) -> u8 {
            0x3E
        }
    }

    impl Encode for ClientboundSetEntityMotion {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.entity_id.encode(dst)?;
            dst.put_i16(self.velocity_x);
            dst.put_i16(self.velocity_y);
            dst.put_i16(self.velocity_z);
            Ok(())
        }
    }

    impl Decode for ClientboundSetEntityMotion {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let entity_id = VarInt::decode(src)?;
            need(src, 2 + 2 + 2)?;
            Ok(Self {
                entity_id,
                velocity_x: src.get_i16(),
                velocity_y: src.get_i16(),
                velocity_z: src.get_i16(),
            })
        }
    }

    // ── SetEquipment (0x3F) ───────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundSetEquipment {
        pub entity_id: VarInt,
        pub slot: VarInt,
        pub item: Vec<u8>,
    }

    impl PacketId for ClientboundSetEquipment {
        fn packet_id(_ver: u32) -> u8 {
            0x3F
        }
    }

    impl Encode for ClientboundSetEquipment {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.entity_id.encode(dst)?;
            self.slot.encode(dst)?;
            dst.put_slice(&self.item);
            Ok(())
        }
    }

    impl Decode for ClientboundSetEquipment {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let entity_id = VarInt::decode(src)?;
            let slot = VarInt::decode(src)?;
            let len = src.remaining();
            let mut item = vec![0u8; len];
            src.copy_to_slice(&mut item);
            Ok(Self {
                entity_id,
                slot,
                item,
            })
        }
    }

    // ── SetExperience (0x40) ──────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundSetExperience {
        pub experience_bar: f32,
        pub level: VarInt,
        pub total_experience: VarInt,
    }

    impl PacketId for ClientboundSetExperience {
        fn packet_id(_ver: u32) -> u8 {
            0x40
        }
    }

    impl Encode for ClientboundSetExperience {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_f32(self.experience_bar);
            self.level.encode(dst)?;
            self.total_experience.encode(dst)
        }
    }

    impl Decode for ClientboundSetExperience {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 4)?;
            let experience_bar = src.get_f32();
            let level = VarInt::decode(src)?;
            let total_experience = VarInt::decode(src)?;
            Ok(Self {
                experience_bar,
                level,
                total_experience,
            })
        }
    }

    // ── SetHealth (0x41) ──────────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundSetHealth {
        pub health: f32,
        pub food: VarInt,
        pub food_saturation: f32,
    }

    impl PacketId for ClientboundSetHealth {
        fn packet_id(_ver: u32) -> u8 {
            0x41
        }
    }

    impl Encode for ClientboundSetHealth {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_f32(self.health);
            self.food.encode(dst)?;
            dst.put_f32(self.food_saturation);
            Ok(())
        }
    }

    impl Decode for ClientboundSetHealth {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 4)?;
            let health = src.get_f32();
            let food = VarInt::decode(src)?;
            need(src, 4)?;
            let food_saturation = src.get_f32();
            Ok(Self {
                health,
                food,
                food_saturation,
            })
        }
    }

    // ── SetScoreboardObjective (0x42) ─────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundSetScoreboardObjective {
        pub objective_name: String,
        pub mode: u8,
        pub objective_value: Option<String>,
        pub kind: Option<String>,
    }

    impl PacketId for ClientboundSetScoreboardObjective {
        fn packet_id(_ver: u32) -> u8 {
            0x42
        }
    }

    impl Encode for ClientboundSetScoreboardObjective {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            encode_str(&self.objective_name, dst)?;
            dst.put_u8(self.mode);
            if self.mode == 0 || self.mode == 2 {
                if let Some(v) = &self.objective_value {
                    encode_str(v, dst)?;
                }
                if let Some(k) = &self.kind {
                    encode_str(k, dst)?;
                }
            }
            Ok(())
        }
    }

    impl Decode for ClientboundSetScoreboardObjective {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let objective_name = decode_str(src, "ClientboundSetScoreboardObjective name")?;
            need(src, 1)?;
            let mode = src.get_u8();
            let (objective_value, kind) = if mode == 0 || mode == 2 {
                let v = decode_str(src, "ClientboundSetScoreboardObjective value")?;
                let k = decode_str(src, "ClientboundSetScoreboardObjective kind")?;
                (Some(v), Some(k))
            } else {
                (None, None)
            };
            Ok(Self {
                objective_name,
                mode,
                objective_value,
                kind,
            })
        }
    }

    // ── SetScoreboardScore (0x45) ─────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundSetScoreboardScore {
        pub entity_name: String,
        pub action: u8,
        pub objective_name: String,
        pub value: Option<VarInt>,
    }

    impl PacketId for ClientboundSetScoreboardScore {
        fn packet_id(_ver: u32) -> u8 {
            0x45
        }
    }

    impl Encode for ClientboundSetScoreboardScore {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            encode_str(&self.entity_name, dst)?;
            dst.put_u8(self.action);
            encode_str(&self.objective_name, dst)?;
            if self.action != 1 {
                if let Some(v) = &self.value {
                    v.encode(dst)?;
                }
            }
            Ok(())
        }
    }

    impl Decode for ClientboundSetScoreboardScore {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let entity_name = decode_str(src, "ClientboundSetScoreboardScore entity_name")?;
            need(src, 1)?;
            let action = src.get_u8();
            let objective_name = decode_str(src, "ClientboundSetScoreboardScore objective_name")?;
            let value = if action != 1 {
                Some(VarInt::decode(src)?)
            } else {
                None
            };
            Ok(Self {
                entity_name,
                action,
                objective_name,
                value,
            })
        }
    }

    // ── SetTime (0x47) ────────────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundSetTime {
        pub world_age: i64,
        pub time_of_day: i64,
    }

    impl PacketId for ClientboundSetTime {
        fn packet_id(_ver: u32) -> u8 {
            0x47
        }
    }

    impl Encode for ClientboundSetTime {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_i64(self.world_age);
            dst.put_i64(self.time_of_day);
            Ok(())
        }
    }

    impl Decode for ClientboundSetTime {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 8 + 8)?;
            Ok(Self {
                world_age: src.get_i64(),
                time_of_day: src.get_i64(),
            })
        }
    }

    // ── Sound (0x48) ──────────────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundSound {
        pub sound_id: VarInt,
        pub sound_category: VarInt,
        pub effect_pos_x: i32,
        pub effect_pos_y: i32,
        pub effect_pos_z: i32,
        pub volume: f32,
        pub pitch: f32,
    }

    impl PacketId for ClientboundSound {
        fn packet_id(_ver: u32) -> u8 {
            0x48
        }
    }

    impl Encode for ClientboundSound {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.sound_id.encode(dst)?;
            self.sound_category.encode(dst)?;
            dst.put_i32(self.effect_pos_x);
            dst.put_i32(self.effect_pos_y);
            dst.put_i32(self.effect_pos_z);
            dst.put_f32(self.volume);
            dst.put_f32(self.pitch);
            Ok(())
        }
    }

    impl Decode for ClientboundSound {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let sound_id = VarInt::decode(src)?;
            let sound_category = VarInt::decode(src)?;
            need(src, 4 + 4 + 4 + 4 + 4)?;
            Ok(Self {
                sound_id,
                sound_category,
                effect_pos_x: src.get_i32(),
                effect_pos_y: src.get_i32(),
                effect_pos_z: src.get_i32(),
                volume: src.get_f32(),
                pitch: src.get_f32(),
            })
        }
    }

    // ── TabList (0x48 tab-list) ───────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundTabList {
        pub header: String,
        pub footer: String,
    }

    impl PacketId for ClientboundTabList {
        fn packet_id(_ver: u32) -> u8 {
            0x48
        }
    }

    impl Encode for ClientboundTabList {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            encode_str(&self.header, dst)?;
            encode_str(&self.footer, dst)
        }
    }

    impl Decode for ClientboundTabList {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let header = decode_str(src, "ClientboundTabList header")?;
            let footer = decode_str(src, "ClientboundTabList footer")?;
            Ok(Self { header, footer })
        }
    }

    // ── TakeItemEntity (0x4B) ─────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundTakeItemEntity {
        pub collected_entity_id: VarInt,
        pub collector_entity_id: VarInt,
        pub pickup_item_count: VarInt,
    }

    impl PacketId for ClientboundTakeItemEntity {
        fn packet_id(_ver: u32) -> u8 {
            0x4B
        }
    }

    impl Encode for ClientboundTakeItemEntity {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.collected_entity_id.encode(dst)?;
            self.collector_entity_id.encode(dst)?;
            self.pickup_item_count.encode(dst)
        }
    }

    impl Decode for ClientboundTakeItemEntity {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            Ok(Self {
                collected_entity_id: VarInt::decode(src)?,
                collector_entity_id: VarInt::decode(src)?,
                pickup_item_count: VarInt::decode(src)?,
            })
        }
    }

    // ── TeleportEntity (0x4C) ─────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundTeleportEntity {
        pub entity_id: VarInt,
        pub x: f64,
        pub y: f64,
        pub z: f64,
        pub yaw: u8,
        pub pitch: u8,
        pub on_ground: bool,
    }

    impl PacketId for ClientboundTeleportEntity {
        fn packet_id(_ver: u32) -> u8 {
            0x4C
        }
    }

    impl Encode for ClientboundTeleportEntity {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.entity_id.encode(dst)?;
            dst.put_f64(self.x);
            dst.put_f64(self.y);
            dst.put_f64(self.z);
            dst.put_u8(self.yaw);
            dst.put_u8(self.pitch);
            dst.put_u8(self.on_ground as u8);
            Ok(())
        }
    }

    impl Decode for ClientboundTeleportEntity {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let entity_id = VarInt::decode(src)?;
            need(src, 8 + 8 + 8 + 1 + 1 + 1)?;
            Ok(Self {
                entity_id,
                x: src.get_f64(),
                y: src.get_f64(),
                z: src.get_f64(),
                yaw: src.get_u8(),
                pitch: src.get_u8(),
                on_ground: src.get_u8() != 0,
            })
        }
    }

    // ── UpdateEffects (0x4F) ──────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundUpdateEffects {
        pub entity_id: VarInt,
        pub effect_id: u8,
        pub amplifier: u8,
        pub duration: VarInt,
        pub flags: u8,
    }

    impl PacketId for ClientboundUpdateEffects {
        fn packet_id(_ver: u32) -> u8 {
            0x4F
        }
    }

    impl Encode for ClientboundUpdateEffects {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.entity_id.encode(dst)?;
            dst.put_u8(self.effect_id);
            dst.put_u8(self.amplifier);
            self.duration.encode(dst)?;
            dst.put_u8(self.flags);
            Ok(())
        }
    }

    impl Decode for ClientboundUpdateEffects {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let entity_id = VarInt::decode(src)?;
            need(src, 1 + 1)?;
            let effect_id = src.get_u8();
            let amplifier = src.get_u8();
            let duration = VarInt::decode(src)?;
            need(src, 1)?;
            let flags = src.get_u8();
            Ok(Self {
                entity_id,
                effect_id,
                amplifier,
                duration,
                flags,
            })
        }
    }

    // ── SpawnGlobalEntity (0x02) ──────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundSpawnGlobalEntity {
        pub entity_id: i32, // FIX: V 1.12.2 je toto natvrdo i32, nikoliv VarInt!
        pub kind: u8,       // Vždy 1 pro blesk
        pub x: f64,
        pub y: f64,
        pub z: f64,
    }

    impl PacketId for ClientboundSpawnGlobalEntity {
        fn packet_id(_ver: u32) -> u8 {
            0x02
        }
    }

    impl Encode for ClientboundSpawnGlobalEntity {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.clear();
            // 4 bajty (i32 Big Endian)
            dst.put_i32(self.entity_id);

            // 1 bajt (u8)
            dst.put_u8(self.kind);

            // 3x 8 bajtů (f64 Big Endian)
            dst.put_f64(self.x);
            dst.put_f64(self.y);
            dst.put_f64(self.z);
            Ok(())
        }
    }

    impl Decode for ClientboundSpawnGlobalEntity {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            // Celková pevná délka v 1.12.2: 4 (i32) + 1 (u8) + 24 (3x f64) = 29 bajtů
            if src.remaining() < 29 {
                return Err(ProtocolError::Io(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "Nedostatek dat pro ClientboundSpawnGlobalEntity (vyzadovano 29B)",
                )));
            }

            let entity_id = src.get_i32();
            let kind = src.get_u8();
            let x = src.get_f64();
            let y = src.get_f64();
            let z = src.get_f64();

            Ok(Self {
                entity_id,
                kind,
                x,
                y,
                z,
            })
        }
    }

    // ── ServerboundCustomPayload (0x0A) ───────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ServerboundCustomPayload {
        pub channel: String,
        pub data: Vec<u8>,
    }

    impl PacketId for ServerboundCustomPayload {
        fn packet_id(_ver: u32) -> u8 {
            0x0A
        }
    }

    impl Encode for ServerboundCustomPayload {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            encode_str(&self.channel, dst)?;
            dst.put_slice(&self.data);
            Ok(())
        }
    }

    impl Decode for ServerboundCustomPayload {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let channel = decode_str(src, "ServerboundCustomPayload channel")?;
            let len = src.remaining();
            let mut data = vec![0u8; len];
            src.copy_to_slice(&mut data);
            Ok(Self { channel, data })
        }
    }

    // ── Serverbound packets ───────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ServerboundTeleportConfirm {
        pub teleport_id: VarInt,
    }
    impl PacketId for ServerboundTeleportConfirm {
        fn packet_id(_ver: u32) -> u8 {
            0x00
        }
    }
    impl Encode for ServerboundTeleportConfirm {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.teleport_id.encode(dst)
        }
    }
    impl Decode for ServerboundTeleportConfirm {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            Ok(Self {
                teleport_id: VarInt::decode(src)?,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ServerboundCommandSuggestion {
        pub text: String,
        pub block: Option<u64>,
    }
    impl PacketId for ServerboundCommandSuggestion {
        fn packet_id(_ver: u32) -> u8 {
            0x01
        }
    }
    impl Encode for ServerboundCommandSuggestion {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            encode_str(&self.text, dst)?;
            dst.put_u8(self.block.is_some() as u8);
            if let Some(b) = self.block {
                dst.put_u64(b);
            }
            Ok(())
        }
    }
    impl Decode for ServerboundCommandSuggestion {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let text = decode_str(src, "ServerboundCommandSuggestion text")?;
            need(src, 1)?;
            let has_block = src.get_u8() != 0;
            let block = if has_block {
                need(src, 8)?;
                Some(src.get_u64())
            } else {
                None
            };
            Ok(Self { text, block })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ServerboundClientStatus {
        pub action_id: VarInt,
    }
    impl PacketId for ServerboundClientStatus {
        fn packet_id(_ver: u32) -> u8 {
            0x03
        }
    }
    impl Encode for ServerboundClientStatus {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.action_id.encode(dst)
        }
    }
    impl Decode for ServerboundClientStatus {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            Ok(Self {
                action_id: VarInt::decode(src)?,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ServerboundClientSettings {
        pub locale: String,
        pub view_distance: u8,
        pub chat_mode: VarInt,
        pub chat_colors: bool,
        pub displayed_skin_parts: u8,
        pub main_hand: VarInt,
    }
    impl PacketId for ServerboundClientSettings {
        fn packet_id(_ver: u32) -> u8 {
            0x04
        }
    }
    impl Encode for ServerboundClientSettings {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            encode_str(&self.locale, dst)?;
            dst.put_u8(self.view_distance);
            self.chat_mode.encode(dst)?;
            dst.put_u8(self.chat_colors as u8);
            dst.put_u8(self.displayed_skin_parts);
            self.main_hand.encode(dst)
        }
    }
    impl Decode for ServerboundClientSettings {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let locale = decode_str(src, "ServerboundClientSettings locale")?;
            need(src, 1)?;
            let view_distance = src.get_u8();
            let chat_mode = VarInt::decode(src)?;
            need(src, 1 + 1)?;
            let chat_colors = src.get_u8() != 0;
            let displayed_skin_parts = src.get_u8();
            let main_hand = VarInt::decode(src)?;
            Ok(Self {
                locale,
                view_distance,
                chat_mode,
                chat_colors,
                displayed_skin_parts,
                main_hand,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ServerboundConfirmTransaction {
        pub window_id: i8,
        pub action_number: i16,
        pub accepted: bool,
    }
    impl PacketId for ServerboundConfirmTransaction {
        fn packet_id(_ver: u32) -> u8 {
            0x05
        }
    }
    impl Encode for ServerboundConfirmTransaction {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_i8(self.window_id);
            dst.put_i16(self.action_number);
            dst.put_u8(self.accepted as u8);
            Ok(())
        }
    }
    impl Decode for ServerboundConfirmTransaction {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 1 + 2 + 1)?;
            Ok(Self {
                window_id: src.get_i8(),
                action_number: src.get_i16(),
                accepted: src.get_u8() != 0,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ServerboundEnchantItem {
        pub window_id: i8,
        pub enchantment: i8,
    }
    impl PacketId for ServerboundEnchantItem {
        fn packet_id(_ver: u32) -> u8 {
            0x06
        }
    }
    impl Encode for ServerboundEnchantItem {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_i8(self.window_id);
            dst.put_i8(self.enchantment);
            Ok(())
        }
    }
    impl Decode for ServerboundEnchantItem {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 1 + 1)?;
            Ok(Self {
                window_id: src.get_i8(),
                enchantment: src.get_i8(),
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ServerboundClickWindowButton {
        pub window_id: i8,
        pub button_id: i8,
    }
    impl PacketId for ServerboundClickWindowButton {
        fn packet_id(_ver: u32) -> u8 {
            0x06
        }
    }
    impl Encode for ServerboundClickWindowButton {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_i8(self.window_id);
            dst.put_i8(self.button_id);
            Ok(())
        }
    }
    impl Decode for ServerboundClickWindowButton {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 1 + 1)?;
            Ok(Self {
                window_id: src.get_i8(),
                button_id: src.get_i8(),
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ServerboundCloseWindow {
        pub window_id: u8,
    }
    impl PacketId for ServerboundCloseWindow {
        fn packet_id(_ver: u32) -> u8 {
            0x08
        }
    }
    impl Encode for ServerboundCloseWindow {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_u8(self.window_id);
            Ok(())
        }
    }
    impl Decode for ServerboundCloseWindow {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 1)?;
            Ok(Self {
                window_id: src.get_u8(),
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ServerboundMovePlayerStatusOnly {
        pub on_ground: bool,
    }
    impl PacketId for ServerboundMovePlayerStatusOnly {
        fn packet_id(_ver: u32) -> u8 {
            0x0F
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
    pub struct ServerboundVehicleMove {
        pub x: f64,
        pub y: f64,
        pub z: f64,
        pub yaw: f32,
        pub pitch: f32,
    }
    impl PacketId for ServerboundVehicleMove {
        fn packet_id(_ver: u32) -> u8 {
            0x10
        }
    }
    impl Encode for ServerboundVehicleMove {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_f64(self.x);
            dst.put_f64(self.y);
            dst.put_f64(self.z);
            dst.put_f32(self.yaw);
            dst.put_f32(self.pitch);
            Ok(())
        }
    }
    impl Decode for ServerboundVehicleMove {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 8 + 8 + 8 + 4 + 4)?;
            Ok(Self {
                x: src.get_f64(),
                y: src.get_f64(),
                z: src.get_f64(),
                yaw: src.get_f32(),
                pitch: src.get_f32(),
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ServerboundSteerBoat {
        pub left_paddle: bool,
        pub right_paddle: bool,
    }
    impl PacketId for ServerboundSteerBoat {
        fn packet_id(_ver: u32) -> u8 {
            0x11
        }
    }
    impl Encode for ServerboundSteerBoat {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_u8(self.left_paddle as u8);
            dst.put_u8(self.right_paddle as u8);
            Ok(())
        }
    }
    impl Decode for ServerboundSteerBoat {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 1 + 1)?;
            Ok(Self {
                left_paddle: src.get_u8() != 0,
                right_paddle: src.get_u8() != 0,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ServerboundPlayerAction {
        pub status: VarInt,
        pub location: u64,
        pub face: u8,
    }
    impl PacketId for ServerboundPlayerAction {
        fn packet_id(_ver: u32) -> u8 {
            0x13
        }
    }
    impl Encode for ServerboundPlayerAction {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.status.encode(dst)?;
            dst.put_u64(self.location);
            dst.put_u8(self.face);
            Ok(())
        }
    }
    impl Decode for ServerboundPlayerAction {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let status = VarInt::decode(src)?;
            need(src, 8 + 1)?;
            Ok(Self {
                status,
                location: src.get_u64(),
                face: src.get_u8(),
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ServerboundEntityAction {
        pub entity_id: VarInt,
        pub action_id: VarInt,
        pub jump_boost: VarInt,
    }
    impl PacketId for ServerboundEntityAction {
        fn packet_id(_ver: u32) -> u8 {
            0x14
        }
    }
    impl Encode for ServerboundEntityAction {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.entity_id.encode(dst)?;
            self.action_id.encode(dst)?;
            self.jump_boost.encode(dst)
        }
    }
    impl Decode for ServerboundEntityAction {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            Ok(Self {
                entity_id: VarInt::decode(src)?,
                action_id: VarInt::decode(src)?,
                jump_boost: VarInt::decode(src)?,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ServerboundSteerVehicle {
        pub sideways: f32,
        pub forward: f32,
        pub flags: u8,
    }
    impl PacketId for ServerboundSteerVehicle {
        fn packet_id(_ver: u32) -> u8 {
            0x15
        }
    }
    impl Encode for ServerboundSteerVehicle {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_f32(self.sideways);
            dst.put_f32(self.forward);
            dst.put_u8(self.flags);
            Ok(())
        }
    }
    impl Decode for ServerboundSteerVehicle {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 4 + 4 + 1)?;
            Ok(Self {
                sideways: src.get_f32(),
                forward: src.get_f32(),
                flags: src.get_u8(),
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ServerboundResourcePackStatus {
        pub result: VarInt,
    }
    impl PacketId for ServerboundResourcePackStatus {
        fn packet_id(_ver: u32) -> u8 {
            0x17
        }
    }
    impl Encode for ServerboundResourcePackStatus {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.result.encode(dst)
        }
    }
    impl Decode for ServerboundResourcePackStatus {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            Ok(Self {
                result: VarInt::decode(src)?,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ServerboundRecipeBookSeenRecipe {
        pub recipe_id: VarInt,
    }
    impl PacketId for ServerboundRecipeBookSeenRecipe {
        fn packet_id(_ver: u32) -> u8 {
            0x18
        }
    }
    impl Encode for ServerboundRecipeBookSeenRecipe {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.recipe_id.encode(dst)
        }
    }
    impl Decode for ServerboundRecipeBookSeenRecipe {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            Ok(Self {
                recipe_id: VarInt::decode(src)?,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ServerboundHeldItemChange {
        pub slot: i16,
    }
    impl PacketId for ServerboundHeldItemChange {
        fn packet_id(_ver: u32) -> u8 {
            0x1A
        }
    }
    impl Encode for ServerboundHeldItemChange {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_i16(self.slot);
            Ok(())
        }
    }
    impl Decode for ServerboundHeldItemChange {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 2)?;
            Ok(Self {
                slot: src.get_i16(),
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ServerboundCreativeInventoryAction {
        pub slot: i16,
        pub clicked_item: Vec<u8>,
    }
    impl PacketId for ServerboundCreativeInventoryAction {
        fn packet_id(_ver: u32) -> u8 {
            0x1B
        }
    }
    impl Encode for ServerboundCreativeInventoryAction {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_i16(self.slot);
            dst.put_slice(&self.clicked_item);
            Ok(())
        }
    }
    impl Decode for ServerboundCreativeInventoryAction {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 2)?;
            let slot = src.get_i16();
            let len = src.remaining();
            let mut clicked_item = vec![0u8; len];
            src.copy_to_slice(&mut clicked_item);
            Ok(Self { slot, clicked_item })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ServerboundUpdateSign {
        pub location: u64,
        pub lines: [String; 4],
    }
    impl PacketId for ServerboundUpdateSign {
        fn packet_id(_ver: u32) -> u8 {
            0x1C
        }
    }
    impl Encode for ServerboundUpdateSign {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_u64(self.location);
            for line in &self.lines {
                encode_str(line, dst)?;
            }
            Ok(())
        }
    }
    impl Decode for ServerboundUpdateSign {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 8)?;
            let location = src.get_u64();
            let lines = [
                decode_str(src, "ServerboundUpdateSign line1")?,
                decode_str(src, "ServerboundUpdateSign line2")?,
                decode_str(src, "ServerboundUpdateSign line3")?,
                decode_str(src, "ServerboundUpdateSign line4")?,
            ];
            Ok(Self { location, lines })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ServerboundAnimation {
        pub hand: VarInt,
    }
    impl PacketId for ServerboundAnimation {
        fn packet_id(_ver: u32) -> u8 {
            0x1D
        }
    }
    impl Encode for ServerboundAnimation {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.hand.encode(dst)
        }
    }
    impl Decode for ServerboundAnimation {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            Ok(Self {
                hand: VarInt::decode(src)?,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ServerboundSpectate {
        pub target_player: Uuid,
    }
    impl PacketId for ServerboundSpectate {
        fn packet_id(_ver: u32) -> u8 {
            0x1E
        }
    }
    impl Encode for ServerboundSpectate {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_slice(self.target_player.as_bytes());
            Ok(())
        }
    }
    impl Decode for ServerboundSpectate {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 16)?;
            let mut b = [0u8; 16];
            src.copy_to_slice(&mut b);
            Ok(Self {
                target_player: Uuid::from_bytes(b),
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ServerboundPlayerBlockPlacement {
        pub location: u64,
        pub face: VarInt,
        pub hand: VarInt,
        pub cursor_x: f32,
        pub cursor_y: f32,
        pub cursor_z: f32,
    }
    impl PacketId for ServerboundPlayerBlockPlacement {
        fn packet_id(_ver: u32) -> u8 {
            0x1F
        }
    }
    impl Encode for ServerboundPlayerBlockPlacement {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_u64(self.location);
            self.face.encode(dst)?;
            self.hand.encode(dst)?;
            dst.put_f32(self.cursor_x);
            dst.put_f32(self.cursor_y);
            dst.put_f32(self.cursor_z);
            Ok(())
        }
    }
    impl Decode for ServerboundPlayerBlockPlacement {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 8)?;
            let location = src.get_u64();
            let face = VarInt::decode(src)?;
            let hand = VarInt::decode(src)?;
            need(src, 4 + 4 + 4)?;
            Ok(Self {
                location,
                face,
                hand,
                cursor_x: src.get_f32(),
                cursor_y: src.get_f32(),
                cursor_z: src.get_f32(),
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ServerboundUseItem {
        pub hand: VarInt,
    }
    impl PacketId for ServerboundUseItem {
        fn packet_id(_ver: u32) -> u8 {
            0x20
        }
    }
    impl Encode for ServerboundUseItem {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.hand.encode(dst)
        }
    }
    impl Decode for ServerboundUseItem {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            Ok(Self {
                hand: VarInt::decode(src)?,
            })
        }
    }

    // ── Opaque raw stubs ──────────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct AttributeModifier {
        pub uuid: Uuid,
        pub amount: f64,
        pub operation: u8,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct Attribute {
        pub key: String,
        pub value: f64,
        pub modifiers: Vec<AttributeModifier>,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundUpdateAttributes {
        pub entity_id: VarInt,
        pub attributes: Vec<Attribute>,
    }

    impl PacketId for ClientboundUpdateAttributes {
        fn packet_id(_ver: u32) -> u8 {
            0x4E
        }
    }

    impl Encode for ClientboundUpdateAttributes {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.entity_id.encode(dst)?;
            dst.put_i32(self.attributes.len() as i32); // NOTE: i32 not VarInt in 1.12.2
            for attr in &self.attributes {
                encode_str(&attr.key, dst)?;
                dst.put_f64(attr.value);
                VarInt(attr.modifiers.len() as i32).encode(dst)?;
                for m in &attr.modifiers {
                    dst.put_slice(m.uuid.as_bytes());
                    dst.put_f64(m.amount);
                    dst.put_u8(m.operation);
                }
            }
            Ok(())
        }
    }

    impl Decode for ClientboundUpdateAttributes {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let entity_id = VarInt::decode(src)?;
            need(src, 4)?;
            let count = src.get_i32() as usize; // NOTE: i32 not VarInt in 1.12.2
            let mut attributes = Vec::with_capacity(count);
            for _ in 0..count {
                let key = decode_str(src, "attribute key")?;
                need(src, 8)?;
                let value = src.get_f64();
                let mod_count = VarInt::decode(src)?.0 as usize;
                let mut modifiers = Vec::with_capacity(mod_count);
                for _ in 0..mod_count {
                    need(src, 16 + 8 + 1)?;
                    let mut b = [0u8; 16];
                    src.copy_to_slice(&mut b);
                    let uuid = Uuid::from_bytes(b);
                    let amount = src.get_f64();
                    let operation = src.get_u8();
                    modifiers.push(AttributeModifier {
                        uuid,
                        amount,
                        operation,
                    });
                }
                attributes.push(Attribute {
                    key,
                    value,
                    modifiers,
                });
            }
            Ok(Self {
                entity_id,
                attributes,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct AdvancementDisplay {
        pub title: String,
        pub description: String,
        pub icon_item_id: i16, // -1 = no icon
        pub icon_count: i8,
        pub icon_damage: i16,
        pub icon_nbt: Nbt,
        pub frame_type: VarInt,         // 0=task, 1=challenge, 2=goal
        pub flags: i32,                 // bit 0=has bg, bit 1=show toast, bit 2=hidden
        pub background: Option<String>, // only present if flags & 1
        pub x: f32,
        pub y: f32,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct Advancement {
        pub id: String,
        pub parent_id: Option<String>,
        pub display: Option<AdvancementDisplay>,
        pub criteria: Vec<String>,
        pub requirements: Vec<Vec<String>>,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct AdvancementProgress {
        pub id: String,
        pub criteria: Vec<(String, Option<i64>)>, // criterion id + completion timestamp if done
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundUpdateAdvancements {
        pub reset: bool,
        pub added: Vec<Advancement>,
        pub removed: Vec<String>,
        pub progress: Vec<AdvancementProgress>,
    }

    impl PacketId for ClientboundUpdateAdvancements {
        fn packet_id(_ver: u32) -> u8 {
            0x4D
        }
    }

    impl Encode for ClientboundUpdateAdvancements {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_u8(self.reset as u8);

            VarInt(self.added.len() as i32).encode(dst)?;
            for adv in &self.added {
                encode_str(&adv.id, dst)?;
                dst.put_u8(adv.parent_id.is_some() as u8);
                if let Some(p) = &adv.parent_id {
                    encode_str(p, dst)?;
                }
                dst.put_u8(adv.display.is_some() as u8);
                if let Some(d) = &adv.display {
                    encode_str(&d.title, dst)?;
                    encode_str(&d.description, dst)?;
                    // icon slot: item_id i16, if not -1: count i8 + damage i16 + nbt
                    dst.put_i16(d.icon_item_id);
                    if d.icon_item_id != -1 {
                        dst.put_i8(d.icon_count);
                        dst.put_i16(d.icon_damage);
                        d.icon_nbt.encode(dst)?;
                    }
                    d.frame_type.encode(dst)?;
                    dst.put_i32(d.flags);
                    if d.flags & 1 != 0 {
                        if let Some(bg) = &d.background {
                            encode_str(bg, dst)?;
                        }
                    }
                    dst.put_f32(d.x);
                    dst.put_f32(d.y);
                }
                VarInt(adv.criteria.len() as i32).encode(dst)?;
                for c in &adv.criteria {
                    encode_str(c, dst)?;
                }
                VarInt(adv.requirements.len() as i32).encode(dst)?;
                for req in &adv.requirements {
                    VarInt(req.len() as i32).encode(dst)?;
                    for r in req {
                        encode_str(r, dst)?;
                    }
                }
            }

            VarInt(self.removed.len() as i32).encode(dst)?;
            for id in &self.removed {
                encode_str(id, dst)?;
            }

            VarInt(self.progress.len() as i32).encode(dst)?;
            for prog in &self.progress {
                encode_str(&prog.id, dst)?;
                VarInt(prog.criteria.len() as i32).encode(dst)?;
                for (crit_id, timestamp) in &prog.criteria {
                    encode_str(crit_id, dst)?;
                    dst.put_u8(timestamp.is_some() as u8);
                    if let Some(ts) = timestamp {
                        dst.put_i64(*ts);
                    }
                }
            }
            Ok(())
        }
    }

    impl Decode for ClientboundUpdateAdvancements {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 1)?;
            let reset = src.get_u8() != 0;

            let added_count = VarInt::decode(src)?.0 as usize;
            let mut added = Vec::with_capacity(added_count);
            for _ in 0..added_count {
                let id = decode_str(src, "advancement id")?;
                need(src, 1)?;
                let has_parent = src.get_u8() != 0;
                let parent_id = if has_parent {
                    Some(decode_str(src, "advancement parent")?)
                } else {
                    None
                };
                need(src, 1)?;
                let has_display = src.get_u8() != 0;
                let display = if has_display {
                    let title = decode_str(src, "advancement title")?;
                    let description = decode_str(src, "advancement description")?;
                    need(src, 2)?;
                    let icon_item_id = src.get_i16();
                    let (icon_count, icon_damage, icon_nbt) = if icon_item_id != -1 {
                        need(src, 1 + 2)?;
                        let c = src.get_i8();
                        let d = src.get_i16();
                        let n = Nbt::decode(src)?;
                        (c, d, n)
                    } else {
                        (0, 0, Nbt::empty(""))
                    };
                    let frame_type = VarInt::decode(src)?;
                    need(src, 4)?;
                    let flags = src.get_i32();
                    let background = if flags & 1 != 0 {
                        Some(decode_str(src, "advancement background")?)
                    } else {
                        None
                    };
                    need(src, 4 + 4)?;
                    let x = src.get_f32();
                    let y = src.get_f32();
                    Some(AdvancementDisplay {
                        title,
                        description,
                        icon_item_id,
                        icon_count,
                        icon_damage,
                        icon_nbt,
                        frame_type,
                        flags,
                        background,
                        x,
                        y,
                    })
                } else {
                    None
                };
                let crit_count = VarInt::decode(src)?.0 as usize;
                let mut criteria = Vec::with_capacity(crit_count);
                for _ in 0..crit_count {
                    criteria.push(decode_str(src, "criterion id")?);
                }
                let req_count = VarInt::decode(src)?.0 as usize;
                let mut requirements = Vec::with_capacity(req_count);
                for _ in 0..req_count {
                    let len = VarInt::decode(src)?.0 as usize;
                    let mut req = Vec::with_capacity(len);
                    for _ in 0..len {
                        req.push(decode_str(src, "requirement")?);
                    }
                    requirements.push(req);
                }
                added.push(Advancement {
                    id,
                    parent_id,
                    display,
                    criteria,
                    requirements,
                });
            }

            let removed_count = VarInt::decode(src)?.0 as usize;
            let mut removed = Vec::with_capacity(removed_count);
            for _ in 0..removed_count {
                removed.push(decode_str(src, "removed advancement id")?);
            }

            let progress_count = VarInt::decode(src)?.0 as usize;
            let mut progress = Vec::with_capacity(progress_count);
            for _ in 0..progress_count {
                let id = decode_str(src, "progress id")?;
                let crit_count = VarInt::decode(src)?.0 as usize;
                let mut criteria = Vec::with_capacity(crit_count);
                for _ in 0..crit_count {
                    let crit_id = decode_str(src, "criterion id")?;
                    need(src, 1)?;
                    let achieved = src.get_u8() != 0;
                    let timestamp = if achieved {
                        need(src, 8)?;
                        Some(src.get_i64())
                    } else {
                        None
                    };
                    criteria.push((crit_id, timestamp));
                }
                progress.push(AdvancementProgress { id, criteria });
            }

            Ok(Self {
                reset,
                added,
                removed,
                progress,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundPlayerCombatEnter;

    impl PacketId for ClientboundPlayerCombatEnter {
        fn packet_id(_ver: u32) -> u8 {
            0x2D
        }
    }

    impl Encode for ClientboundPlayerCombatEnter {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            VarInt(0).encode(dst) // action = 0: enter combat
        }
    }

    impl Decode for ClientboundPlayerCombatEnter {
        fn decode(_src: &mut Bytes) -> Result<Self, ProtocolError> {
            // action VarInt already consumed by dispatcher
            Ok(Self)
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundPlayerCombatEnd {
        pub duration: VarInt, // ticks in combat
        pub entity_id: i32,   // opponent entity id
    }

    impl PacketId for ClientboundPlayerCombatEnd {
        fn packet_id(_ver: u32) -> u8 {
            0x2D
        }
    }

    impl Encode for ClientboundPlayerCombatEnd {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            VarInt(1).encode(dst)?; // action = 1: end combat
            self.duration.encode(dst)?;
            dst.put_i32(self.entity_id);
            Ok(())
        }
    }

    impl Decode for ClientboundPlayerCombatEnd {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let duration = VarInt::decode(src)?;
            need(src, 4)?;
            let entity_id = src.get_i32();
            Ok(Self {
                duration,
                entity_id,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundPlayerCombatKill {
        pub player_id: VarInt, // entity id of the player that died
        pub entity_id: i32,    // killer entity id (-1 if none)
        pub message: String,   // death message JSON
    }

    impl PacketId for ClientboundPlayerCombatKill {
        fn packet_id(_ver: u32) -> u8 {
            0x2D
        }
    }

    impl Encode for ClientboundPlayerCombatKill {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            VarInt(2).encode(dst)?; // action = 2: entity dead
            self.player_id.encode(dst)?;
            dst.put_i32(self.entity_id);
            encode_str(&self.message, dst)?;
            Ok(())
        }
    }

    impl Decode for ClientboundPlayerCombatKill {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let player_id = VarInt::decode(src)?;
            need(src, 4)?;
            let entity_id = src.get_i32();
            let message = decode_str(src, "ClientboundPlayerCombatKill message")?;
            Ok(Self {
                player_id,
                entity_id,
                message,
            })
        }
    }

    // ── World border packets (0x38) ───────────────────────────────────────────────
    // In 1.12.2 all border variants share packet id 0x38,
    // distinguished by a leading VarInt action field.

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundSetBorderSize {
        pub diameter: f64,
    }

    impl PacketId for ClientboundSetBorderSize {
        fn packet_id(_ver: u32) -> u8 {
            0x38
        }
    }

    impl Encode for ClientboundSetBorderSize {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            VarInt(0).encode(dst)?; // action = 0: set size
            dst.put_f64(self.diameter);
            Ok(())
        }
    }

    impl Decode for ClientboundSetBorderSize {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 8)?;
            Ok(Self {
                diameter: src.get_f64(),
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundSetBorderLerpSize {
        pub old_diameter: f64,
        pub new_diameter: f64,
        pub speed: VarInt, // ticks until new diameter is reached
    }

    impl PacketId for ClientboundSetBorderLerpSize {
        fn packet_id(_ver: u32) -> u8 {
            0x38
        }
    }

    impl Encode for ClientboundSetBorderLerpSize {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            VarInt(1).encode(dst)?; // action = 1: lerp size
            dst.put_f64(self.old_diameter);
            dst.put_f64(self.new_diameter);
            self.speed.encode(dst)?;
            Ok(())
        }
    }

    impl Decode for ClientboundSetBorderLerpSize {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 8 + 8)?;
            let old_diameter = src.get_f64();
            let new_diameter = src.get_f64();
            let speed = VarInt::decode(src)?;
            Ok(Self {
                old_diameter,
                new_diameter,
                speed,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundSetBorderCenter {
        pub new_center_x: f64,
        pub new_center_z: f64,
    }

    impl PacketId for ClientboundSetBorderCenter {
        fn packet_id(_ver: u32) -> u8 {
            0x38
        }
    }

    impl Encode for ClientboundSetBorderCenter {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            VarInt(2).encode(dst)?; // action = 2: set center
            dst.put_f64(self.new_center_x);
            dst.put_f64(self.new_center_z);
            Ok(())
        }
    }

    impl Decode for ClientboundSetBorderCenter {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 8 + 8)?;
            Ok(Self {
                new_center_x: src.get_f64(),
                new_center_z: src.get_f64(),
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundInitializeBorder {
        pub x: f64,
        pub z: f64,
        pub old_diameter: f64,
        pub new_diameter: f64,
        pub speed: VarInt,
        pub portal_teleport_boundary: VarInt,
        pub warning_blocks: VarInt,
        pub warning_time: VarInt,
    }

    impl PacketId for ClientboundInitializeBorder {
        fn packet_id(_ver: u32) -> u8 {
            0x38
        }
    }

    impl Encode for ClientboundInitializeBorder {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            VarInt(3).encode(dst)?; // action = 3: initialize
            dst.put_f64(self.x);
            dst.put_f64(self.z);
            dst.put_f64(self.old_diameter);
            dst.put_f64(self.new_diameter);
            self.speed.encode(dst)?;
            self.portal_teleport_boundary.encode(dst)?;
            self.warning_blocks.encode(dst)?;
            self.warning_time.encode(dst)?;
            Ok(())
        }
    }

    impl Decode for ClientboundInitializeBorder {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 8 + 8 + 8 + 8)?;
            let x = src.get_f64();
            let z = src.get_f64();
            let old_diameter = src.get_f64();
            let new_diameter = src.get_f64();
            let speed = VarInt::decode(src)?;
            let portal_teleport_boundary = VarInt::decode(src)?;
            let warning_blocks = VarInt::decode(src)?;
            let warning_time = VarInt::decode(src)?;
            Ok(Self {
                x,
                z,
                old_diameter,
                new_diameter,
                speed,
                portal_teleport_boundary,
                warning_blocks,
                warning_time,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundSetBorderWarningTime {
        pub warning_time: VarInt, // seconds
    }

    impl PacketId for ClientboundSetBorderWarningTime {
        fn packet_id(_ver: u32) -> u8 {
            0x38
        }
    }

    impl Encode for ClientboundSetBorderWarningTime {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            VarInt(4).encode(dst)?; // action = 4: set warning time
            self.warning_time.encode(dst)
        }
    }

    impl Decode for ClientboundSetBorderWarningTime {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            Ok(Self {
                warning_time: VarInt::decode(src)?,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundSetBorderWarningDistance {
        pub warning_blocks: VarInt,
    }

    impl PacketId for ClientboundSetBorderWarningDistance {
        fn packet_id(_ver: u32) -> u8 {
            0x38
        }
    }

    impl Encode for ClientboundSetBorderWarningDistance {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            VarInt(5).encode(dst)?; // action = 5: set warning distance
            self.warning_blocks.encode(dst)
        }
    }

    impl Decode for ClientboundSetBorderWarningDistance {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            Ok(Self {
                warning_blocks: VarInt::decode(src)?,
            })
        }
    }

    // Note: ClientboundSetBorderWarningDelay is the same as ClientboundSetBorderWarningTime
    // (action 4). They are the same packet, just aliased differently in your exports.
    pub type ClientboundSetBorderWarningDelay = ClientboundSetBorderWarningTime;

    // ── ClientboundHorseScreenOpen (0x1F) ─────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundHorseScreenOpen {
        pub window_id: u8,
        pub slot_count: VarInt,
        pub entity_id: i32,
    }

    impl PacketId for ClientboundHorseScreenOpen {
        fn packet_id(_ver: u32) -> u8 {
            0x1F
        }
    }

    impl Encode for ClientboundHorseScreenOpen {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_u8(self.window_id);
            self.slot_count.encode(dst)?;
            dst.put_i32(self.entity_id);
            Ok(())
        }
    }

    impl Decode for ClientboundHorseScreenOpen {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 1)?;
            let window_id = src.get_u8();
            let slot_count = VarInt::decode(src)?;
            need(src, 4)?;
            let entity_id = src.get_i32();
            Ok(Self {
                window_id,
                slot_count,
                entity_id,
            })
        }
    }

    // ── ClientboundPlaceGhostRecipe (0x31 in 1.12.2) ──────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundPlaceGhostRecipe {
        pub window_id: u8,
        pub recipe: String,
    }

    impl PacketId for ClientboundPlaceGhostRecipe {
        fn packet_id(_ver: u32) -> u8 {
            0x31
        }
    }

    impl Encode for ClientboundPlaceGhostRecipe {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_u8(self.window_id);
            encode_str(&self.recipe, dst)?;
            Ok(())
        }
    }

    impl Decode for ClientboundPlaceGhostRecipe {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 1)?;
            let window_id = src.get_u8();
            let recipe = decode_str(src, "ClientboundPlaceGhostRecipe recipe")?;
            Ok(Self { window_id, recipe })
        }
    }

    // ── ClientboundResetScore (not in 1.12.2 — stub only) ────────────────────────
    // This packet does not exist in 1.12.2. Keep as raw passthrough.
    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundResetScore {
        pub raw: Vec<u8>,
    }
    impl PacketId for ClientboundResetScore {
        fn packet_id(_ver: u32) -> u8 {
            0xFF
        }
    }
    impl Encode for ClientboundResetScore {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_slice(&self.raw);
            Ok(())
        }
    }
    impl Decode for ClientboundResetScore {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let len = src.remaining();
            let mut raw = vec![0u8; len];
            src.copy_to_slice(&mut raw);
            Ok(Self { raw })
        }
    }

    // ── ClientboundStopSound (0x48 in 1.12.2) ────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundStopSound {
        pub flags: u8,
        pub source: Option<VarInt>, // present if flags & 1
        pub sound: Option<String>,  // present if flags & 2
    }

    impl PacketId for ClientboundStopSound {
        fn packet_id(_ver: u32) -> u8 {
            0x48
        }
    }

    impl Encode for ClientboundStopSound {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_u8(self.flags);
            if self.flags & 1 != 0 {
                if let Some(s) = &self.source {
                    s.encode(dst)?;
                }
            }
            if self.flags & 2 != 0 {
                if let Some(s) = &self.sound {
                    encode_str(s, dst)?;
                }
            }
            Ok(())
        }
    }

    impl Decode for ClientboundStopSound {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 1)?;
            let flags = src.get_u8();
            let source = if flags & 1 != 0 {
                Some(VarInt::decode(src)?)
            } else {
                None
            };
            let sound = if flags & 2 != 0 {
                Some(decode_str(src, "ClientboundStopSound sound")?)
            } else {
                None
            };
            Ok(Self {
                flags,
                source,
                sound,
            })
        }
    }

    // ── ServerboundClickWindow (0x07) ─────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ServerboundClickWindow {
        pub window_id: u8,
        pub slot: i16,
        pub button: i8,
        pub action_number: i16,
        pub mode: VarInt,
        pub clicked_item: LegacySlot,
    }

    impl PacketId for ServerboundClickWindow {
        fn packet_id(_ver: u32) -> u8 {
            0x07
        }
    }

    impl Encode for ServerboundClickWindow {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_u8(self.window_id);
            dst.put_i16(self.slot);
            dst.put_i8(self.button);
            dst.put_i16(self.action_number);
            self.mode.encode(dst)?;
            self.clicked_item.encode(dst)?;
            Ok(())
        }
    }

    impl Decode for ServerboundClickWindow {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 1 + 2 + 1 + 2)?;
            let window_id = src.get_u8();
            let slot = src.get_i16();
            let button = src.get_i8();
            let action_number = src.get_i16();
            let mode = VarInt::decode(src)?;
            let clicked_item = LegacySlot::decode(src)?;
            Ok(Self {
                window_id,
                slot,
                button,
                action_number,
                mode,
                clicked_item,
            })
        }
    }

    // ── ServerboundRecipeBookChangeSettings (0x16) ────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ServerboundRecipeBookChangeSettings {
        pub book_id: VarInt, // 0 = crafting, 1 = furnace
        pub open: bool,
        pub filter: bool,
    }

    impl PacketId for ServerboundRecipeBookChangeSettings {
        fn packet_id(_ver: u32) -> u8 {
            0x16
        }
    }

    impl Encode for ServerboundRecipeBookChangeSettings {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.book_id.encode(dst)?;
            dst.put_u8(self.open as u8);
            dst.put_u8(self.filter as u8);
            Ok(())
        }
    }

    impl Decode for ServerboundRecipeBookChangeSettings {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let book_id = VarInt::decode(src)?;
            need(src, 1 + 1)?;
            let open = src.get_u8() != 0;
            let filter = src.get_u8() != 0;
            Ok(Self {
                book_id,
                open,
                filter,
            })
        }
    }

    // ── ServerboundPickItem (0x15 in 1.12.2) ──────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ServerboundPickItem {
        pub slot_to_use: VarInt,
    }

    impl PacketId for ServerboundPickItem {
        fn packet_id(_ver: u32) -> u8 {
            0x15
        }
    }

    impl Encode for ServerboundPickItem {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.slot_to_use.encode(dst)
        }
    }

    impl Decode for ServerboundPickItem {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            Ok(Self {
                slot_to_use: VarInt::decode(src)?,
            })
        }
    }

    // ── ServerboundPlaceRecipe (0x17 in 1.12.2) ───────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ServerboundPlaceRecipe {
        pub window_id: i8,
        pub recipe: String,
        pub make_all: bool,
    }

    impl PacketId for ServerboundPlaceRecipe {
        fn packet_id(_ver: u32) -> u8 {
            0x17
        }
    }

    impl Encode for ServerboundPlaceRecipe {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_i8(self.window_id);
            encode_str(&self.recipe, dst)?;
            dst.put_u8(self.make_all as u8);
            Ok(())
        }
    }

    impl Decode for ServerboundPlaceRecipe {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 1)?;
            let window_id = src.get_i8();
            let recipe = decode_str(src, "ServerboundPlaceRecipe recipe")?;
            need(src, 1)?;
            let make_all = src.get_u8() != 0;
            Ok(Self {
                window_id,
                recipe,
                make_all,
            })
        }
    }

    // ── ServerboundSetBeaconEffect (0x19 in 1.12.2) ───────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ServerboundSetBeaconEffect {
        pub primary_effect: VarInt, // potion effect id
        pub secondary_effect: VarInt,
    }

    impl PacketId for ServerboundSetBeaconEffect {
        fn packet_id(_ver: u32) -> u8 {
            0x19
        }
    }

    impl Encode for ServerboundSetBeaconEffect {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.primary_effect.encode(dst)?;
            self.secondary_effect.encode(dst)
        }
    }

    impl Decode for ServerboundSetBeaconEffect {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            Ok(Self {
                primary_effect: VarInt::decode(src)?,
                secondary_effect: VarInt::decode(src)?,
            })
        }
    }

    // ── ServerboundSetStructureBlock (0x1E in 1.12.2) ─────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ServerboundSetStructureBlock {
        pub location: u64,
        pub action: VarInt, // 0=update data, 1=save, 2=load, 3=detect size
        pub mode: VarInt,   // 0=save, 1=load, 2=corner, 3=data
        pub name: String,
        pub offset_x: i8,
        pub offset_y: i8,
        pub offset_z: i8,
        pub size_x: i8,
        pub size_y: i8,
        pub size_z: i8,
        pub mirror: VarInt,   // 0=none, 1=left-right, 2=front-back
        pub rotation: VarInt, // 0=none, 1=cw90, 2=cw180, 3=ccw90
        pub metadata: String,
        pub integrity: f32,
        pub seed: VarInt,
        pub flags: u8, // bit 0=ignore entities, bit 1=show air, bit 2=show bounding box
    }

    impl PacketId for ServerboundSetStructureBlock {
        fn packet_id(_ver: u32) -> u8 {
            0x1E
        }
    }

    impl Encode for ServerboundSetStructureBlock {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_u64(self.location);
            self.action.encode(dst)?;
            self.mode.encode(dst)?;
            encode_str(&self.name, dst)?;
            dst.put_i8(self.offset_x);
            dst.put_i8(self.offset_y);
            dst.put_i8(self.offset_z);
            dst.put_i8(self.size_x);
            dst.put_i8(self.size_y);
            dst.put_i8(self.size_z);
            self.mirror.encode(dst)?;
            self.rotation.encode(dst)?;
            encode_str(&self.metadata, dst)?;
            dst.put_f32(self.integrity);
            self.seed.encode(dst)?;
            dst.put_u8(self.flags);
            Ok(())
        }
    }

    impl Decode for ServerboundSetStructureBlock {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 8)?;
            let location = src.get_u64();
            let action = VarInt::decode(src)?;
            let mode = VarInt::decode(src)?;
            let name = decode_str(src, "ServerboundSetStructureBlock name")?;
            need(src, 6)?;
            let offset_x = src.get_i8();
            let offset_y = src.get_i8();
            let offset_z = src.get_i8();
            let size_x = src.get_i8();
            let size_y = src.get_i8();
            let size_z = src.get_i8();
            let mirror = VarInt::decode(src)?;
            let rotation = VarInt::decode(src)?;
            let metadata = decode_str(src, "ServerboundSetStructureBlock metadata")?;
            need(src, 4)?;
            let integrity = src.get_f32();
            let seed = VarInt::decode(src)?;
            need(src, 1)?;
            let flags = src.get_u8();
            Ok(Self {
                location,
                action,
                mode,
                name,
                offset_x,
                offset_y,
                offset_z,
                size_x,
                size_y,
                size_z,
                mirror,
                rotation,
                metadata,
                integrity,
                seed,
                flags,
            })
        }
    }

    // ── ServerboundSelectTrade (0x1F in 1.12.2) ───────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ServerboundSelectTrade {
        pub selected_slot: VarInt,
    }

    impl PacketId for ServerboundSelectTrade {
        fn packet_id(_ver: u32) -> u8 {
            0x1F
        }
    }

    impl Encode for ServerboundSelectTrade {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.selected_slot.encode(dst)
        }
    }

    impl Decode for ServerboundSelectTrade {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            Ok(Self {
                selected_slot: VarInt::decode(src)?,
            })
        }
    }

    // ── ServerboundUpdateCommandBlock (0x21 in 1.12.2) ───────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ServerboundUpdateCommandBlock {
        pub location: u64,
        pub command: String,
        pub mode: VarInt, // 0=sequence, 1=auto, 2=redstone
        pub flags: u8,    // bit 0=track output, bit 1=conditional, bit 2=automatic
    }

    impl PacketId for ServerboundUpdateCommandBlock {
        fn packet_id(_ver: u32) -> u8 {
            0x21
        }
    }

    impl Encode for ServerboundUpdateCommandBlock {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_u64(self.location);
            encode_str(&self.command, dst)?;
            self.mode.encode(dst)?;
            dst.put_u8(self.flags);
            Ok(())
        }
    }

    impl Decode for ServerboundUpdateCommandBlock {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 8)?;
            let location = src.get_u64();
            let command = decode_str(src, "ServerboundUpdateCommandBlock command")?;
            let mode = VarInt::decode(src)?;
            need(src, 1)?;
            let flags = src.get_u8();
            Ok(Self {
                location,
                command,
                mode,
                flags,
            })
        }
    }

    // ── ServerboundUpdateCommandBlockMinecart (0x22 in 1.12.2) ───────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ServerboundUpdateCommandBlockMinecart {
        pub entity_id: VarInt,
        pub command: String,
        pub track_output: bool,
    }

    impl PacketId for ServerboundUpdateCommandBlockMinecart {
        fn packet_id(_ver: u32) -> u8 {
            0x22
        }
    }

    impl Encode for ServerboundUpdateCommandBlockMinecart {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.entity_id.encode(dst)?;
            encode_str(&self.command, dst)?;
            dst.put_u8(self.track_output as u8);
            Ok(())
        }
    }

    impl Decode for ServerboundUpdateCommandBlockMinecart {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let entity_id = VarInt::decode(src)?;
            let command = decode_str(src, "ServerboundUpdateCommandBlockMinecart command")?;
            need(src, 1)?;
            let track_output = src.get_u8() != 0;
            Ok(Self {
                entity_id,
                command,
                track_output,
            })
        }
    }
}
