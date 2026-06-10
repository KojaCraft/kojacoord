use bytes::{Buf, BufMut, Bytes, BytesMut};

use crate::codec::{Decode, Encode, PacketId};
use crate::error::ProtocolError;
use crate::types::VarInt;

pub use packets::{
    ClientboundChatMessage, ClientboundDisconnect, ClientboundHeldItemChange, ClientboundJoinGame, ClientboundKeepAlive, ClientboundNamedSoundEffect, ClientboundPlayerAbilities, ClientboundPlayerPosition, ClientboundRespawn,
};

fn need(src: &Bytes, n: usize) -> Result<(), ProtocolError> {
    if src.remaining() < n {
        return Err(ProtocolError::Io(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "Not enough bytes",
        )));
    }
    Ok(())
}

fn encode_string(s: &str, dst: &mut BytesMut) -> Result<(), ProtocolError> {
    let bytes = s.as_bytes();
    VarInt(bytes.len() as i32).encode(dst)?;
    dst.put_slice(bytes);
    Ok(())
}

fn decode_string(src: &mut Bytes) -> Result<String, ProtocolError> {
    let len = VarInt::decode(src)?.0 as usize;
    if src.remaining() < len {
        return Err(ProtocolError::Io(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "Missing bytes for string",
        )));
    }
    let mut buf = vec![0u8; len];
    src.copy_to_slice(&mut buf);
    String::from_utf8(buf).map_err(|_| {
        ProtocolError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Invalid UTF-8 in string",
        ))
    })
}

