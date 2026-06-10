//! Inventory slot wire types.
//!
//! Two shapes coexist because Minecraft refactored slots in 1.13:
//!   - [`LegacySlot`] — pre-1.13: `i16 item_id + i8 count + i16 damage + optional NBT`
//!   - [`Slot`] — 1.13+: `bool present + varint item_id + i8 count + optional NBT`
//!
//! The 1.19.4 / 1.20.5 changes (state-id prefix, structured component
//! data) ride on top of these as wrappers in the converter code; the
//! base structs here stay focused on the round-trip shape.

use bytes::{Buf, BufMut, Bytes, BytesMut};

use crate::{
    codec::{Decode, Encode},
    error::ProtocolError,
    types::{var_int::VarInt, Nbt},
};

#[derive(Debug, Clone, PartialEq)]
pub struct Slot(pub Option<SlotData>);

#[derive(Debug, Clone, PartialEq)]
pub struct SlotData {
    pub item_id: i32,

    pub count: i8,

    pub nbt: Option<Nbt>,
}

impl Encode for Slot {
    fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
        match &self.0 {
            None => {
                dst.put_u8(0);
            },
            Some(data) => {
                dst.put_u8(1);
                VarInt(data.item_id).encode(dst)?;
                dst.put_i8(data.count);
                match &data.nbt {
                    None => dst.put_u8(0),
                    Some(nbt) => nbt.encode(dst)?,
                }
            },
        }
        Ok(())
    }
}

impl Decode for Slot {
    fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
        if src.is_empty() {
            return Err(ProtocolError::UnexpectedEof);
        }
        let present = src.get_u8() != 0;
        if !present {
            return Ok(Slot(None));
        }
        let item_id = VarInt::decode(src)?.0;
        if src.is_empty() {
            return Err(ProtocolError::UnexpectedEof);
        }
        let count = src.get_i8();
        let nbt = if src.first() == Some(&0) {
            src.advance(1);
            None
        } else {
            Some(Nbt::decode(src)?)
        };
        Ok(Slot(Some(SlotData {
            item_id,
            count,
            nbt,
        })))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LegacySlot(pub Option<LegacySlotData>);

#[derive(Debug, Clone, PartialEq)]
pub struct LegacySlotData {
    pub item_id: i16,

    pub count: i8,

    pub damage: i16,

    pub nbt: Option<Nbt>,
}

impl Encode for LegacySlot {
    fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
        match &self.0 {
            None => {
                dst.put_i16(-1);
            },
            Some(data) => {
                dst.put_i16(data.item_id);
                dst.put_i8(data.count);
                dst.put_i16(data.damage);
                match &data.nbt {
                    None => dst.put_u8(0),
                    Some(nbt) => nbt.encode(dst)?,
                }
            },
        }
        Ok(())
    }
}

impl Decode for LegacySlot {
    fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
        if src.remaining() < 2 {
            return Err(ProtocolError::UnexpectedEof);
        }
        let item_id = src.get_i16();
        if item_id == -1 {
            return Ok(LegacySlot(None));
        }
        if src.remaining() < 3 {
            return Err(ProtocolError::UnexpectedEof);
        }
        let count = src.get_i8();
        let damage = src.get_i16();
        let nbt = if src.first() == Some(&0) {
            src.advance(1);
            None
        } else {
            Some(Nbt::decode(src)?)
        };
        Ok(LegacySlot(Some(LegacySlotData {
            item_id,
            count,
            damage,
            nbt,
        })))
    }
}
