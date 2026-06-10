use crate::codec::{Decode, Encode, PacketId};
use crate::error::ProtocolError;
use crate::types::VarInt;
use bytes::{Buf, BufMut, Bytes, BytesMut};

pub use packets::{
    BossBarAction, ClientboundBossBar, ClientboundChatMessage, ClientboundDisconnect,
    ClientboundJoinGame, ClientboundKeepAlive, ClientboundPlayerAbilities,
    ClientboundPlayerPosition, ClientboundRespawn, ClientboundSetCarriedItem, ClientboundSound,
};

mod packets {
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
        fn packet_id(ver: u32) -> u8 {
            crate::registry::cb_play(ver, "ClientboundJoinGame")
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
        fn packet_id(ver: u32) -> u8 {
            crate::registry::cb_play(ver, "ClientboundRespawn")
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
        fn packet_id(ver: u32) -> u8 {
            crate::registry::cb_play(ver, "ClientboundPlayerPosition")
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

    // ── KeepAlive (0x1F / 0x0B) ──────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundKeepAlive {
        pub keep_alive_id: i64,
    }

    impl PacketId for ClientboundKeepAlive {
        fn packet_id(ver: u32) -> u8 {
            crate::registry::cb_play(ver, "ClientboundKeepAlive")
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

    // ── Chat (0x0F / 0x02) ────────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundChatMessage {
        pub json_message: String,
        pub position: u8,
    }

    impl PacketId for ClientboundChatMessage {
        fn packet_id(ver: u32) -> u8 {
            crate::registry::cb_play(ver, "ClientboundChatMessage")
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

    // ── Movement ──────────────────────────────────────────────────────────────

    // ── Interact (0x0A) ───────────────────────────────────────────────────────

    // ── PluginMessage (0x18 / 0x09) ───────────────────────────────────────────

    // ── Disconnect (0x1A) ─────────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundDisconnect {
        pub reason: String,
    }

    impl PacketId for ClientboundDisconnect {
        fn packet_id(ver: u32) -> u8 {
            crate::registry::cb_play(ver, "ClientboundDisconnect")
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
        fn packet_id(ver: u32) -> u8 {
            crate::registry::cb_play(ver, "ClientboundPlayerAbilities")
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

    // ── SetCarriedItem (0x3A) ─────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundSetCarriedItem {
        pub slot: i8,
    }

    impl PacketId for ClientboundSetCarriedItem {
        fn packet_id(ver: u32) -> u8 {
            crate::registry::cb_play(ver, "ClientboundSetCarriedItem")
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
        fn packet_id(ver: u32) -> u8 {
            crate::registry::cb_play(ver, "ClientboundBossBar")
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

    // ── SpawnPlayer (0x05) ────────────────────────────────────────────────────

    // ── EntityAnimation (0x06) ────────────────────────────────────────────────

    // ── AwardStats (0x07) ─────────────────────────────────────────────────────

    // ── BlockDestroyStage (0x08) ──────────────────────────────────────────────

    // ── BlockEntityData (0x09) ────────────────────────────────────────────────

    // ── BlockAction (0x0A) ────────────────────────────────────────────────────

    // ── BlockUpdate (0x0B) ────────────────────────────────────────────────────

    // ── ChangeDifficulty (0x0D) ───────────────────────────────────────────────

    // ── CommandSuggestions (0x0E) ─────────────────────────────────────────────

    // ── SectionBlocksUpdate (0x10) ────────────────────────────────────────────

    // ── ContainerClose (0x12) ─────────────────────────────────────────────────

    // ── OpenScreen (0x13) ─────────────────────────────────────────────────────

    // ── ContainerSetContent (0x14) ────────────────────────────────────────────

    // ── ContainerSetProperty (0x15) ───────────────────────────────────────────

    // ── ContainerSetSlot (0x16) ───────────────────────────────────────────────

    // ── Cooldown (0x17) ───────────────────────────────────────────────────────

    // ── EntityEvent (0x1B) ────────────────────────────────────────────────────

    // ── Explosion (0x1C) ──────────────────────────────────────────────────────

    // ── ForgetLevelChunk (0x1D) ───────────────────────────────────────────────

    // ── GameEvent (0x1E) ──────────────────────────────────────────────────────

    // (LevelChunkWithLight / SPacketChunkData no longer required by the proxy.)

    // ── LevelEvent (0x21) ─────────────────────────────────────────────────────

    // ── LevelParticles (0x22) ─────────────────────────────────────────────────

    // ── MapItemData (0x24) ────────────────────────────────────────────────────

    // ── MoveEntityPos (0x25) ──────────────────────────────────────────────────

    // ── MoveEntityPosRot (0x26) ───────────────────────────────────────────────

    // ── MoveEntityRot (0x27) ──────────────────────────────────────────────────

    // ── MoveVehicle (0x29) ────────────────────────────────────────────────────

    // ── OpenSignEditor (0x2A) ─────────────────────────────────────────────────

    // ── PlayerInfoUpdate (0x2E) — opaque raw ─────────────────────────────────

    // ── Recipes (0x31) — opaque raw ───────────────────────────────────────────

    // ── RemoveEntities (0x32) ─────────────────────────────────────────────────

    // ── RemoveEntityEffect (0x33) ─────────────────────────────────────────────

    // ── ResourcePackPush (0x34) ───────────────────────────────────────────────

    // ── RotateHead (0x36) ─────────────────────────────────────────────────────

    // ── SelectAdvancementsTab (0x37) ──────────────────────────────────────────

    // ── SetCamera (0x39) ──────────────────────────────────────────────────────

    // ── SetHeldItem (0x3A) ────────────────────────────────────────────────────

    // ── SetEntityLink (0x3D) ──────────────────────────────────────────────────

    // ── SetEntityMotion (0x3E) ────────────────────────────────────────────────

    // ── SetEquipment (0x3F) ───────────────────────────────────────────────────

    // ── SetExperience (0x40) ──────────────────────────────────────────────────

    // ── SetHealth (0x41) ──────────────────────────────────────────────────────

    // ── SetScoreboardObjective (0x42) ─────────────────────────────────────────

    // ── SetScoreboardScore (0x45) ─────────────────────────────────────────────

    // ── SetTime (0x47) ────────────────────────────────────────────────────────

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
        fn packet_id(ver: u32) -> u8 {
            crate::registry::cb_play(ver, "ClientboundSound")
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

    // ── TakeItemEntity (0x4B) ─────────────────────────────────────────────────

    // ── TeleportEntity (0x4C) ─────────────────────────────────────────────────

    // ── UpdateEffects (0x4F) ──────────────────────────────────────────────────

    // ── SpawnGlobalEntity (0x02) ──────────────────────────────────────────────

    // ── ServerboundCustomPayload (0x0A) ───────────────────────────────────────

    // ── Serverbound packets ───────────────────────────────────────────────────

    // ── Opaque raw stubs ──────────────────────────────────────────────────────

    // ── World border packets (0x38) ───────────────────────────────────────────────
    // In 1.12.2 all border variants share packet id 0x38,
    // distinguished by a leading VarInt action field.

    // (border-warning typedef removed — not used by the proxy.)

    // ── ClientboundHorseScreenOpen (0x1F) ─────────────────────────────────────────

    // ── ClientboundPlaceGhostRecipe (0x31 in 1.12.2) ──────────────────────────────

    // ── ClientboundResetScore (not in 1.12.2 — stub only) ────────────────────────
    // This packet does not exist in 1.12.2. Keep as raw passthrough.

    // ── ClientboundStopSound (0x48 in 1.12.2) ────────────────────────────────────

    // ── ServerboundClickWindow (0x07) ─────────────────────────────────────────────

    // ── ServerboundRecipeBookChangeSettings (0x16) ────────────────────────────────

    // ── ServerboundPickItem (0x15 in 1.12.2) ──────────────────────────────────────

    // ── ServerboundPlaceRecipe (0x17 in 1.12.2) ───────────────────────────────────

    // ── ServerboundSetBeaconEffect (0x19 in 1.12.2) ───────────────────────────────

    // ── ServerboundSetStructureBlock (0x1E in 1.12.2) ─────────────────────────────

    // ── ServerboundSelectTrade (0x1F in 1.12.2) ───────────────────────────────────

    // ── ServerboundUpdateCommandBlock (0x21 in 1.12.2) ───────────────────────────

    // ── ServerboundUpdateCommandBlockMinecart (0x22 in 1.12.2) ───────────────────
}
