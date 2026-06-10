use bytes::{Buf, Bytes, BytesMut};

use crate::codec::{Decode, Encode, PacketId};
use crate::error::ProtocolError;
use crate::types::VarInt;

pub use packets::{
    ClientboundDisconnect, ClientboundKeepAlive, ClientboundLogin, ClientboundPlayerAbilities, ClientboundPlayerPosition, ClientboundRespawn, ClientboundSetCarriedItem, ClientboundSystemChat,
};

mod packets {
    use super::*;

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
            self.reason.encode(dst)
        }
    }
    impl Decode for ClientboundDisconnect {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            Ok(Self {
                reason: String::decode(src)?,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundKeepAlive {
        pub id: i64,
    }
    impl PacketId for ClientboundKeepAlive {
        fn packet_id(ver: u32) -> u8 {
            crate::registry::cb_play(ver, "ClientboundKeepAlive")
        }
    }
    impl Encode for ClientboundKeepAlive {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.id.encode(dst)
        }
    }
    impl Decode for ClientboundKeepAlive {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            Ok(Self {
                id: i64::decode(src)?,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundLogin {
        pub entity_id: i32,
        pub is_hardcore: bool,
        pub game_mode: u8,
        pub previous_game_mode: i8,
        pub dimensions: Vec<String>,

        pub registry_codec: Vec<u8>,
        pub dimension_type: String,
        pub dimension_name: String,
        pub hashed_seed: i64,
        pub max_players: VarInt,
        pub chunk_radius: VarInt,
        pub simulation_distance: VarInt,
        pub reduced_debug_info: bool,
        pub enable_respawn_screen: bool,
        pub is_debug: bool,
        pub is_flat: bool,
        pub death_location: Option<(String, i64)>,
    }
    impl PacketId for ClientboundLogin {
        fn packet_id(ver: u32) -> u8 {
            crate::registry::cb_play(ver, "ClientboundLogin")
        }
    }
    impl Encode for ClientboundLogin {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.entity_id.encode(dst)?;
            self.is_hardcore.encode(dst)?;
            self.game_mode.encode(dst)?;
            self.previous_game_mode.encode(dst)?;
            self.dimensions.encode(dst)?;
            dst.extend_from_slice(&self.registry_codec);
            self.dimension_type.encode(dst)?;
            self.dimension_name.encode(dst)?;
            self.hashed_seed.encode(dst)?;
            self.max_players.encode(dst)?;
            self.chunk_radius.encode(dst)?;
            self.simulation_distance.encode(dst)?;
            self.reduced_debug_info.encode(dst)?;
            self.enable_respawn_screen.encode(dst)?;
            self.is_debug.encode(dst)?;
            self.is_flat.encode(dst)?;
            match &self.death_location {
                Some((dim, pos)) => {
                    true.encode(dst)?;
                    dim.encode(dst)?;
                    pos.encode(dst)?;
                },
                None => false.encode(dst)?,
            }
            Ok(())
        }
    }
    impl Decode for ClientboundLogin {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let entity_id = i32::decode(src)?;
            let is_hardcore = bool::decode(src)?;
            let game_mode = u8::decode(src)?;
            let previous_game_mode = i8::decode(src)?;
            let dimensions = Vec::<String>::decode(src)?;
            let mut codec_view = src.clone();
            let codec_len = crate::types::nbt::skip(&mut codec_view)?;
            let registry_codec = src.copy_to_bytes(codec_len).to_vec();
            let dimension_type = String::decode(src)?;
            let dimension_name = String::decode(src)?;
            let hashed_seed = i64::decode(src)?;
            let max_players = VarInt::decode(src)?;
            let chunk_radius = VarInt::decode(src)?;
            let simulation_distance = VarInt::decode(src)?;
            let reduced_debug_info = bool::decode(src)?;
            let enable_respawn_screen = bool::decode(src)?;
            let is_debug = bool::decode(src)?;
            let is_flat = bool::decode(src)?;
            let has_death = bool::decode(src)?;
            let death_location = if has_death {
                Some((String::decode(src)?, i64::decode(src)?))
            } else {
                None
            };
            Ok(Self {
                entity_id,
                is_hardcore,
                game_mode,
                previous_game_mode,
                dimensions,
                registry_codec,
                dimension_type,
                dimension_name,
                hashed_seed,
                max_players,
                chunk_radius,
                simulation_distance,
                reduced_debug_info,
                enable_respawn_screen,
                is_debug,
                is_flat,
                death_location,
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
            self.x.encode(dst)?;
            self.y.encode(dst)?;
            self.z.encode(dst)?;
            self.yaw.encode(dst)?;
            self.pitch.encode(dst)?;
            self.flags.encode(dst)?;
            self.teleport_id.encode(dst)
        }
    }
    impl Decode for ClientboundPlayerPosition {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            Ok(Self {
                x: f64::decode(src)?,
                y: f64::decode(src)?,
                z: f64::decode(src)?,
                yaw: f32::decode(src)?,
                pitch: f32::decode(src)?,
                flags: u8::decode(src)?,
                teleport_id: VarInt::decode(src)?,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundRespawn {
        pub dimension_type: String,
        pub dimension_name: String,
        pub hashed_seed: i64,
        pub game_mode: u8,
        pub previous_game_mode: i8,
        pub is_debug: bool,
        pub is_flat: bool,
        pub data_kept: u8,
        pub death_location: Option<(String, i64)>,
    }
    impl PacketId for ClientboundRespawn {
        fn packet_id(ver: u32) -> u8 {
            crate::registry::cb_play(ver, "ClientboundRespawn")
        }
    }
    impl Encode for ClientboundRespawn {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.dimension_type.encode(dst)?;
            self.dimension_name.encode(dst)?;
            self.hashed_seed.encode(dst)?;
            self.game_mode.encode(dst)?;
            self.previous_game_mode.encode(dst)?;
            self.is_debug.encode(dst)?;
            self.is_flat.encode(dst)?;
            self.data_kept.encode(dst)?;
            match &self.death_location {
                Some((dim, pos)) => {
                    true.encode(dst)?;
                    dim.encode(dst)?;
                    pos.encode(dst)?;
                },
                None => false.encode(dst)?,
            }
            Ok(())
        }
    }
    impl Decode for ClientboundRespawn {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let dimension_type = String::decode(src)?;
            let dimension_name = String::decode(src)?;
            let hashed_seed = i64::decode(src)?;
            let game_mode = u8::decode(src)?;
            let previous_game_mode = i8::decode(src)?;
            let is_debug = bool::decode(src)?;
            let is_flat = bool::decode(src)?;
            let data_kept = u8::decode(src)?;
            let has_death = bool::decode(src)?;
            let death_location = if has_death {
                Some((String::decode(src)?, i64::decode(src)?))
            } else {
                None
            };
            Ok(Self {
                dimension_type,
                dimension_name,
                hashed_seed,
                game_mode,
                previous_game_mode,
                is_debug,
                is_flat,
                data_kept,
                death_location,
            })
        }
    }

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
            self.flags.encode(dst)?;
            self.flying_speed.encode(dst)?;
            self.walking_speed.encode(dst)
        }
    }
    impl Decode for ClientboundPlayerAbilities {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            Ok(Self {
                flags: u8::decode(src)?,
                flying_speed: f32::decode(src)?,
                walking_speed: f32::decode(src)?,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundSystemChat {
        pub content: String,
        pub overlay: bool,
    }
    impl PacketId for ClientboundSystemChat {
        fn packet_id(ver: u32) -> u8 {
            crate::registry::cb_play(ver, "ClientboundSystemChat")
        }
    }
    impl Encode for ClientboundSystemChat {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            self.content.encode(dst)?;
            self.overlay.encode(dst)
        }
    }
    impl Decode for ClientboundSystemChat {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            Ok(Self {
                content: String::decode(src)?,
                overlay: bool::decode(src)?,
            })
        }
    }

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
            self.slot.encode(dst)
        }
    }
    impl Decode for ClientboundSetCarriedItem {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            Ok(Self {
                slot: i8::decode(src)?,
            })
        }
    }

}