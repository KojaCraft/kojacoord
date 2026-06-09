use bytes::{Bytes, BytesMut};

use crate::codec::{Decode, Encode, PacketId};
use crate::error::ProtocolError;

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
        self.json_response.encode(dst)
    }
}

impl Decode for ClientboundStatusResponse {
    fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
        Ok(Self {
            json_response: String::decode(src)?,
        })
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
        self.payload.encode(dst)
    }
}

impl Decode for ServerboundPingRequest {
    fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
        Ok(Self {
            payload: i64::decode(src)?,
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
        self.payload.encode(dst)
    }
}

impl Decode for ClientboundPongResponse {
    fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
        Ok(Self {
            payload: i64::decode(src)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let p = ServerboundPingRequest { payload: 1234 };
        let mut buf = BytesMut::new();
        p.encode(&mut buf).unwrap();
        let mut b = buf.freeze();
        assert_eq!(ServerboundPingRequest::decode(&mut b).unwrap(), p);
    }
}
