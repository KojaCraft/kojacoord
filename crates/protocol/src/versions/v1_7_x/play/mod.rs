use bytes::{Buf, BufMut, Bytes, BytesMut};

use crate::codec::{Decode, Encode, PacketId};
use crate::error::ProtocolError;
use crate::types::VarInt;

pub use packets::{
    ClientboundChatMessage, ClientboundDisconnect, ClientboundJoinGame, ClientboundKeepAlive,
    ClientboundPlayerAbilities, ClientboundPlayerPosition,
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

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundKeepAlive {
        // 1.7.10 wire encodes this as i32, not VarInt — VarInt was
        // introduced for keepalives in 1.8.
        pub keep_alive_id: i32,
    }

    impl PacketId for ClientboundKeepAlive {
        fn packet_id(ver: u32) -> u8 {
            crate::registry::cb_play(ver, "ClientboundKeepAlive")
        }
    }

    impl Encode for ClientboundKeepAlive {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            dst.put_i32(self.keep_alive_id);
            Ok(())
        }
    }

    impl Decode for ClientboundKeepAlive {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 4)?;
            Ok(Self {
                keep_alive_id: src.get_i32(),
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundJoinGame {
        pub entity_id: i32,
        pub game_mode: u8,
        pub dimension: i8,
        pub difficulty: u8,
        pub max_players: u8,
        pub level_type: String,
        // 1.7 has no reduced_debug_info field; that was added in 1.8.
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
            Ok(Self {
                entity_id,
                game_mode,
                dimension,
                difficulty,
                max_players,
                level_type,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundChatMessage {
        pub json_message: String,
        // 1.7.x has no position byte; that was added in 1.8.
    }

    impl PacketId for ClientboundChatMessage {
        fn packet_id(ver: u32) -> u8 {
            crate::registry::cb_play(ver, "ClientboundChatMessage")
        }
    }

    impl Encode for ClientboundChatMessage {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            encode_str(&self.json_message, dst)
        }
    }

    impl Decode for ClientboundChatMessage {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let json_message = decode_str(src, "ClientboundChatMessage json_message")?;
            Ok(Self { json_message })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundPlayerPosition {
        // 1.7 clientbound position: x/y/z/yaw/pitch/onGround.
        // No flags bitfield (added in 1.8), no head_y (1.7 transmits feet y).
        pub x: f64,
        pub y: f64,
        pub z: f64,
        pub yaw: f32,
        pub pitch: f32,
        pub on_ground: bool,
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
            dst.put_u8(self.on_ground as u8);
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
                on_ground: src.get_u8() != 0,
            })
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

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundPlayerAbilities {
        // 1.7 wire: flags (i8), flyingSpeed (f32), walkingSpeed (f32).
        pub flags: i8,
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
            dst.put_i8(self.flags);
            dst.put_f32(self.flying_speed);
            dst.put_f32(self.walking_speed);
            Ok(())
        }
    }

    impl Decode for ClientboundPlayerAbilities {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            need(src, 1 + 4 + 4)?;
            Ok(Self {
                flags: src.get_i8(),
                flying_speed: src.get_f32(),
                walking_speed: src.get_f32(),
            })
        }
    }
}
