//! Login-state packet builders, keyed by canonical version.
//!
//! Mirrors [`crate::limbo_packets`] but for the smaller surface
//! `connection.rs` needs during the login handshake:
//!   * Server-issued LoginSuccess
//!   * Server-issued EncryptionRequest
//!   * Server-issued LoginDisconnect
//!
//! Each builder returns `(packet_id, body)` as an
//! [`EncodedPacket`]. `None` means "this canonical bucket doesn't
//! speak that packet" (e.g. pre-netty has its own LoginRequestS2C
//! shape that's handled separately).

use bytes::BytesMut;
use kojacoord_protocol::{
    codec::{Encode, PacketId},
    CanonicalVersion,
};
use uuid::Uuid;

pub struct EncodedPacket {
    pub id: u8,
    pub body: BytesMut,
}

/// Encode a typed login packet (`id` + `body`). Returns `None` when
/// the proto sentinel says this packet doesn't exist for that
/// version.
fn encode<T: Encode + PacketId>(proto: u32, pkt: T) -> Option<EncodedPacket> {
    let id = T::packet_id(proto);
    if id == 0xFF {
        return None;
    }
    let mut body = BytesMut::new();
    pkt.encode(&mut body).ok()?;
    Some(EncodedPacket { id, body })
}

/// Profile properties as the caller knows them (auth crate type).
/// Each impl converts these into its own typed `ProfileProperty`
/// before encoding.
pub struct LoginProfile<'a> {
    pub uuid: Uuid,
    pub username: &'a str,
    pub properties: &'a [kojacoord_auth::ProfileProperty],
}

/// Build a clientbound LoginSuccess packet for the given canonical
/// bucket. Returns `None` only for pre-netty (1.6.x) — call sites that
/// support 1.6 must use the alternate `LoginRequestS2C` path.
pub fn build_login_success(
    canonical: CanonicalVersion,
    proto: u32,
    profile: &LoginProfile<'_>,
) -> Option<EncodedPacket> {
    let uuid = profile.uuid;
    let username = profile.username.to_owned();
    match canonical {
        CanonicalVersion::V1_6_4 => None,
        CanonicalVersion::V1_7_10 => {
            use kojacoord_protocol::versions::v1_7_x::login::ClientboundLoginSuccess;
            encode(proto, ClientboundLoginSuccess { uuid, username })
        },
        CanonicalVersion::V1_8 => {
            use kojacoord_protocol::versions::v1_8_x::login::ClientboundLoginSuccess;
            encode(proto, ClientboundLoginSuccess { uuid, username })
        },
        CanonicalVersion::V1_12_2 => {
            use kojacoord_protocol::versions::v1_12_x::login::{
                ClientboundLoginSuccess, ProfileProperty,
            };
            encode(
                proto,
                ClientboundLoginSuccess {
                    uuid,
                    username,
                    properties: profile
                        .properties
                        .iter()
                        .map(|p| ProfileProperty {
                            name: p.name.clone(),
                            value: p.value.clone(),
                            signature: p.signature.clone(),
                        })
                        .collect(),
                },
            )
        },
        CanonicalVersion::V1_16_5 => {
            use kojacoord_protocol::versions::v1_16_x::login::{
                ClientboundLoginSuccess, ProfileProperty,
            };
            encode(
                proto,
                ClientboundLoginSuccess {
                    uuid,
                    username,
                    properties: profile
                        .properties
                        .iter()
                        .map(|p| ProfileProperty {
                            name: p.name.clone(),
                            value: p.value.clone(),
                            signature: p.signature.clone(),
                        })
                        .collect(),
                },
            )
        },
        CanonicalVersion::V1_19_4 => {
            use kojacoord_protocol::versions::v1_19_x::login::{
                ClientboundLoginSuccess, ProfileProperty,
            };
            encode(
                proto,
                ClientboundLoginSuccess {
                    uuid,
                    username,
                    properties: profile
                        .properties
                        .iter()
                        .map(|p| ProfileProperty {
                            name: p.name.clone(),
                            value: p.value.clone(),
                            signature: p.signature.clone(),
                        })
                        .collect(),
                },
            )
        },
        CanonicalVersion::V1_20_4 => {
            use kojacoord_protocol::versions::v1_20_x::login::{
                ClientboundLoginSuccess, ProfileProperty,
            };
            // `strictErrorHandling` lives on the wire for 1.20.5+ (proto 766).
            // For 1.20.0–1.20.4 the trailing byte must be absent.
            let strict = if proto >= 766 { Some(true) } else { None };
            encode(
                proto,
                ClientboundLoginSuccess {
                    uuid,
                    username,
                    properties: profile
                        .properties
                        .iter()
                        .map(|p| ProfileProperty {
                            name: p.name.clone(),
                            value: p.value.clone(),
                            signature: p.signature.clone(),
                        })
                        .collect(),
                    strict_error_handling: strict,
                },
            )
        },
        CanonicalVersion::V1_21 => {
            use kojacoord_protocol::versions::v1_21_x::login::{
                ClientboundLoginSuccess, ProfileProperty,
            };
            // 1.21 (767) and 1.21.2 (768) carry strictErrorHandling.
            // 1.21.4+ (769+) dropped it again.
            let strict = if (767..=768).contains(&proto) {
                Some(true)
            } else {
                None
            };
            encode(
                proto,
                ClientboundLoginSuccess {
                    uuid,
                    username,
                    properties: profile
                        .properties
                        .iter()
                        .map(|p| ProfileProperty {
                            name: p.name.clone(),
                            value: p.value.clone(),
                            signature: p.signature.clone(),
                        })
                        .collect(),
                    strict_error_handling: strict,
                },
            )
        },
    }
}

