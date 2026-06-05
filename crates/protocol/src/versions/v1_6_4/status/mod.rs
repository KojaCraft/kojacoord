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

#[derive(Debug, Clone, PartialEq)]
pub struct ClientboundResponse {
    pub response: String,
}

impl PacketId for ClientboundResponse {
    fn packet_id(_ver: u32) -> u8 {
        0x00
    }
}

impl Encode for ClientboundResponse {
    fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
        encode_legacy_string(&self.response, dst);
        Ok(())
    }
}

impl Decode for ClientboundResponse {
    fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
        Ok(Self {
            response: decode_legacy_string(src)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClientboundPong {
    pub payload: i64,
}

impl PacketId for ClientboundPong {
    fn packet_id(_ver: u32) -> u8 {
        0x01
    }
}

impl Encode for ClientboundPong {
    fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
        dst.put_i64(self.payload);
        Ok(())
    }
}

impl Decode for ClientboundPong {
    fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
        Ok(Self {
            payload: src.get_i64(),
        })
    }
}
