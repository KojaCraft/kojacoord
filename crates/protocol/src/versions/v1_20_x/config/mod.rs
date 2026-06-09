use bytes::{Bytes, BytesMut};

use crate::codec::{Decode, Encode, PacketId};
use crate::error::ProtocolError;

#[derive(Debug, Clone, PartialEq)]
pub struct ServerboundAcknowledgeFinishConfiguration;

impl PacketId for ServerboundAcknowledgeFinishConfiguration {
    fn packet_id(_ver: u32) -> u8 {
        0x02
    }
}

impl Encode for ServerboundAcknowledgeFinishConfiguration {
    fn encode(&self, _dst: &mut BytesMut) -> Result<(), ProtocolError> {
        Ok(())
    }
}

impl Decode for ServerboundAcknowledgeFinishConfiguration {
    fn decode(_src: &mut Bytes) -> Result<Self, ProtocolError> {
        Ok(Self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClientboundFinishConfiguration;

impl PacketId for ClientboundFinishConfiguration {
    fn packet_id(_ver: u32) -> u8 {
        0x02
    }
}

impl Encode for ClientboundFinishConfiguration {
    fn encode(&self, _dst: &mut BytesMut) -> Result<(), ProtocolError> {
        Ok(())
    }
}

impl Decode for ClientboundFinishConfiguration {
    fn decode(_src: &mut Bytes) -> Result<Self, ProtocolError> {
        Ok(Self)
    }
}
