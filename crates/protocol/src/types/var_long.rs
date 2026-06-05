use bytes::{Buf, BufMut, Bytes, BytesMut};

use crate::{
    codec::{Decode, Encode},
    error::ProtocolError,
};

pub const VARLONG_MAX_BYTES: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct VarLong(pub i64);

impl Encode for VarLong {
    fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
        let mut val = self.0 as u64;
        loop {
            let byte = (val & 0x7F) as u8;
            val >>= 7;
            if val != 0 {
                dst.put_u8(byte | 0x80);
            } else {
                dst.put_u8(byte);
                break;
            }
        }
        Ok(())
    }
}

impl Decode for VarLong {
    fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
        let mut result: u64 = 0;
        let mut shift = 0u32;
        let mut bytes_read = 0;

        loop {
            if bytes_read >= VARLONG_MAX_BYTES {
                return Err(ProtocolError::VarIntOverflow(bytes_read));
            }
            if src.is_empty() {
                return Err(ProtocolError::UnexpectedEof);
            }
            let byte = src.get_u8();
            bytes_read += 1;
            result |= ((byte & 0x7F) as u64) << shift;
            shift += 7;
            if byte & 0x80 == 0 {
                break;
            }
        }

        Ok(VarLong(result as i64))
    }
}

impl From<i64> for VarLong {
    fn from(v: i64) -> Self {
        VarLong(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip(val: i64) -> i64 {
        let mut buf = BytesMut::new();
        VarLong(val).encode(&mut buf).unwrap();
        let mut bytes = buf.freeze();
        VarLong::decode(&mut bytes).unwrap().0
    }

    #[test]
    fn roundtrip_zero() {
        assert_eq!(roundtrip(0), 0);
    }
    #[test]
    fn roundtrip_max() {
        assert_eq!(roundtrip(i64::MAX), i64::MAX);
    }
    #[test]
    fn roundtrip_min() {
        assert_eq!(roundtrip(i64::MIN), i64::MIN);
    }
    #[test]
    fn roundtrip_minus_one() {
        assert_eq!(roundtrip(-1), -1);
    }
}
