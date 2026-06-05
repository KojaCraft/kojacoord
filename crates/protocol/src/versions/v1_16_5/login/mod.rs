use bytes::{Buf, BufMut, Bytes, BytesMut};

use crate::codec::{Decode, Encode, PacketId};
use crate::error::ProtocolError;
use crate::types::VarInt;

fn encode_string(s: &str, dst: &mut BytesMut) -> Result<(), ProtocolError> {
    let bytes = s.as_bytes();
    VarInt(bytes.len() as i32).encode(dst)?;
    dst.put_slice(bytes);
    Ok(())
}

fn decode_string(src: &mut Bytes) -> Result<String, ProtocolError> {
    let len = VarInt::decode(src)?.0 as usize;
    if src.remaining() < len {
        return Err(ProtocolError::Io(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "Missing bytes for string",
        )));
    }
    let mut buf = vec![0u8; len];
    src.copy_to_slice(&mut buf);
    String::from_utf8(buf).map_err(|_| {
        ProtocolError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Invalid UTF-8 in string",
        ))
    })
}

fn encode_byte_array(data: &[u8], dst: &mut BytesMut) -> Result<(), ProtocolError> {
    VarInt(data.len() as i32).encode(dst)?;
    dst.put_slice(data);
    Ok(())
}

fn decode_byte_array(src: &mut Bytes) -> Result<Vec<u8>, ProtocolError> {
    let len = VarInt::decode(src)?.0 as usize;
    if src.remaining() < len {
        return Err(ProtocolError::Io(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "Missing bytes for byte array",
        )));
    }
    let mut buf = vec![0u8; len];
    src.copy_to_slice(&mut buf);
    Ok(buf)
}

#[derive(Debug, Clone, PartialEq)]
pub struct ServerboundLoginStart {
    pub username: String,
}

impl PacketId for ServerboundLoginStart {
    fn packet_id(_ver: u32) -> u8 {
        0x00
    }
}

impl Encode for ServerboundLoginStart {
    fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
        encode_string(&self.username, dst)
    }
}

