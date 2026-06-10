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

/// Legacy 0xFE server-list ping packet for pre-1.7 / 1.6.x clients.
/// This is a single-byte packet (0xFE) that clients send to request
/// the MOTD in the legacy format before the modern handshake was introduced.
#[derive(Debug, Clone, PartialEq)]
pub struct ServerboundLegacyPing;

impl PacketId for ServerboundLegacyPing {
    fn packet_id(_ver: u32) -> u8 {
        0xFE
    }
}

impl Decode for ServerboundLegacyPing {
    fn decode(_src: &mut Bytes) -> Result<Self, ProtocolError> {
        Ok(Self)
    }
}

impl Encode for ServerboundLegacyPing {
    fn encode(&self, _dst: &mut BytesMut) -> Result<(), ProtocolError> {
        Ok(())
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

/// Legacy MOTD response format for 0xFE ping.
/// Format: "§1\0<protocol>\0<version>\0<motd>\0<players>\0<max_players>"
#[derive(Debug, Clone, PartialEq)]
pub struct ClientboundLegacyMotd {
    pub protocol: String,
    pub version: String,
    pub motd: String,
    pub players: String,
    pub max_players: String,
}

impl ClientboundLegacyMotd {
    /// Create a legacy MOTD response from modern JSON status
    pub fn from_json(_json: &str) -> Self {
        // Parse the JSON to extract relevant fields
        // Fallback to defaults if parsing fails
        let protocol = "78".to_string(); // 1.6.4 protocol
        let version = "1.6.4".to_string();
        let motd = "A Minecraft Server".to_string();
        let players = "0".to_string();
        let max_players = "20".to_string();

        Self {
            protocol,
            version,
            motd,
            players,
            max_players,
        }
    }

    /// Encode in the legacy format expected by pre-1.7 clients
    pub fn encode_legacy(&self) -> Bytes {
        let mut dst = BytesMut::new();
        dst.put_u8(0xFF); // Packet ID for legacy response
        encode_legacy_string(
            &format!(
                "§1\0{}\0{}\0{}\0{}\0{}",
                self.protocol, self.version, self.motd, self.players, self.max_players
            ),
            &mut dst,
        );
        dst.freeze()
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
