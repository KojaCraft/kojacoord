use bytes::{Buf, BufMut, Bytes, BytesMut};

use crate::codec::{Decode, Encode, PacketId};
use crate::error::ProtocolError;
use crate::types::VarInt;

#[derive(Debug, Clone, PartialEq)]
pub struct ServerboundStatusRequest;

impl PacketId for ServerboundStatusRequest {
    fn packet_id(_ver: u32) -> u8 {
        0x00
    }
}

impl Encode for ServerboundStatusRequest {
    fn encode(&self, _dst: &mut BytesMut) -> Result<(), ProtocolError> {
        Ok(())
    }
}

impl Decode for ServerboundStatusRequest {
    fn decode(_src: &mut Bytes) -> Result<Self, ProtocolError> {
        Ok(Self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClientboundStatusResponse {
    pub json_response: String,
}

impl PacketId for ClientboundStatusResponse {
    fn packet_id(_ver: u32) -> u8 {
        0x00
    }
}

impl Encode for ClientboundStatusResponse {
    fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
        let bytes = self.json_response.as_bytes();
        VarInt(bytes.len() as i32).encode(dst)?;
        dst.put_slice(bytes);
        Ok(())
    }
}

impl Decode for ClientboundStatusResponse {
    fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
        let len = VarInt::decode(src)?.0 as usize;
        if src.remaining() < len {
            return Err(ProtocolError::Io(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "Missing bytes for ClientboundStatusResponse json_response",
            )));
        }
        let mut buf = vec![0u8; len];
        src.copy_to_slice(&mut buf);
        let json_response = String::from_utf8(buf).map_err(|_| {
            ProtocolError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid UTF-8 in ClientboundStatusResponse json_response",
            ))
        })?;
        Ok(Self { json_response })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ServerboundPingRequest {
    pub payload: i64,
}

impl PacketId for ServerboundPingRequest {
    fn packet_id(_ver: u32) -> u8 {
        0x01
    }
}

impl Encode for ServerboundPingRequest {
    fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
        dst.put_i64(self.payload);
        Ok(())
    }
}

impl Decode for ServerboundPingRequest {
    fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
        if src.remaining() < 8 {
            return Err(ProtocolError::Io(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "Missing bytes for ServerboundPingRequest payload",
            )));
        }
        Ok(Self {
            payload: src.get_i64(),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClientboundPongResponse {
    pub payload: i64,
}

impl PacketId for ClientboundPongResponse {
    fn packet_id(_ver: u32) -> u8 {
        0x01
    }
}

impl Encode for ClientboundPongResponse {
    fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
        dst.put_i64(self.payload);
        Ok(())
    }
}

impl Decode for ClientboundPongResponse {
    fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
        if src.remaining() < 8 {
            return Err(ProtocolError::Io(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "Missing bytes for ClientboundPongResponse payload",
            )));
        }
        Ok(Self {
            payload: src.get_i64(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_request_roundtrip() {
        let mut buf = BytesMut::new();
        ServerboundStatusRequest.encode(&mut buf).unwrap();
        let mut b = buf.freeze();
        assert_eq!(
            ServerboundStatusRequest::decode(&mut b).unwrap(),
            ServerboundStatusRequest
        );
    }

    #[test]
    fn response_roundtrip() {
        let p = ClientboundStatusResponse {
            json_response: "{}".to_string(),
        };
        let mut buf = BytesMut::new();
        p.encode(&mut buf).unwrap();
        let mut b = buf.freeze();
        assert_eq!(ClientboundStatusResponse::decode(&mut b).unwrap(), p);
    }

    #[test]
    fn ping_roundtrip() {
        let p = ServerboundPingRequest { payload: 99 };
        let mut buf = BytesMut::new();
        p.encode(&mut buf).unwrap();
        let mut b = buf.freeze();
        assert_eq!(ServerboundPingRequest::decode(&mut b).unwrap(), p);
    }

    #[test]
    fn pong_roundtrip() {
        let p = ClientboundPongResponse { payload: -1 };
        let mut buf = BytesMut::new();
        p.encode(&mut buf).unwrap();
        let mut b = buf.freeze();
        assert_eq!(ClientboundPongResponse::decode(&mut b).unwrap(), p);
    }
}
