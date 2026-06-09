pub mod dimension_codec;
pub mod flattening;
mod items;
pub mod modern_to_v1_8;
mod safe;
pub mod v1_12_2_to_v1_16_5;
pub mod v1_16_5_to_v1_12_2;
pub mod v1_16_5_to_v1_20_4;
pub mod v1_20_4_to_v1_16_5;
pub mod v1_6_4_to_v1_12_2;
pub mod v1_6_4_to_v1_16_5;
pub mod v1_7_to_v1_8;
pub mod v1_8_to_modern;
pub mod v1_8_to_v1_7;

use bytes::Bytes;
use kojacoord_protocol::{CanonicalVersion, ProtocolVersion};

pub enum ConversionResult {
    Passthrough,
    Converted(Vec<Bytes>),
    Drop,
    /// Drop the incoming c2s packet and inject these s2c packets back toward
    /// the client. Used for synthetic protocol-state transitions (e.g. emitting
    /// a FinishConfiguration to a 1.20.2+ client whose backend doesn't speak
    /// configuration state).
    InjectS2C(Vec<Bytes>),
}

#[derive(Debug, Clone, Copy)]
pub enum ConversionDirection {
    ServerToClient {
        server_proto: u32,
        client_proto: u32,
    },
    ClientToServer {
        client_proto: u32,
        server_proto: u32,
    },
}

pub struct PacketConverter;

impl PacketConverter {
    pub fn convert(payload: Bytes, direction: ConversionDirection) -> ConversionResult {
        safe::guard("convert", move || Self::convert_inner(payload, direction))
    }

    fn convert_inner(payload: Bytes, direction: ConversionDirection) -> ConversionResult {
        match direction {
            ConversionDirection::ServerToClient {
                server_proto,
                client_proto,
            } => match (nearest(server_proto), nearest(client_proto)) {
                (ProtocolVersion::V1_8, ProtocolVersion::V1_7_10) => {
                    v1_8_to_v1_7::convert_s2c(payload)
                },
                (sv, ProtocolVersion::V1_8) if sv.id() > ProtocolVersion::V1_8.id() => {
                    modern_to_v1_8::convert_s2c(payload, server_proto)
                },
                (sv, ProtocolVersion::V1_7_10) if sv.id() > ProtocolVersion::V1_8.id() => {
                    match modern_to_v1_8::convert_s2c(payload, server_proto) {
                        ConversionResult::Passthrough => ConversionResult::Passthrough,
                        ConversionResult::Drop => ConversionResult::Drop,
                        ConversionResult::InjectS2C(_) => ConversionResult::Drop,
                        ConversionResult::Converted(pkts) => {
                            let mut out = Vec::new();
                            for pkt in pkts {
                                match v1_8_to_v1_7::convert_s2c(pkt) {
                                    ConversionResult::Converted(p2) => out.extend(p2),
                                    ConversionResult::Passthrough => {},
                                    ConversionResult::Drop => {},
                                    ConversionResult::InjectS2C(_) => {},
                                }
                            }
                            if out.is_empty() {
                                ConversionResult::Drop
                            } else {
                                ConversionResult::Converted(out)
                            }
                        },
                    }
                },
                (ProtocolVersion::V1_7_10, ProtocolVersion::V1_8) => {
                    v1_7_to_v1_8::convert_s2c(payload)
                },
                (ProtocolVersion::V1_6_4, ProtocolVersion::V1_12_2) => {
                    v1_6_4_to_v1_12_2::convert_s2c(payload)
                },
                (ProtocolVersion::V1_6_4, ProtocolVersion::V1_16_5) => {
                    v1_6_4_to_v1_16_5::convert_s2c(payload)
                },
                _ => dispatch_canonical_s2c(payload, server_proto, client_proto),
            },

            ConversionDirection::ClientToServer {
                client_proto,
                server_proto,
            } => match (nearest(client_proto), nearest(server_proto)) {
                (ProtocolVersion::V1_7_10, ProtocolVersion::V1_8) => {
                    v1_7_to_v1_8::convert_c2s(payload)
                },

                (ProtocolVersion::V1_8, ProtocolVersion::V1_7_10) => {
                    v1_8_to_v1_7::convert_c2s(payload)
                },
                (ProtocolVersion::V1_8, sv) if sv.id() > ProtocolVersion::V1_8.id() => {
                    v1_8_to_modern::convert_c2s(payload, server_proto)
                },

                (ProtocolVersion::V1_7_10, sv) if sv.id() > ProtocolVersion::V1_8.id() => {
                    match v1_7_to_v1_8::convert_c2s(payload) {
                        ConversionResult::Passthrough => ConversionResult::Passthrough,
                        ConversionResult::Drop => ConversionResult::Drop,
                        ConversionResult::InjectS2C(p) => ConversionResult::InjectS2C(p),
                        ConversionResult::Converted(pkts) => {
                            let mut out = Vec::new();
                            let mut injects = Vec::new();
                            for pkt in pkts {
                                match v1_8_to_modern::convert_c2s(pkt, server_proto) {
                                    ConversionResult::Converted(p2) => out.extend(p2),
                                    ConversionResult::Passthrough => {},
                                    ConversionResult::Drop => {},
                                    ConversionResult::InjectS2C(p2) => injects.extend(p2),
                                }
                            }
                            if !injects.is_empty() && out.is_empty() {
                                ConversionResult::InjectS2C(injects)
                            } else if out.is_empty() {
                                ConversionResult::Drop
                            } else {
                                ConversionResult::Converted(out)
                            }
                        },
                    }
                },
                (ProtocolVersion::V1_6_4, ProtocolVersion::V1_12_2) => {
                    v1_6_4_to_v1_12_2::convert_c2s(payload)
                },
                (ProtocolVersion::V1_6_4, ProtocolVersion::V1_16_5) => {
                    v1_6_4_to_v1_16_5::convert_c2s(payload)
                },
                _ => dispatch_canonical_c2s(payload, client_proto, server_proto),
            },
        }
    }
}

