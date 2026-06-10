//! v1_18_x — alias for the v1_19_x canonical bucket.
//!
//! 1.17, 1.18, and 1.19 share the same typed-packet shape for the
//! subset the proxy cares about (note: 1.17/1.18 use NBT-shaped Login
//! whereas 1.19+ uses a string `world_type` — limbo guards on that
//! via `proto < 759`). Per-version ID differences are handled by
//! `PacketId::packet_id(ver)` and the `registry.rs` table.

pub use crate::versions::v1_19_x::{handshake, login, play, status};
