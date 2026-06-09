use bytes::{Buf, BufMut, Bytes, BytesMut};

use crate::codec::{Decode, Encode, PacketId};
use crate::error::ProtocolError;

fn decode_legacy_string(src: &mut Bytes) -> Result<String, ProtocolError> {
    if src.len() < 2 {
        return Err(ProtocolError::UnexpectedEof);
    }
    let len = u16::from_be_bytes([src[0], src[1]]) as usize;
    src.advance(2);
    let byte_len = len * 2;
    if src.len() < byte_len {
        return Err(ProtocolError::UnexpectedEof);
    }
    let raw = src.copy_to_bytes(byte_len);
    let chars: Vec<u16> = raw
        .chunks(2)
        .map(|c| u16::from_be_bytes([c[0], c[1]]))
        .collect();
    String::from_utf16(&chars).map_err(|_| ProtocolError::UnexpectedEof)
}

fn encode_legacy_string(s: &str, dst: &mut BytesMut) {
    let utf16: Vec<u16> = s.encode_utf16().collect();
    dst.extend_from_slice(&(utf16.len() as u16).to_be_bytes());
    for ch in &utf16 {
        dst.extend_from_slice(&ch.to_be_bytes());
    }
}

fn need(src: &Bytes, n: usize) -> Result<(), ProtocolError> {
    if src.remaining() < n {
        Err(ProtocolError::UnexpectedEof)
    } else {
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClientboundKeepAlive {
    pub keep_alive_id: i32,
}

impl PacketId for ClientboundKeepAlive {
    fn packet_id(_ver: u32) -> u8 {
        0x00
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
    fn packet_id(_ver: u32) -> u8 {
        0x03
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
    fn packet_id(_ver: u32) -> u8 {
        0x13
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
    fn packet_id(_ver: u32) -> u8 {
        0x09
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
    fn packet_id(_ver: u32) -> u8 {
        0x43
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
    fn packet_id(_ver: u32) -> u8 {
        0xFF
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
pub struct ClientboundSpawnPosition {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl PacketId for ClientboundSpawnPosition {
    fn packet_id(_ver: u32) -> u8 {
        0x05
    }
}

impl Encode for ClientboundSpawnPosition {
    fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
        dst.put_i32(self.x);
        dst.put_i32(self.y);
        dst.put_i32(self.z);
        Ok(())
    }
}

impl Decode for ClientboundSpawnPosition {
    fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
        need(src, 4 + 4 + 4)?;
        Ok(Self {
            x: src.get_i32(),
            y: src.get_i32(),
            z: src.get_i32(),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClientboundUpdateHealth {
    pub health: f32,
    pub food: i16,
    pub food_saturation: f32,
}

impl PacketId for ClientboundUpdateHealth {
    fn packet_id(_ver: u32) -> u8 {
        0x08
    }
}

impl Encode for ClientboundUpdateHealth {
    fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
        dst.put_f32(self.health);
        dst.put_i16(self.food);
        dst.put_f32(self.food_saturation);
        Ok(())
    }
}

impl Decode for ClientboundUpdateHealth {
    fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
        need(src, 4 + 2 + 4)?;
        Ok(Self {
            health: src.get_f32(),
            food: src.get_i16(),
            food_saturation: src.get_f32(),
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
    fn packet_id(_ver: u32) -> u8 {
        0x09
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

#[derive(Debug, Clone, PartialEq)]
pub struct ClientboundEntityEquipment {
    pub entity_id: i32,
    pub slot: i16,
    pub item_id: i16,
}

impl PacketId for ClientboundEntityEquipment {
    fn packet_id(_ver: u32) -> u8 {
        0x1C
    }
}

impl Encode for ClientboundEntityEquipment {
    fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
        dst.put_i32(self.entity_id);
        dst.put_i16(self.slot);
        dst.put_i16(self.item_id);
        Ok(())
    }
}

impl Decode for ClientboundEntityEquipment {
    fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
        need(src, 4 + 2 + 2)?;
        Ok(Self {
            entity_id: src.get_i32(),
            slot: src.get_i16(),
            item_id: src.get_i16(),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClientboundSpawnPlayer {
    pub entity_id: i32,
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
        0x14
    }
}

impl Encode for ClientboundSpawnPlayer {
    fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
        dst.put_i32(self.entity_id);
        encode_legacy_string(&self.username, dst);
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
        let entity_id = src.get_i32();
        let username = decode_legacy_string(src)?;
        need(src, 4 + 4 + 4 + 1 + 1 + 2)?;
        Ok(Self {
            entity_id,
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

#[derive(Debug, Clone, PartialEq)]
pub struct ClientboundChunkData {
    pub chunk_x: i32,
    pub chunk_z: i32,
    pub continuous: bool,
    pub primary_bitmap: i16,
    pub add_bitmap: i16,
    pub compressed_data: Vec<u8>,
}

impl PacketId for ClientboundChunkData {
    fn packet_id(_ver: u32) -> u8 {
        0x33
    }
}

impl Encode for ClientboundChunkData {
    fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
        dst.put_i32(self.chunk_x);
        dst.put_i32(self.chunk_z);
        dst.put_u8(self.continuous as u8);
        dst.put_i16(self.primary_bitmap);
        dst.put_i16(self.add_bitmap);
        dst.extend_from_slice(&(self.compressed_data.len() as i32).to_be_bytes());
        dst.extend_from_slice(&self.compressed_data);
        Ok(())
    }
}

impl Decode for ClientboundChunkData {
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

#[derive(Debug, Clone, PartialEq)]
pub struct ClientboundMultiBlockChange {
    pub chunk_x: i32,
    pub chunk_z: i32,
    pub record_count: i32,
    pub data: Vec<u8>,
}

impl PacketId for ClientboundMultiBlockChange {
    fn packet_id(_ver: u32) -> u8 {
        0x34
    }
}

impl Encode for ClientboundMultiBlockChange {
    fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
        dst.put_i32(self.chunk_x);
        dst.put_i32(self.chunk_z);
        dst.put_i32(self.record_count);
        dst.extend_from_slice(&self.data);
        Ok(())
    }
}

impl Decode for ClientboundMultiBlockChange {
    fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
        need(src, 4 + 4 + 4)?;
        let chunk_x = src.get_i32();
        let chunk_z = src.get_i32();
        let record_count = src.get_i32();
        let data = src.to_vec();
        Ok(Self {
            chunk_x,
            chunk_z,
            record_count,
            data,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClientboundBlockChange {
    pub x: i32,
    pub y: i8,
    pub z: i32,
    pub block_id: i16,
    pub block_metadata: i8,
}

impl PacketId for ClientboundBlockChange {
    fn packet_id(_ver: u32) -> u8 {
        0x35
    }
}

impl Encode for ClientboundBlockChange {
    fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
        dst.put_i32(self.x);
        dst.put_i8(self.y);
        dst.put_i32(self.z);
        dst.put_i16(self.block_id);
        dst.put_i8(self.block_metadata);
        Ok(())
    }
}

impl Decode for ClientboundBlockChange {
    fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
        need(src, 4 + 1 + 4 + 2 + 1)?;
        Ok(Self {
            x: src.get_i32(),
            y: src.get_i8(),
            z: src.get_i32(),
            block_id: src.get_i16(),
            block_metadata: src.get_i8(),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClientboundNamedSoundEffect {
    pub sound_name: String,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub volume: f32,
    pub pitch: u8,
}

impl PacketId for ClientboundNamedSoundEffect {
    fn packet_id(_ver: u32) -> u8 {
        0x3E
    }
}

impl Encode for ClientboundNamedSoundEffect {
    fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
        encode_legacy_string(&self.sound_name, dst);
        dst.put_i32(self.x);
        dst.put_i32(self.y);
        dst.put_i32(self.z);
        dst.put_f32(self.volume);
        dst.put_u8(self.pitch);
        Ok(())
    }
}

impl Decode for ClientboundNamedSoundEffect {
    fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
        let sound_name = decode_legacy_string(src)?;
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

#[derive(Debug, Clone, PartialEq)]
pub struct ClientboundSetSlot {
    pub window_id: i8,
    pub slot: i16,
    pub item_id: i16,
    pub item_count: i8,
    pub item_damage: i16,
    pub nbt_data: Vec<u8>,
}

impl PacketId for ClientboundSetSlot {
    fn packet_id(_ver: u32) -> u8 {
        0x67
    }
}

impl Encode for ClientboundSetSlot {
    fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
        dst.put_i8(self.window_id);
        dst.put_i16(self.slot);
        dst.put_i16(self.item_id);
        dst.put_i8(self.item_count);
        dst.put_i16(self.item_damage);
        dst.put_i16(self.nbt_data.len() as i16);
        dst.extend_from_slice(&self.nbt_data);
        Ok(())
    }
}

impl Decode for ClientboundSetSlot {
    fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
        need(src, 1 + 2 + 2 + 1 + 2 + 2)?;
        let window_id = src.get_i8();
        let slot = src.get_i16();
        let item_id = src.get_i16();
        let item_count = src.get_i8();
        let item_damage = src.get_i16();
        let nbt_len = src.get_i16() as usize;
        if src.remaining() < nbt_len {
            return Err(ProtocolError::UnexpectedEof);
        }
        let nbt_data = src.copy_to_bytes(nbt_len).to_vec();
        Ok(Self {
            window_id,
            slot,
            item_id,
            item_count,
            item_damage,
            nbt_data,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClientboundWindowItems {
    pub window_id: i8,
    pub items: Vec<(i16, i8, i16, Vec<u8>)>,
}

impl PacketId for ClientboundWindowItems {
    fn packet_id(_ver: u32) -> u8 {
        0x68
    }
}

impl Encode for ClientboundWindowItems {
    fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
        dst.put_i8(self.window_id);
        dst.put_i16(self.items.len() as i16);
        for (item_id, item_count, item_damage, nbt_data) in &self.items {
            dst.put_i16(*item_id);
            dst.put_i8(*item_count);
            dst.put_i16(*item_damage);
            dst.put_i16(nbt_data.len() as i16);
            dst.extend_from_slice(nbt_data);
        }
        Ok(())
    }
}

impl Decode for ClientboundWindowItems {
    fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
        need(src, 1 + 2)?;
        let window_id = src.get_i8();
        let count = src.get_i16() as usize;
        let mut items = Vec::with_capacity(count);
        for _ in 0..count {
            need(src, 2 + 1 + 2 + 2)?;
            let item_id = src.get_i16();
            let item_count = src.get_i8();
            let item_damage = src.get_i16();
            let nbt_len = src.get_i16() as usize;
            if src.remaining() < nbt_len {
                return Err(ProtocolError::UnexpectedEof);
            }
            let nbt_data = src.copy_to_bytes(nbt_len).to_vec();
            items.push((item_id, item_count, item_damage, nbt_data));
        }
        Ok(Self { window_id, items })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClientboundUpdateTime {
    pub age: i64,
    pub time: i64,
}

impl PacketId for ClientboundUpdateTime {
    fn packet_id(_ver: u32) -> u8 {
        0x04
    }
}

impl Encode for ClientboundUpdateTime {
    fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
        dst.put_i64(self.age);
        dst.put_i64(self.time);
        Ok(())
    }
}

impl Decode for ClientboundUpdateTime {
    fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
        need(src, 8 + 8)?;
        Ok(Self {
            age: src.get_i64(),
            time: src.get_i64(),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClientboundEntity {
    pub entity_id: i32,
}

impl PacketId for ClientboundEntity {
    fn packet_id(_ver: u32) -> u8 {
        0x1E
    }
}

impl Encode for ClientboundEntity {
    fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
        dst.put_i32(self.entity_id);
        Ok(())
    }
}

impl Decode for ClientboundEntity {
    fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
        need(src, 4)?;
        Ok(Self {
            entity_id: src.get_i32(),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClientboundEntityRelativeMove {
    pub entity_id: i32,
    pub dx: i8,
    pub dy: i8,
    pub dz: i8,
}

impl PacketId for ClientboundEntityRelativeMove {
    fn packet_id(_ver: u32) -> u8 {
        0x15
    }
}

impl Encode for ClientboundEntityRelativeMove {
    fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
        dst.put_i32(self.entity_id);
        dst.put_i8(self.dx);
        dst.put_i8(self.dy);
        dst.put_i8(self.dz);
        Ok(())
    }
}

impl Decode for ClientboundEntityRelativeMove {
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

#[derive(Debug, Clone, PartialEq)]
pub struct ClientboundEntityLook {
    pub entity_id: i32,
    pub yaw: i8,
    pub pitch: i8,
}

impl PacketId for ClientboundEntityLook {
    fn packet_id(_ver: u32) -> u8 {
        0x16
    }
}

impl Encode for ClientboundEntityLook {
    fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
        dst.put_i32(self.entity_id);
        dst.put_i8(self.yaw);
        dst.put_i8(self.pitch);
        Ok(())
    }
}

impl Decode for ClientboundEntityLook {
    fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
        need(src, 4 + 1 + 1)?;
        Ok(Self {
            entity_id: src.get_i32(),
            yaw: src.get_i8(),
            pitch: src.get_i8(),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClientboundEntityMoveLook {
    pub entity_id: i32,
    pub dx: i8,
    pub dy: i8,
    pub dz: i8,
    pub yaw: i8,
    pub pitch: i8,
}

impl PacketId for ClientboundEntityMoveLook {
    fn packet_id(_ver: u32) -> u8 {
        0x17
    }
}

impl Encode for ClientboundEntityMoveLook {
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

impl Decode for ClientboundEntityMoveLook {
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

#[derive(Debug, Clone, PartialEq)]
pub struct ClientboundEntityTeleport {
    pub entity_id: i32,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub yaw: i8,
    pub pitch: i8,
}

impl PacketId for ClientboundEntityTeleport {
    fn packet_id(_ver: u32) -> u8 {
        0x18
    }
}

impl Encode for ClientboundEntityTeleport {
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

impl Decode for ClientboundEntityTeleport {
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

#[derive(Debug, Clone, PartialEq)]
pub struct ClientboundEntityHeadRotation {
    pub entity_id: i32,
    pub head_yaw: i8,
}

impl PacketId for ClientboundEntityHeadRotation {
    fn packet_id(_ver: u32) -> u8 {
        0x19
    }
}

impl Encode for ClientboundEntityHeadRotation {
    fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
        dst.put_i32(self.entity_id);
        dst.put_i8(self.head_yaw);
        Ok(())
    }
}

impl Decode for ClientboundEntityHeadRotation {
    fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
        need(src, 4 + 1)?;
        Ok(Self {
            entity_id: src.get_i32(),
            head_yaw: src.get_i8(),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClientboundSetExperience {
    pub experience_bar: f32,
    pub level: i16,
    pub total_experience: i16,
}

impl PacketId for ClientboundSetExperience {
    fn packet_id(_ver: u32) -> u8 {
        0x2B
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

#[derive(Debug, Clone, PartialEq)]
pub struct ClientboundMapData {
    pub item_damage: i16,
    pub scale: i8,
    pub data: Vec<u8>,
}

impl PacketId for ClientboundMapData {
    fn packet_id(_ver: u32) -> u8 {
        0x36
    }
}

impl Encode for ClientboundMapData {
    fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
        dst.put_i16(self.item_damage);
        dst.put_i8(self.scale);
        dst.extend_from_slice(&self.data);
        Ok(())
    }
}

impl Decode for ClientboundMapData {
    fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
        need(src, 2 + 1)?;
        let item_damage = src.get_i16();
        let scale = src.get_i8();
        let data = src.to_vec();
        Ok(Self {
            item_damage,
            scale,
            data,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClientboundOpenWindow {
    pub window_id: i8,
    pub inventory_type: i8,
    pub window_title: String,
    pub slots_count: i8,
    pub use_provided_title: bool,
}

impl PacketId for ClientboundOpenWindow {
    fn packet_id(_ver: u32) -> u8 {
        0x64
    }
}

impl Encode for ClientboundOpenWindow {
    fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
        dst.put_i8(self.window_id);
        dst.put_i8(self.inventory_type);
        encode_legacy_string(&self.window_title, dst);
        dst.put_i8(self.slots_count);
        dst.put_u8(self.use_provided_title as u8);
        Ok(())
    }
}

impl Decode for ClientboundOpenWindow {
    fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
        let window_id = src.get_i8();
        let inventory_type = src.get_i8();
        let window_title = decode_legacy_string(src)?;
        need(src, 1 + 1)?;
        let slots_count = src.get_i8();
        let use_provided_title = src.get_u8() != 0;
        Ok(Self {
            window_id,
            inventory_type,
            window_title,
            slots_count,
            use_provided_title,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClientboundCloseWindow {
    pub window_id: i8,
}

impl PacketId for ClientboundCloseWindow {
    fn packet_id(_ver: u32) -> u8 {
        0x65
    }
}

impl Encode for ClientboundCloseWindow {
    fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
        dst.put_i8(self.window_id);
        Ok(())
    }
}

impl Decode for ClientboundCloseWindow {
    fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
        need(src, 1)?;
        Ok(Self {
            window_id: src.get_i8(),
        })
    }
}
