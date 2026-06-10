use bytes::{Buf, BufMut, Bytes, BytesMut};

use crate::codec::{Decode, Encode, PacketId};
use crate::error::ProtocolError;

fn need(src: &Bytes, n: usize) -> Result<(), ProtocolError> {
    if src.remaining() < n {
        Err(ProtocolError::UnexpectedEof)
    } else {
        Ok(())
    }
}

/// Pre-netty (1.6.x) strings are UCS-2 with a u16 BE length prefix.
fn encode_legacy_string(s: &str, dst: &mut BytesMut) {
    let units: Vec<u16> = s.encode_utf16().collect();
    dst.put_u16(units.len() as u16);
    for u in units {
        dst.put_u16(u);
    }
}

fn decode_legacy_string(src: &mut Bytes) -> Result<String, ProtocolError> {
    need(src, 2)?;
    let len = src.get_u16() as usize;
    need(src, len * 2)?;
    let mut units = Vec::with_capacity(len);
    for _ in 0..len {
        units.push(src.get_u16());
    }
    String::from_utf16(&units).map_err(|_| {
        ProtocolError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "invalid UCS-2 in pre-netty string",
        ))
    })
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClientboundKeepAlive {
    pub keep_alive_id: i32,
}

impl PacketId for ClientboundKeepAlive {
    fn packet_id(ver: u32) -> u8 {
        crate::registry::cb_play(ver, "ClientboundKeepAlive")
    }
}

impl Encode for ClientboundKeepAlive {
    fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
        dst.extend_from_slice(&self.keep_alive_id.to_be_bytes());
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
pub struct ClientboundChatMessage {
    pub message: String,
}

impl PacketId for ClientboundChatMessage {
    fn packet_id(ver: u32) -> u8 {
        crate::registry::cb_play(ver, "ClientboundChatMessage")
    }
}

impl Encode for ClientboundChatMessage {
    fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
        encode_legacy_string(&self.message, dst);
        Ok(())
    }
}

impl Decode for ClientboundChatMessage {
    fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
        Ok(Self {
            message: decode_legacy_string(src)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClientboundPlayerPosition {
    pub x: f64,
    pub y: f64,
    pub stance: f64,
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
        dst.put_f64(self.stance);
        dst.put_f64(self.z);
        dst.put_f32(self.yaw);
        dst.put_f32(self.pitch);
        dst.put_u8(self.on_ground as u8);
        Ok(())
    }
}

impl Decode for ClientboundPlayerPosition {
    fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
        need(src, 8 + 8 + 8 + 8 + 4 + 4 + 1)?;
        Ok(Self {
            x: src.get_f64(),
            y: src.get_f64(),
            stance: src.get_f64(),
            z: src.get_f64(),
            yaw: src.get_f32(),
            pitch: src.get_f32(),
            on_ground: src.get_u8() != 0,
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
        Ok(Self { slot: src.get_i8() })
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
        encode_legacy_string(&self.reason, dst);
        Ok(())
    }
}

impl Decode for ClientboundDisconnect {
    fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
        Ok(Self {
            reason: decode_legacy_string(src)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClientboundRespawn {
    pub dimension: i8,
    pub difficulty: u8,
    pub gamemode: u8,
    pub world_height: i16,
}

impl PacketId for ClientboundRespawn {
    fn packet_id(ver: u32) -> u8 {
        crate::registry::cb_play(ver, "ClientboundRespawn")
    }
}

impl Encode for ClientboundRespawn {
    fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
        dst.put_i8(self.dimension);
        dst.put_u8(self.difficulty);
        dst.put_u8(self.gamemode);
        dst.put_i16(self.world_height);
        Ok(())
    }
}

impl Decode for ClientboundRespawn {
    fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
        need(src, 1 + 1 + 1 + 2)?;
        Ok(Self {
            dimension: src.get_i8(),
            difficulty: src.get_u8(),
            gamemode: src.get_u8(),
            world_height: src.get_i16(),
        })
    }
}
