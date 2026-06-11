use bytes::{Buf, BufMut, Bytes, BytesMut};

use crate::codec::{Decode, Encode, PacketId};
use crate::error::ProtocolError;
use crate::types::VarInt;

pub use packets::{
    ClientboundChatMessage, ClientboundDisconnect, ClientboundJoinGame, ClientboundKeepAlive,
    ClientboundPlayerAbilities, ClientboundPlayerPosition, ClientboundPluginMessage,
    ClientboundRespawn, ClientboundSetHeldItem, ClientboundSound,
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
        fn packet_id(ver: u32) -> u8 {
            crate::registry::cb_play(ver, "ClientboundJoinGame")
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

    // ── Respawn (0x07) ────────────────────────────────────────────────────────

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

    // ── PlayerPosition (0x08) ─────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundPlayerPosition {
        pub x: f64,
        pub y: f64,
        pub z: f64,
        pub yaw: f32,
        pub pitch: f32,
        pub flags: u8,
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
            Ok(())
        }
    }

    impl Decode for ClientboundPlayerPosition {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 8 + 8 + 8 + 4 + 4 + 1)?;
            Ok(Self {
                x: src.get_f64(),
                y: src.get_f64(),
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
        fn packet_id(ver: u32) -> u8 {
            crate::registry::cb_play(ver, "ClientboundKeepAlive")
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

    // ── Chat (0x02 / 0x01) ────────────────────────────────────────────────────

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
            Ok(Self {
                json_message,
                position: src.get_u8(),
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
        fn packet_id(ver: u32) -> u8 {
            crate::registry::cb_play(ver, "ClientboundPluginMessage")
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

    // ── Disconnect (0x40) ─────────────────────────────────────────────────────

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
        fn packet_id(ver: u32) -> u8 {
            crate::registry::cb_play(ver, "ClientboundSetHeldItem")
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

    // ── Movement (0x03–0x06) ──────────────────────────────────────────────────

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
        fn packet_id(ver: u32) -> u8 {
            crate::registry::cb_play(ver, "ClientboundSound")
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

    // ── Player Abilities (0x39) ───────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundPlayerAbilities {
        pub flags: u8,
        pub flying_speed: f32,
        pub field_of_view_modifier: f32,
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

    // ── ContainerSetContent (0x30) ────────────────────────────────────────────

    // ── ContainerSetSlot (0x2F) ───────────────────────────────────────────────
}
