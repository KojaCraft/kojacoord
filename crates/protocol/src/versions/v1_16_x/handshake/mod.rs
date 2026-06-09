use bytes::{Buf, BufMut, Bytes, BytesMut};

use crate::codec::{Decode, Encode, PacketId};
use crate::error::ProtocolError;
use crate::types::VarInt;

#[derive(Debug, Clone, PartialEq)]
pub struct ServerboundHandshake {
    pub protocol_version: VarInt,
    pub server_address: String,
    pub server_port: u16,
    pub next_state: VarInt,
}

impl PacketId for ServerboundHandshake {
    fn packet_id(_ver: u32) -> u8 {
        0x00
    }
}

impl Encode for ServerboundHandshake {
    fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
        self.protocol_version.encode(dst)?;

        let addr_bytes = self.server_address.as_bytes();
        VarInt(addr_bytes.len() as i32).encode(dst)?;
        dst.put_slice(addr_bytes);

        dst.put_u16(self.server_port);

        self.next_state.encode(dst)
    }
}

impl Decode for ServerboundHandshake {
    fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
        let protocol_version = VarInt::decode(src)?;

        let addr_len = VarInt::decode(src)?.0 as usize;
        if src.remaining() < addr_len {
            return Err(ProtocolError::Io(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "Missing bytes for ServerboundHandshake server_address",
            )));
        }
        let mut addr_bytes = vec![0u8; addr_len];
        src.copy_to_slice(&mut addr_bytes);
        let server_address = String::from_utf8(addr_bytes).map_err(|_| {
            ProtocolError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid UTF-8 in ServerboundHandshake server_address",
            ))
        })?;

        if src.remaining() < 2 {
            return Err(ProtocolError::Io(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "Missing bytes for ServerboundHandshake server_port",
            )));
        }
        let server_port = src.get_u16();

        let next_state = VarInt::decode(src)?;

        Ok(Self {
            protocol_version,
            server_address,
            server_port,
            next_state,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let p = ServerboundHandshake {
            protocol_version: VarInt(754),
            server_address: "play.example.com".to_string(),
            server_port: 25565,
            next_state: VarInt(2),
        };
        let mut buf = BytesMut::new();
        p.encode(&mut buf).unwrap();
        let mut b = buf.freeze();
        assert_eq!(ServerboundHandshake::decode(&mut b).unwrap(), p);
    }
}
