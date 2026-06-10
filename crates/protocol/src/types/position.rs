//! Packed `(x, y, z)` block position.
//!
//! Wire format is a single i64 split into three signed bitfields. The
//! tricky bit is that Mojang reshuffled the field order in 1.14: the
//! legacy layout is `x:26 | y:12 | z:26`, the modern is `x:26 | z:26 |
//! y:12`. The `Encode`/`Decode` trait impls target the *modern*
//! layout; the converter modules call the free `*_legacy_position`
//! / `*_modern_position` helpers explicitly when they need to
//! disambiguate.

use bytes::{Buf, BufMut, BytesMut};
use serde::{Deserialize, Serialize};

use crate::{
    codec::{Decode, Encode},
    error::ProtocolError,
};

/// Signed integer block coordinates. Bit widths on the wire are
/// 26/12/26 (x/y/z), so values outside ±33,554,431 in xz or ±2,047 in
/// y will be truncated on encode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Position {
    pub x: i32,

    pub y: i32,

    pub z: i32,
}

impl Position {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }
}

impl Encode for Position {
    fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
        let x = (self.x as i64) & 0x3FFFFFF;
        let z = (self.z as i64) & 0x3FFFFFF;
        let y = (self.y as i64) & 0xFFF;

        let packed = ((x << 38) | (z << 12) | y) as u64;
        dst.put_u64(packed);
        Ok(())
    }
}

impl Decode for Position {
    fn decode(src: &mut bytes::Bytes) -> Result<Self, ProtocolError> {
        if src.remaining() < 8 {
            return Err(ProtocolError::UnexpectedEof);
        }

        let val = src.get_u64() as i64;

        let mut x = (val >> 38) & 0x3FFFFFF;
        let mut y = val & 0xFFF;
        let mut z = (val >> 12) & 0x3FFFFFF;

        if x >= 0x2000000 {
            x -= 0x4000000;
        }
        if z >= 0x2000000 {
            z -= 0x4000000;
        }

        if y >= 0x800 {
            y -= 0x1000;
        }

        Ok(Position {
            x: x as i32,
            y: y as i32,
            z: z as i32,
        })
    }
}

// ── Pre-1.14 packed-position helpers ──────────────────────────────────
// The legacy bit layout (`x:26 | y:12 | z:26`, MSB-first) was replaced
// in 1.14 by the modern layout the trait impls above target. Both are
// kept here because the cross-version converters need to call out
// which one they're producing.

/// Unpack a legacy-layout i64 (sign-extended from the bitfields) into
/// a [`Position`]. Used by converters reading from a pre-1.14 source.
pub fn decode_legacy_position(val: u64) -> Position {
    let v = val as i64;
    let mut x = (v >> 38) & 0x3FF_FFFF;
    let mut y = (v >> 26) & 0xFFF;
    let mut z = v & 0x3FF_FFFF;
    if x >= 0x200_0000 {
        x -= 0x400_0000;
    }
    if y >= 0x800 {
        y -= 0x1000;
    }
    if z >= 0x200_0000 {
        z -= 0x400_0000;
    }
    Position {
        x: x as i32,
        y: y as i32,
        z: z as i32,
    }
}

/// Inverse of [`decode_legacy_position`]. Returned as i64 so callers
/// can put it on the wire with `put_i64` directly.
pub fn encode_legacy_position(pos: Position) -> i64 {
    ((pos.x as i64) & 0x3FF_FFFF) << 38
        | ((pos.y as i64) & 0xFFF) << 26
        | ((pos.z as i64) & 0x3FF_FFFF)
}

/// Modern (1.14+) decode as a free function. Functionally identical to
/// `<Position as Decode>::decode`; exists so converter code that's
/// shuttling between layouts can read symmetrically:
/// `decode_legacy_position` / `decode_modern_position` at the same
/// call site.
pub fn decode_modern_position(val: u64) -> Position {
    let v = val as i64;
    let mut x = (v >> 38) & 0x3FF_FFFF;
    let mut z = (v >> 12) & 0x3FF_FFFF;
    let mut y = v & 0xFFF;
    if x >= 0x200_0000 {
        x -= 0x400_0000;
    }
    if z >= 0x200_0000 {
        z -= 0x400_0000;
    }
    if y >= 0x800 {
        y -= 0x1000;
    }
    Position {
        x: x as i32,
        y: y as i32,
        z: z as i32,
    }
}

/// Modern (1.14+) encode as a free function. Pairs with
/// [`decode_modern_position`] for the same reason — see those docs.
pub fn encode_modern_position(pos: Position) -> i64 {
    ((pos.x as i64) & 0x3FF_FFFF) << 38
        | ((pos.z as i64) & 0x3FF_FFFF) << 12
        | ((pos.y as i64) & 0xFFF)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip(pos: Position) -> Position {
        let mut buf = BytesMut::new();
        pos.encode(&mut buf).unwrap();
        let mut bytes = buf.freeze();
        Position::decode(&mut bytes).unwrap()
    }

    #[test]
    fn roundtrip_origin() {
        let p = Position::new(0, 0, 0);
        assert_eq!(roundtrip(p), p);
    }

    #[test]
    fn roundtrip_positive() {
        let p = Position::new(18357644, 831, -20882616);
        assert_eq!(roundtrip(p), p);
    }

    #[test]
    fn roundtrip_negative() {
        let p = Position::new(-1, -2048, -1);
        assert_eq!(roundtrip(p), p);
    }

    #[test]
    fn roundtrip_mixed() {
        let p = Position::new(100, 256, -100);
        assert_eq!(roundtrip(p), p);
    }

    #[test]
    fn roundtrip_max_values() {
        let p = Position::new(33554431, 2047, -33554432);
        assert_eq!(roundtrip(p), p);
    }
}