fn nearest(raw: u32) -> ProtocolVersion {
    kojacoord_protocol::VersionRegistry::nearest(raw)
}

/// Canonical bucket for a raw protocol id. Routes 1.21 through the
/// 1.20.4 converter for now — see module docs in `v1_16_5_to_v1_20_4.rs`.
fn canonical_for_dispatch(raw: u32) -> CanonicalVersion {
    let v = nearest(raw).canonical_typed_packet_version();
    match v {
        CanonicalVersion::V1_21 => CanonicalVersion::V1_20_4,
        other => other,
    }
}

fn dispatch_canonical_s2c(
    payload: Bytes,
    server_proto: u32,
    client_proto: u32,
) -> ConversionResult {
    match (
        canonical_for_dispatch(server_proto),
        canonical_for_dispatch(client_proto),
    ) {
        (CanonicalVersion::V1_16_5, CanonicalVersion::V1_20_4) => {
            v1_16_5_to_v1_20_4::convert_s2c(payload)
        },
        (CanonicalVersion::V1_20_4, CanonicalVersion::V1_16_5) => {
            v1_20_4_to_v1_16_5::convert_s2c(payload)
        },
        (CanonicalVersion::V1_12_2, CanonicalVersion::V1_16_5) => {
            v1_12_2_to_v1_16_5::convert_s2c(payload)
        },
        (CanonicalVersion::V1_16_5, CanonicalVersion::V1_12_2) => {
            v1_16_5_to_v1_12_2::convert_s2c(payload)
        },
        _ => ConversionResult::Passthrough,
    }
}

fn dispatch_canonical_c2s(
    payload: Bytes,
    client_proto: u32,
    server_proto: u32,
) -> ConversionResult {
    match (
        canonical_for_dispatch(client_proto),
        canonical_for_dispatch(server_proto),
    ) {
        (CanonicalVersion::V1_20_4, CanonicalVersion::V1_16_5) => {
            v1_20_4_to_v1_16_5::convert_c2s(payload)
        },
        (CanonicalVersion::V1_16_5, CanonicalVersion::V1_20_4) => {
            v1_16_5_to_v1_20_4::convert_c2s(payload)
        },
        (CanonicalVersion::V1_12_2, CanonicalVersion::V1_16_5) => {
            v1_12_2_to_v1_16_5::convert_c2s(payload)
        },
        (CanonicalVersion::V1_16_5, CanonicalVersion::V1_12_2) => {
            v1_16_5_to_v1_12_2::convert_c2s(payload)
        },
        _ => ConversionResult::Passthrough,
    }
}

use bytes::BytesMut;
use kojacoord_protocol::codec::Encode;

pub(crate) fn build_payload(id: u8, body: &[u8]) -> Bytes {
    let mut buf = BytesMut::new();
    kojacoord_protocol::types::VarInt(id as i32)
        .encode(&mut buf)
        .unwrap();
    buf.extend_from_slice(body);
    buf.freeze()
}

pub(crate) fn split_id(mut payload: Bytes) -> Option<(u8, Bytes)> {
    use kojacoord_protocol::codec::Decode;
    let id = kojacoord_protocol::types::VarInt::decode(&mut payload)
        .ok()?
        .0;
    Some((id as u8, payload))
}