/// Build a clientbound EncryptionRequest packet.
/// `proto` controls whether `should_authenticate` is serialised
/// (1.20.5+).
pub fn build_encryption_request(
    canonical: CanonicalVersion,
    proto: u32,
    server_id: &str,
    public_key: &[u8],
    verify_token: &[u8],
) -> Option<EncodedPacket> {
    let server_id = server_id.to_owned();
    let public_key = public_key.to_vec();
    let verify_token = verify_token.to_vec();
    match canonical {
        CanonicalVersion::V1_6_4 => None,
        CanonicalVersion::V1_7_10 => {
            use kojacoord_protocol::versions::v1_7_x::login::ClientboundEncryptionRequest;
            encode(
                proto,
                ClientboundEncryptionRequest {
                    server_id,
                    public_key,
                    verify_token,
                },
            )
        },
        CanonicalVersion::V1_8 => {
            use kojacoord_protocol::versions::v1_8_x::login::ClientboundEncryptionRequest;
            encode(
                proto,
                ClientboundEncryptionRequest {
                    server_id,
                    public_key,
                    verify_token,
                },
            )
        },
        CanonicalVersion::V1_12_2 => {
            use kojacoord_protocol::versions::v1_12_x::login::ClientboundEncryptionRequest;
            encode(
                proto,
                ClientboundEncryptionRequest {
                    server_id,
                    public_key,
                    verify_token,
                },
            )
        },
        CanonicalVersion::V1_16_5 => {
            use kojacoord_protocol::versions::v1_16_x::login::ClientboundEncryptionRequest;
            encode(
                proto,
                ClientboundEncryptionRequest {
                    server_id,
                    public_key,
                    verify_token,
                },
            )
        },
        CanonicalVersion::V1_19_4 => {
            use kojacoord_protocol::versions::v1_19_x::login::ClientboundEncryptionRequest;
            encode(
                proto,
                ClientboundEncryptionRequest {
                    server_id,
                    public_key,
                    verify_token,
                },
            )
        },
        CanonicalVersion::V1_20_4 => {
            use kojacoord_protocol::versions::v1_20_x::login::ClientboundEncryptionRequest;
            // 1.20.5+ added the `should_authenticate` boolean.
            let auth = if proto >= 766 { Some(true) } else { None };
            encode(
                proto,
                ClientboundEncryptionRequest {
                    server_id,
                    public_key,
                    verify_token,
                    should_authenticate: auth,
                },
            )
        },
        CanonicalVersion::V1_21 => {
            use kojacoord_protocol::versions::v1_21_x::login::ClientboundEncryptionRequest;
            encode(
                proto,
                ClientboundEncryptionRequest {
                    server_id,
                    public_key,
                    verify_token,
                    should_authenticate: Some(true),
                },
            )
        },
    }
}

/// Build a clientbound LoginDisconnect packet. Pre-netty (1.6.x) uses
/// a different framing — callers handle that separately and only call
/// this for 1.7+.
pub fn build_login_disconnect(
    canonical: CanonicalVersion,
    proto: u32,
    reason_json: &str,
) -> Option<EncodedPacket> {
    let reason = reason_json.to_owned();
    match canonical {
        CanonicalVersion::V1_6_4 => None,
        CanonicalVersion::V1_7_10 => {
            use kojacoord_protocol::versions::v1_7_x::login::ClientboundLoginDisconnect;
            encode(proto, ClientboundLoginDisconnect { reason })
        },
        CanonicalVersion::V1_8 => {
            use kojacoord_protocol::versions::v1_8_x::login::ClientboundLoginDisconnect;
            encode(proto, ClientboundLoginDisconnect { reason })
        },
        CanonicalVersion::V1_12_2 => {
            use kojacoord_protocol::versions::v1_12_x::login::ClientboundLoginDisconnect;
            encode(proto, ClientboundLoginDisconnect { reason })
        },
        CanonicalVersion::V1_16_5 => {
            use kojacoord_protocol::versions::v1_16_x::login::ClientboundLoginDisconnect;
            encode(proto, ClientboundLoginDisconnect { reason })
        },
        CanonicalVersion::V1_19_4 => {
            use kojacoord_protocol::versions::v1_19_x::login::ClientboundLoginDisconnect;
            encode(proto, ClientboundLoginDisconnect { reason })
        },
        CanonicalVersion::V1_20_4 => {
            use kojacoord_protocol::versions::v1_20_x::login::ClientboundLoginDisconnect;
            encode(proto, ClientboundLoginDisconnect { reason })
        },
        CanonicalVersion::V1_21 => {
            use kojacoord_protocol::versions::v1_21_x::login::ClientboundLoginDisconnect;
            encode(proto, ClientboundLoginDisconnect { reason })
        },
    }
}
