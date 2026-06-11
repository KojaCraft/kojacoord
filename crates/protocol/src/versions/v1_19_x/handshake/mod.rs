use bytes::{Bytes, BytesMut};

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
        self.server_address.encode(dst)?;
        self.server_port.encode(dst)?;
        self.next_state.encode(dst)
    }
}

impl Decode for ServerboundHandshake {
    fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
        let protocol_version = VarInt::decode(src)?;
        let server_address = String::decode(src)?;
        let server_port = u16::decode(src)?;
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
            protocol_version: VarInt(762),
            server_address: "mc.example.com".to_string(),
            server_port: 25565,
            next_state: VarInt(2),
        };
        let mut buf = BytesMut::new();
        p.encode(&mut buf).unwrap();
        let mut b = buf.freeze();
        assert_eq!(ServerboundHandshake::decode(&mut b).unwrap(), p);
    }
}
