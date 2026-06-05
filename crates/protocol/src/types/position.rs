use bytes::{Buf, BufMut, BytesMut};
use serde::{Deserialize, Serialize};

use crate::{
    codec::{Decode, Encode},
    error::ProtocolError,
};

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