mod packets {
    use super::*;












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
            let flags = src.get_u8();
            let flying_speed = src.get_f32();
            let walking_speed = src.get_f32();
            Ok(Self {
                flags,
                flying_speed,
                walking_speed,
            })
        }
    }




    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundHeldItemChange {
        pub slot: i8,
    }

    impl PacketId for ClientboundHeldItemChange {
        fn packet_id(ver: u32) -> u8 {
            crate::registry::cb_play(ver, "ClientboundHeldItemChange")
        }
    }

    impl Encode for ClientboundHeldItemChange {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_i8(self.slot);
            Ok(())
        }
    }

    impl Decode for ClientboundHeldItemChange {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 1)?;
            let slot = src.get_i8();
            Ok(Self { slot })
        }
    }













    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundNamedSoundEffect {
        pub sound_name: String,
        pub sound_category: VarInt,
        pub effect_position_x: i32,
        pub effect_position_y: i32,
        pub effect_position_z: i32,
        pub volume: f32,
        pub pitch: f32,
    }

    impl PacketId for ClientboundNamedSoundEffect {
        fn packet_id(ver: u32) -> u8 {
            crate::registry::cb_play(ver, "ClientboundNamedSoundEffect")
        }
    }

    impl Encode for ClientboundNamedSoundEffect {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            encode_string(&self.sound_name, dst)?;
            self.sound_category.encode(dst)?;
            dst.put_i32(self.effect_position_x);
            dst.put_i32(self.effect_position_y);
            dst.put_i32(self.effect_position_z);
            dst.put_f32(self.volume);
            dst.put_f32(self.pitch);
            Ok(())
        }
    }

    impl Decode for ClientboundNamedSoundEffect {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let sound_name = decode_string(src)?;
            let sound_category = VarInt::decode(src)?;
            if src.remaining() < 20 {
                return Err(ProtocolError::Io(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "Missing bytes for ClientboundNamedSoundEffect position/volume/pitch",
                )));
            }
            let effect_position_x = src.get_i32();
            let effect_position_y = src.get_i32();
            let effect_position_z = src.get_i32();
            let volume = src.get_f32();
            let pitch = src.get_f32();
            Ok(Self {
                sound_name,
                sound_category,
                effect_position_x,
                effect_position_y,
                effect_position_z,
                volume,
                pitch,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundJoinGame {
        pub entity_id: i32,
        pub is_hardcore: bool,
        pub game_mode: u8,
        pub previous_game_mode: i8,
        pub world_names: Vec<String>,
        pub dimension: String,
        pub world_name: String,
        pub hashed_seed: i64,
        pub max_players: VarInt,
        pub view_distance: VarInt,
        pub reduced_debug_info: bool,
        pub enable_respawn_screen: bool,
        pub is_debug: bool,
        pub is_flat: bool,
    }

    impl PacketId for ClientboundJoinGame {
        fn packet_id(ver: u32) -> u8 {
            crate::registry::cb_play(ver, "ClientboundJoinGame")
        }
    }

    impl Encode for ClientboundJoinGame {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_i32(self.entity_id);
            dst.put_u8(self.is_hardcore as u8);
            dst.put_u8(self.game_mode);
            dst.put_i8(self.previous_game_mode);
            VarInt(self.world_names.len() as i32).encode(dst)?;
            for name in &self.world_names {
                encode_string(name, dst)?;
            }
            encode_string(&self.dimension, dst)?;
            encode_string(&self.world_name, dst)?;
            dst.put_i64(self.hashed_seed);
            self.max_players.encode(dst)?;
            self.view_distance.encode(dst)?;
            dst.put_u8(self.reduced_debug_info as u8);
            dst.put_u8(self.enable_respawn_screen as u8);
            dst.put_u8(self.is_debug as u8);
            dst.put_u8(self.is_flat as u8);
            Ok(())
        }
    }

    impl Decode for ClientboundJoinGame {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            if src.remaining() < 4 {
                return Err(ProtocolError::Io(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "Missing bytes for ClientboundJoinGame entity_id",
                )));
            }
            let entity_id = src.get_i32();
            if src.remaining() < 3 {
                return Err(ProtocolError::Io(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "Missing bytes for ClientboundJoinGame flags",
                )));
            }
            let is_hardcore = src.get_u8() != 0;
            let game_mode = src.get_u8();
            let previous_game_mode = src.get_i8();
            let world_count = VarInt::decode(src)?.0 as usize;
            let mut world_names = Vec::with_capacity(world_count);
            for _ in 0..world_count {
                world_names.push(decode_string(src)?);
            }
            let dimension = decode_string(src)?;
            let world_name = decode_string(src)?;
            if src.remaining() < 8 {
                return Err(ProtocolError::Io(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "Missing bytes for ClientboundJoinGame hashed_seed",
                )));
            }
            let hashed_seed = src.get_i64();
            let max_players = VarInt::decode(src)?;
            let view_distance = VarInt::decode(src)?;
            if src.remaining() < 4 {
                return Err(ProtocolError::Io(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "Missing bytes for ClientboundJoinGame boolean flags",
                )));
            }
            let reduced_debug_info = src.get_u8() != 0;
            let enable_respawn_screen = src.get_u8() != 0;
            let is_debug = src.get_u8() != 0;
            let is_flat = src.get_u8() != 0;
            Ok(Self {
                entity_id,
                is_hardcore,
                game_mode,
                previous_game_mode,
                world_names,
                dimension,
                world_name,
                hashed_seed,
                max_players,
                view_distance,
                reduced_debug_info,
                enable_respawn_screen,
                is_debug,
                is_flat,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundRespawn {
        pub dimension: String,
        pub world_name: String,
        pub hashed_seed: i64,
        pub game_mode: u8,
        pub previous_game_mode: i8,
        pub is_debug: bool,
        pub is_flat: bool,
        pub copy_metadata: bool,
    }

    impl PacketId for ClientboundRespawn {
        fn packet_id(ver: u32) -> u8 {
            crate::registry::cb_play(ver, "ClientboundRespawn")
        }
    }

    impl Encode for ClientboundRespawn {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            encode_string(&self.dimension, dst)?;
            encode_string(&self.world_name, dst)?;
            dst.put_i64(self.hashed_seed);
            dst.put_u8(self.game_mode);
            dst.put_i8(self.previous_game_mode);
            dst.put_u8(self.is_debug as u8);
            dst.put_u8(self.is_flat as u8);
            dst.put_u8(self.copy_metadata as u8);
            Ok(())
        }
    }

    impl Decode for ClientboundRespawn {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let dimension = decode_string(src)?;
            let world_name = decode_string(src)?;
            if src.remaining() < 8 {
                return Err(ProtocolError::Io(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "Missing bytes for ClientboundRespawn hashed_seed",
                )));
            }
            let hashed_seed = src.get_i64();
            if src.remaining() < 3 {
                return Err(ProtocolError::Io(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "Missing bytes for ClientboundRespawn game modes",
                )));
            }
            let game_mode = src.get_u8();
            let previous_game_mode = src.get_i8();
            if src.remaining() < 3 {
                return Err(ProtocolError::Io(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "Missing bytes for ClientboundRespawn flags",
                )));
            }
            let is_debug = src.get_u8() != 0;
            let is_flat = src.get_u8() != 0;
            let copy_metadata = src.get_u8() != 0;
            Ok(Self {
                dimension,
                world_name,
                hashed_seed,
                game_mode,
                previous_game_mode,
                is_debug,
                is_flat,
                copy_metadata,
            })
        }
    }

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
            if src.remaining() < 8 * 3 + 4 * 2 + 1 {
                return Err(ProtocolError::Io(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "Missing bytes for ClientboundPlayerPosition",
                )));
            }
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
            if src.remaining() < 8 {
                return Err(ProtocolError::Io(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "Missing bytes for ClientboundKeepAlive",
                )));
            }
            Ok(Self {
                keep_alive_id: src.get_i64(),
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundChatMessage {
        pub json_message: String,
        pub position: u8,
        pub sender: uuid::Uuid,
    }

    impl PacketId for ClientboundChatMessage {
        fn packet_id(ver: u32) -> u8 {
            crate::registry::cb_play(ver, "ClientboundChatMessage")
        }
    }

    impl Encode for ClientboundChatMessage {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            encode_string(&self.json_message, dst)?;
            dst.put_u8(self.position);
            let (hi, lo) = self.sender.as_u64_pair();
            dst.put_i64(hi as i64);
            dst.put_i64(lo as i64);
            Ok(())
        }
    }

    impl Decode for ClientboundChatMessage {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let json_message = decode_string(src)?;
            if src.remaining() < 1 + 16 {
                return Err(ProtocolError::Io(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "Missing bytes for ClientboundChatMessage position/sender",
                )));
            }
            let position = src.get_u8();
            let hi = src.get_i64() as u64;
            let lo = src.get_i64() as u64;
            let sender = uuid::Uuid::from_u64_pair(hi, lo);
            Ok(Self {
                json_message,
                position,
                sender,
            })
        }
    }

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
            encode_string(&self.reason, dst)
        }
    }

    impl Decode for ClientboundDisconnect {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            Ok(Self {
                reason: decode_string(src)?,
            })
        }
    }
}

