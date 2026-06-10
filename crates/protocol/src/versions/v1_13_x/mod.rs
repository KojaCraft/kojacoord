//! v1_13_x — alias for the v1_16_x canonical bucket.
//!
//! 1.13 introduced "flattening" (numeric → string registry ids); 1.13
//! through 1.16.5 share the same typed-packet shape for the subset the
//! proxy cares about. Per-version ID differences are handled by
//! `PacketId::packet_id(ver)` and the `registry.rs` table.

pub use crate::versions::v1_16_x::{handshake, login, play, status};