impl Decode for ServerboundLoginStart {
    fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
        Ok(Self {
            username: decode_string(src)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClientboundLoginDisconnect {
    pub reason: String,
}

impl PacketId for ClientboundLoginDisconnect {
    fn packet_id(_ver: u32) -> u8 {
        0x00
    }
}

impl Encode for ClientboundLoginDisconnect {
    fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
        encode_string(&self.reason, dst)
    }
}

impl Decode for ClientboundLoginDisconnect {
    fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
        Ok(Self {
            reason: decode_string(src)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClientboundEncryptionRequest {
    pub server_id: String,
    pub public_key: Vec<u8>,
    pub verify_token: Vec<u8>,
    pub should_authenticate: bool,
}

impl PacketId for ClientboundEncryptionRequest {
    fn packet_id(_ver: u32) -> u8 {
        0x01
    }
}

impl Encode for ClientboundEncryptionRequest {
    fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
        encode_string(&self.server_id, dst)?;
        encode_byte_array(&self.public_key, dst)?;
        encode_byte_array(&self.verify_token, dst)?;
        dst.put_u8(self.should_authenticate as u8);
        Ok(())
    }
}

impl Decode for ClientboundEncryptionRequest {
    fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
        let server_id = decode_string(src)?;
        let public_key = decode_byte_array(src)?;
        let verify_token = decode_byte_array(src)?;
        let should_authenticate = src.get_u8() != 0;
        Ok(Self {
            server_id,
            public_key,
            verify_token,
            should_authenticate,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProfileProperty {
    pub name: String,
    pub value: String,
    pub signature: Option<String>,
}

impl Encode for ProfileProperty {
    fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
        encode_string(&self.name, dst)?;
        encode_string(&self.value, dst)?;

        if let Some(sig) = &self.signature {
            dst.put_u8(1);
            encode_string(sig, dst)?;
        } else {
            dst.put_u8(0);
        }
        Ok(())
    }
}

impl Decode for ProfileProperty {
    fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
        let name = decode_string(src)?;
        let value = decode_string(src)?;
        if src.remaining() < 1 {
            return Err(ProtocolError::Io(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "Missing boolean flag for ProfileProperty signature",
            )));
        }
        let signature = if src.get_u8() != 0 {
            Some(decode_string(src)?)
        } else {
            None
        };
        Ok(Self {
            name,
            value,
            signature,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClientboundLoginSuccess {
    pub uuid: uuid::Uuid,
    pub username: String,
    pub properties: Vec<ProfileProperty>,
}

impl PacketId for ClientboundLoginSuccess {
    fn packet_id(_ver: u32) -> u8 {
        0x02
    }
}

impl Encode for ClientboundLoginSuccess {
    fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
        let (hi, lo) = self.uuid.as_u64_pair();
        dst.put_i64(hi as i64);
        dst.put_i64(lo as i64);
        encode_string(&self.username, dst)?;
        VarInt(self.properties.len() as i32).encode(dst)?;
        for prop in &self.properties {
            prop.encode(dst)?;
        }
        Ok(())
    }
}

impl Decode for ClientboundLoginSuccess {
    fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
        if src.remaining() < 16 {
            return Err(ProtocolError::Io(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "Missing bytes for ClientboundLoginSuccess uuid",
            )));
        }
        let hi = src.get_i64() as u64;
        let lo = src.get_i64() as u64;
        let uuid = uuid::Uuid::from_u64_pair(hi, lo);
        let username = decode_string(src)?;
        let prop_count = VarInt::decode(src)?.0 as usize;
        let mut properties = Vec::with_capacity(prop_count);
        for _ in 0..prop_count {
            properties.push(ProfileProperty::decode(src)?);
        }
        Ok(Self {
            uuid,
            username,
            properties,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClientboundSetCompression {
    pub threshold: VarInt,
}

impl PacketId for ClientboundSetCompression {
    fn packet_id(_ver: u32) -> u8 {
        0x03
    }
}

impl Encode for ClientboundSetCompression {
    fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
        self.threshold.encode(dst)
    }
}

impl Decode for ClientboundSetCompression {
    fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
        Ok(Self {
            threshold: VarInt::decode(src)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ServerboundEncryptionResponse {
    pub shared_secret: Vec<u8>,
    pub verify_token: Vec<u8>,
}

impl PacketId for ServerboundEncryptionResponse {
    fn packet_id(_ver: u32) -> u8 {
        0x01
    }
}

impl Encode for ServerboundEncryptionResponse {
    fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
        encode_byte_array(&self.shared_secret, dst)?;
        encode_byte_array(&self.verify_token, dst)
    }
}

impl Decode for ServerboundEncryptionResponse {
    fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
        let shared_secret = decode_byte_array(src)?;
        let verify_token = decode_byte_array(src)?;
        Ok(Self {
            shared_secret,
            verify_token,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn login_start_roundtrip() {
        let p = ServerboundLoginStart {
            username: "Alex".to_string(),
        };
        let mut buf = BytesMut::new();
        p.encode(&mut buf).unwrap();
        let mut b = buf.freeze();
        assert_eq!(ServerboundLoginStart::decode(&mut b).unwrap(), p);
    }

    #[test]
    fn login_disconnect_roundtrip() {
        let p = ClientboundLoginDisconnect {
            reason: r#"{"text":"Not whitelisted."}"#.to_string(),
        };
        let mut buf = BytesMut::new();
        p.encode(&mut buf).unwrap();
        let mut b = buf.freeze();
        assert_eq!(ClientboundLoginDisconnect::decode(&mut b).unwrap(), p);
    }

    #[test]
    fn encryption_request_roundtrip() {
        let p = ClientboundEncryptionRequest {
            server_id: String::new(),
            public_key: vec![0xDE, 0xAD, 0xBE, 0xEF],
            verify_token: vec![0x01, 0x02, 0x03, 0x04],
            should_authenticate: true,
        };
        let mut buf = BytesMut::new();
        p.encode(&mut buf).unwrap();
        let mut b = buf.freeze();
        assert_eq!(ClientboundEncryptionRequest::decode(&mut b).unwrap(), p);
    }

    #[test]
    fn login_success_uuid_binary() {
        let p = ClientboundLoginSuccess {
            uuid: uuid::Uuid::new_v4(),
            username: "Alex".to_string(),
            properties: Vec::new(),
        };
        let mut buf = BytesMut::new();
        p.encode(&mut buf).unwrap();
        let mut b = buf.freeze();
        let d = ClientboundLoginSuccess::decode(&mut b).unwrap();
        assert_eq!(d.uuid, p.uuid);
        assert_eq!(d.username, p.username);
        assert_eq!(d.properties, p.properties);
    }

    #[test]
    fn login_success_with_properties() {
        let p = ClientboundLoginSuccess {
            uuid: uuid::Uuid::new_v4(),
            username: "Alex".to_string(),
            properties: vec![
                ProfileProperty {
                    name: "textures".to_string(),
                    value: "base64data==".to_string(),
                    signature: Some("sig".to_string()),
                },
                ProfileProperty {
                    name: "other".to_string(),
                    value: "val".to_string(),
                    signature: None,
                },
            ],
        };
        let mut buf = BytesMut::new();
        p.encode(&mut buf).unwrap();
        let mut b = buf.freeze();
        assert_eq!(ClientboundLoginSuccess::decode(&mut b).unwrap(), p);
    }

    #[test]
    fn set_compression_roundtrip() {
        let p = ClientboundSetCompression {
            threshold: VarInt(256),
        };
        let mut buf = BytesMut::new();
        p.encode(&mut buf).unwrap();
        let mut b = buf.freeze();
        assert_eq!(ClientboundSetCompression::decode(&mut b).unwrap(), p);
    }

    #[test]
    fn encryption_response_roundtrip() {
        let p = ServerboundEncryptionResponse {
            shared_secret: vec![0u8; 128],
            verify_token: vec![0xAA; 128],
        };
        let mut buf = BytesMut::new();
        p.encode(&mut buf).unwrap();
        let mut b = buf.freeze();
        assert_eq!(ServerboundEncryptionResponse::decode(&mut b).unwrap(), p);
    }
}
