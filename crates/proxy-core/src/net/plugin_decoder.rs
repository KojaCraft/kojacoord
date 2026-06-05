use bytes::{Buf, Bytes};
use kojacoord_protocol::{codec::Decode, ProtocolVersion};

#[derive(Debug, Clone)]
pub struct PluginMessage {
    pub channel: String,
    pub data: Vec<u8>,
}

pub fn decode_serverbound_plugin_message(
    mut payload: Bytes,
    proto: u32,
) -> Result<PluginMessage, PluginMessageError> {
    let ver = kojacoord_protocol::VersionRegistry::nearest(proto);

    match ver {
        ProtocolVersion::V1_6_4 | ProtocolVersion::V1_7_10 | ProtocolVersion::V1_8 => {
            let channel = String::decode(&mut payload)
                .map_err(|e| PluginMessageError::Decode(format!("channel: {}", e)))?;

            if payload.len() < 2 {
                return Err(PluginMessageError::UnexpectedEof);
            }
            let length = u16::from_be_bytes([payload[0], payload[1]]) as usize;
            payload.advance(2);

            if payload.len() < length {
                return Err(PluginMessageError::DataTooShort {
                    expected: length,
                    got: payload.len(),
                });
            }

            let data = payload.slice(0..length).to_vec();

            Ok(PluginMessage { channel, data })
        },
        ProtocolVersion::V1_12_2 => {
            let channel = String::decode(&mut payload)
                .map_err(|e| PluginMessageError::Decode(format!("channel: {}", e)))?;
            let data = payload.to_vec();

            Ok(PluginMessage { channel, data })
        },
        _ => {
            let channel = String::decode(&mut payload)
                .map_err(|e| PluginMessageError::Decode(format!("channel: {}", e)))?;
            let data = payload.to_vec();

            Ok(PluginMessage { channel, data })
        },
    }
}

pub fn decode_clientbound_plugin_message(
    mut payload: Bytes,
    proto: u32,
) -> Result<PluginMessage, PluginMessageError> {
    let ver = kojacoord_protocol::VersionRegistry::nearest(proto);

    match ver {
        ProtocolVersion::V1_6_4 | ProtocolVersion::V1_7_10 | ProtocolVersion::V1_8 => {
            let channel = String::decode(&mut payload)
                .map_err(|e| PluginMessageError::Decode(format!("channel: {}", e)))?;

            if payload.len() < 2 {
                return Err(PluginMessageError::UnexpectedEof);
            }
            let length = u16::from_be_bytes([payload[0], payload[1]]) as usize;
            payload.advance(2);

            if payload.len() < length {
                return Err(PluginMessageError::DataTooShort {
                    expected: length,
                    got: payload.len(),
                });
            }

            let data = payload.slice(0..length).to_vec();

            Ok(PluginMessage { channel, data })
        },
        _ => {
            let channel = String::decode(&mut payload)
                .map_err(|e| PluginMessageError::Decode(format!("channel: {}", e)))?;
            let data = payload.to_vec();

            Ok(PluginMessage { channel, data })
        },
    }
}

pub fn encode_plugin_message(
    channel: &str,
    data: &[u8],
    proto: u32,
) -> Result<Bytes, PluginMessageError> {
    use bytes::BytesMut;
    use kojacoord_protocol::codec::Encode;

    let ver = kojacoord_protocol::VersionRegistry::nearest(proto);
    let mut buf = BytesMut::new();

    match ver {
        ProtocolVersion::V1_6_4 | ProtocolVersion::V1_7_10 | ProtocolVersion::V1_8 => {
            channel
                .to_owned()
                .encode(&mut buf)
                .map_err(|e| PluginMessageError::Encode(format!("channel: {}", e)))?;

            if data.len() > u16::MAX as usize {
                return Err(PluginMessageError::DataTooLarge {
                    size: data.len(),
                    max: u16::MAX as usize,
                });
            }

            (data.len() as u16)
                .encode(&mut buf)
                .map_err(|e| PluginMessageError::Encode(format!("length: {}", e)))?;
            buf.extend_from_slice(data);
        },
        _ => {
            channel
                .to_owned()
                .encode(&mut buf)
                .map_err(|e| PluginMessageError::Encode(format!("channel: {}", e)))?;
            buf.extend_from_slice(data);
        },
    }

    Ok(buf.freeze())
}

#[derive(Debug, thiserror::Error)]
pub enum PluginMessageError {
    #[error("Failed to decode: {0}")]
    Decode(String),

    #[error("Failed to encode: {0}")]
    Encode(String),

    #[error("Unexpected end of data")]
    UnexpectedEof,

    #[error("Data too short: expected {expected} bytes, got {got}")]
    DataTooShort { expected: usize, got: usize },

    #[error("Data too large: {size} bytes exceeds maximum of {max}")]
    DataTooLarge { size: usize, max: usize },
}
