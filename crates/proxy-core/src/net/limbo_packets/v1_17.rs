//! Limbo packets for v1_17.x — same wire shape as the v1_19_x bucket.
//! Note: pre-1.19 (proto <759) returns None from chat/join_game because
//! 1.17/1.18 carry an NBT dimension blob limbo doesn't synthesise.
pub use super::v1_19::V1_19 as V1_17;
