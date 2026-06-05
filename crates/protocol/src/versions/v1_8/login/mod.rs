use bytes::{Buf, BufMut, Bytes, BytesMut};

use crate::codec::{Decode, Encode, PacketId};
use crate::error::ProtocolError;
use crate::types::VarInt;

pub use packets::{
    ClientboundEncryptionRequest, ClientboundLoginDisconnect, ClientboundLoginSuccess,
    ClientboundSetCompression, ProfileProperty, ServerboundEncryptionResponse,
    ServerboundLoginStart,
};

mod packets {
    use super::*;

    fn encode_str(s: &str, dst: &mut BytesMut) -> Result<(), ProtocolError> {
        let bytes = s.as_bytes();
        VarInt(bytes.len() as i32).encode(dst)?;
        dst.put_slice(bytes);
        Ok(())
    }

    fn decode_str(src: &mut Bytes, ctx: &'static str) -> Result<String, ProtocolError> {
        let len = VarInt::decode(src)?.0 as usize;
        if src.remaining() < len {
            return Err(ProtocolError::Io(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                format!("Missing bytes for {ctx}"),
            )));
        }
        let mut b = vec![0u8; len];
        src.copy_to_slice(&mut b);
        String::from_utf8(b).map_err(|_| {
            ProtocolError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Invalid UTF-8 in {ctx}"),
            ))
        })
    }

    fn encode_byte_array(data: &[u8], dst: &mut BytesMut) -> Result<(), ProtocolError> {
        VarInt(data.len() as i32).encode(dst)?;
        dst.put_slice(data);
        Ok(())
    }

    fn decode_byte_array(src: &mut Bytes, ctx: &'static str) -> Result<Vec<u8>, ProtocolError> {
        let len = VarInt::decode(src)?.0 as usize;
        if src.remaining() < len {
            return Err(ProtocolError::Io(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                format!("Missing bytes for {ctx}"),
            )));
        }
        let mut b = vec![0u8; len];
        src.copy_to_slice(&mut b);
        Ok(b)
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
            encode_str(&self.username, dst)
        }
    }

    impl Decode for ServerboundLoginStart {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let username = decode_str(src, "ServerboundLoginStart username")?;
            Ok(Self { username })
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
            encode_str(&self.reason, dst)
        }
    }

    impl Decode for ClientboundLoginDisconnect {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let reason = decode_str(src, "ClientboundLoginDisconnect reason")?;
            Ok(Self { reason })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ClientboundEncryptionRequest {
        pub server_id: String,
        pub public_key: Vec<u8>,
        pub verify_token: Vec<u8>,
    }

    impl PacketId for ClientboundEncryptionRequest {
        fn packet_id(_ver: u32) -> u8 {
            0x01
        }
    }

    impl Encode for ClientboundEncryptionRequest {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            encode_str(&self.server_id, dst)?;
            encode_byte_array(&self.public_key, dst)?;
            encode_byte_array(&self.verify_token, dst)
        }
    }

    impl Decode for ClientboundEncryptionRequest {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let server_id = decode_str(src, "ClientboundEncryptionRequest server_id")?;
            let public_key = decode_byte_array(src, "ClientboundEncryptionRequest public_key")?;
            let verify_token = decode_byte_array(src, "ClientboundEncryptionRequest verify_token")?;
            Ok(Self {
                server_id,
                public_key,
                verify_token,
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
            encode_str(&self.name, dst)?;
            encode_str(&self.value, dst)?;
            match &self.signature {
                Some(sig) => {
                    dst.put_u8(1);
                    encode_str(sig, dst)?;
                },
                None => dst.put_u8(0),
            }
            Ok(())
        }
    }

    impl Decode for ProfileProperty {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let name = decode_str(src, "ProfileProperty name")?;
            let value = decode_str(src, "ProfileProperty value")?;
            if src.remaining() < 1 {
                return Err(ProtocolError::Io(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "Missing signature flag in ProfileProperty",
                )));
            }
            let signature = if src.get_u8() != 0 {
                Some(decode_str(src, "ProfileProperty signature")?)
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
    }

    impl PacketId for ClientboundLoginSuccess {
        fn packet_id(_ver: u32) -> u8 {
            0x02
        }
    }

    impl Encode for ClientboundLoginSuccess {
        fn encode(&self, dst: &mut BytesMut) -> Result<(), ProtocolError> {
            encode_str(&self.uuid.hyphenated().to_string(), dst)?;
            encode_str(&self.username, dst)?;
            Ok(())
        }
    }

    impl Decode for ClientboundLoginSuccess {
        fn decode(src: &mut Bytes) -> Result<Self, ProtocolError> {
            let uuid_str = decode_str(src, "ClientboundLoginSuccess uuid")?;
            let uuid = uuid::Uuid::parse_str(&uuid_str).map_err(|_| {
                ProtocolError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Invalid UUID in ClientboundLoginSuccess",
                ))
            })?;
            let username = decode_str(src, "ClientboundLoginSuccess username")?;

            Ok(Self { uuid, username })
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
            let threshold = VarInt::decode(src)?;
            Ok(Self { threshold })
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
            let shared_secret =
                decode_byte_array(src, "ServerboundEncryptionResponse shared_secret")?;
            let verify_token =
                decode_byte_array(src, "ServerboundEncryptionResponse verify_token")?;
            Ok(Self {
                shared_secret,
                verify_token,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rt_login_start(p: ServerboundLoginStart) -> ServerboundLoginStart {
        let mut buf = BytesMut::new();
        p.encode(&mut buf).unwrap();
        let mut b = buf.freeze();
        ServerboundLoginStart::decode(&mut b).unwrap()
    }

    #[test]
    fn login_start_roundtrip() {
        let p = ServerboundLoginStart {
            username: "Notch".to_string(),
        };
        assert_eq!(rt_login_start(p.clone()), p);
    }

    #[test]
    fn login_disconnect_roundtrip() {
        let p = ClientboundLoginDisconnect {
            reason: r#"{"text":"banned"}"#.to_string(),
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
            public_key: vec![1, 2, 3, 4],
            verify_token: vec![5, 6, 7, 8],
        };
        let mut buf = BytesMut::new();
        p.encode(&mut buf).unwrap();
        let mut b = buf.freeze();
        assert_eq!(ClientboundEncryptionRequest::decode(&mut b).unwrap(), p);
    }

    #[test]
    fn encryption_response_roundtrip() {
        let p = ServerboundEncryptionResponse {
            shared_secret: vec![0xAA; 128],
            verify_token: vec![0xBB; 128],
        };
        let mut buf = BytesMut::new();
        p.encode(&mut buf).unwrap();
        let mut b = buf.freeze();
        assert_eq!(ServerboundEncryptionResponse::decode(&mut b).unwrap(), p);
    }

    #[test]
    fn login_success_roundtrip() {
        let p = ClientboundLoginSuccess {
            uuid: uuid::Uuid::new_v4(),
            username: "Notch".to_string(),
        };
        let mut buf = BytesMut::new();
        p.encode(&mut buf).unwrap();
        let mut b = buf.freeze();
        let decoded = ClientboundLoginSuccess::decode(&mut b).unwrap();
        assert_eq!(decoded.uuid, p.uuid);
        assert_eq!(decoded.username, p.username);
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
}
