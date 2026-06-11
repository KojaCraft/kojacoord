//! Thin wrappers over `PacketRegistry::get_id_for_version` keyed by
//! state / direction. Prefer the compile-time `PacketId` trait at
//! new call sites; this string-keyed surface is mostly used for
//! packet-id comparisons in the relay where the typed struct isn't
//! in scope.

use kojacoord_protocol::{
    registry::{Direction, PacketRegistry, ProtocolState},
    ProtocolVersion,
};

pub use kojacoord_protocol::build_default_registry;

lazy_static::lazy_static! {
    pub static ref REGISTRY: PacketRegistry = build_default_registry();
}

#[inline]
pub fn nearest(proto: u32) -> ProtocolVersion {
    kojacoord_protocol::VersionRegistry::nearest(proto)
}

fn lookup(proto: u32, state: ProtocolState, dir: Direction, name: &'static str) -> u8 {
    REGISTRY
        .get_id_for_version(proto, state, dir, name)
        .unwrap_or_else(|| {
            tracing::warn!(
                packet_name = name,
                protocol = proto,
                ?state,
                ?dir,
                "packet not found in registry — using 0xFF"
            );
            0xFF
        })
}

pub fn cb_play(proto: u32, name: &'static str) -> u8 {
    lookup(proto, ProtocolState::Play, Direction::Clientbound, name)
}

pub fn sb_play(proto: u32, name: &'static str) -> u8 {
    lookup(proto, ProtocolState::Play, Direction::Serverbound, name)
}

pub fn cb_login(proto: u32, name: &'static str) -> u8 {
    lookup(proto, ProtocolState::Login, Direction::Clientbound, name)
}

pub fn sb_login(proto: u32, name: &'static str) -> u8 {
    lookup(proto, ProtocolState::Login, Direction::Serverbound, name)
}

pub fn cb_config(proto: u32, name: &'static str) -> u8 {
    lookup(
        proto,
        ProtocolState::Configuration,
        Direction::Clientbound,
        name,
    )
}

pub fn sb_config(proto: u32, name: &'static str) -> u8 {
    lookup(
        proto,
        ProtocolState::Configuration,
        Direction::Serverbound,
        name,
    )
}

pub fn cb_plugin_message_id(proto: u32) -> u8 {
    cb_play(proto, "ClientboundPluginMessage")
}

pub fn sb_plugin_message_id(proto: u32) -> u8 {
    sb_play(proto, "ServerboundPluginMessage")
}

pub fn cb_chat_id(proto: u32) -> u8 {
    match nearest(proto) {
        ProtocolVersion::V1_6_4
        | ProtocolVersion::V1_7_10
        | ProtocolVersion::V1_8
        | ProtocolVersion::V1_12_2
        | ProtocolVersion::V1_16_5 => cb_play(proto, "ClientboundChatMessage"),
        _ => cb_play(proto, "ClientboundSystemChat"),
    }
}

pub fn chat_packet_ids_for(proto: u32) -> Vec<u8> {
    match nearest(proto) {
        ProtocolVersion::V1_6_4
        | ProtocolVersion::V1_7_10
        | ProtocolVersion::V1_8
        | ProtocolVersion::V1_12_2
        | ProtocolVersion::V1_16_5 => vec![sb_play(proto, "ServerboundChatMessage")],
        _ => {
            let mut ids = vec![sb_play(proto, "ServerboundChatMessage")];
            let cmd_id = sb_play(proto, "ServerboundChatCommand");
            if cmd_id != 0xFF && !ids.contains(&cmd_id) {
                ids.push(cmd_id);
            }
            ids
        },
    }
}
