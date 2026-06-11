//! v1_11_x — alias for the v1_12_x canonical bucket.
//!
//! Minecraft 1.9 / 1.10 / 1.11 / 1.12 all share the same typed-packet
//! wire shape for the subset the proxy looks at (JoinGame, Respawn,
//! KeepAlive, Position, Abilities, Chat, PluginMessage, Sound, etc.).
//! The packet IDs differ between sub-versions but those are dispatched
//! through the version-conditional `PacketId::packet_id(ver)` impls
//! living in v1_12_x — and through the
//! `(protocol, state, direction, name) -> id` table in `registry.rs`.
//!
//! This module re-exports the canonical bucket so call sites can write
//! `v1_11_x::play::ClientboundKeepAlive` while the underlying
//! definition stays in one place.

pub use crate::versions::v1_12_x::{handshake, login, play, status};
