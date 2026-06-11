//! 1.21+ configuration-state packets (proto 766 / 767+).
//!
//! IDs shifted by 1 relative to 1.20.4 because `Cookie Request` was inserted
//! at 0x00 in 1.20.5. Verified against minecraft.wiki §Configuration.

use bytes::{Bytes, BytesMut};

use crate::codec::{Decode, Encode, PacketId};
use crate::error::ProtocolError;

#[derive(Debug, Clone, PartialEq)]
pub struct ServerboundAcknowledgeFinishConfiguration;

impl PacketId for ServerboundAcknowledgeFinishConfiguration {
    fn packet_id(_ver: u32) -> u8 {
        // 1.20.4: 0x02. 1.20.5+/1.21: 0x03 (Cookie Response inserted at 0x01).
        0x03
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
        // 1.20.4: 0x02. 1.20.5+/1.21: 0x03 (Cookie Request inserted at 0x00).
        0x03
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
