//! Per-canonical-version packet modules.
//!
//! Each `v1_X_x` folder either holds full typed-packet structs (the
//! "canonical bucket" modules) or re-exports a sibling canonical
//! bucket whose wire shape it shares (alias modules — there to give
//! call sites and converters per-version namespaces without
//! duplicating ~500 LOC of struct definitions).
//!
//! Canonical buckets (own struct definitions):
//!   * `v1_6_x`  — 1.6.x (pre-netty wire format)
//!   * `v1_7_x`  — 1.7.x
//!   * `v1_8_x`  — 1.8.x
//!   * `v1_12_x` — 1.9.x → 1.12.2 (modern numeric block IDs era)
//!   * `v1_16_x` — 1.13.x → 1.16.5 (flattening + dimension codec)
//!   * `v1_19_x` — 1.17.x → 1.19.4 (chat signing, new world height)
//!   * `v1_20_x` — 1.20.x (configuration phase introduced)
//!   * `v1_21_x` — 1.21.x (registry data rework)
//!
//! Alias modules (re-export from the canonical bucket):
//!   * `v1_9_x`, `v1_10_x`, `v1_11_x` → `v1_12_x`
//!   * `v1_13_x`, `v1_14_x`, `v1_15_x` → `v1_16_x`
//!   * `v1_17_x`, `v1_18_x` → `v1_19_x`
//!
//! Per-sub-version packet-id differences are NOT handled by separate
//! struct definitions — they're handled by the version-conditional
//! `PacketId::packet_id(ver)` impls inside each canonical bucket plus
//! the `(protocol, state, direction, name) -> id` table in
//! `crate::registry`. The id table covers every protocol number we
//! support (1.6.4 through 1.21.5); see `registry.rs`.

pub mod v1_6_x;
pub mod v1_7_x;
pub mod v1_8_x;
pub mod v1_9_x;
pub mod v1_10_x;
pub mod v1_11_x;
pub mod v1_12_x;
pub mod v1_13_x;
pub mod v1_14_x;
pub mod v1_15_x;
pub mod v1_16_x;
pub mod v1_17_x;
pub mod v1_18_x;
pub mod v1_19_x;
pub mod v1_20_x;
pub mod v1_21_x;
