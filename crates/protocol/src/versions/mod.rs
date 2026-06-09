//! Per-canonical-version packet modules.
//!
//! Each folder `v1_X_x` covers one entry in
//! [`crate::CanonicalVersion`] — i.e. the version range its typed packet
//! definitions are accurate for. The proxy dispatches to these via
//! `ProtocolVersion::canonical_typed_packet_version()`.
//!
//! Mapping:
//!   * `v1_6_x`  — 1.6.x (pre-netty)
//!   * `v1_7_x`  — 1.7.x
//!   * `v1_8_x`  — 1.8.x
//!   * `v1_12_x` — 1.9.x → 1.12.2 (modern numeric block IDs era)
//!   * `v1_16_x` — 1.13.x → 1.16.5 (flattening + dimension codec)
//!   * `v1_19_x` — 1.17.x → 1.19.4 (chat signing, new world height)
//!   * `v1_20_x` — 1.20.x (configuration phase introduced)
//!   * `v1_21_x` — 1.21.x (registry data rework)
//!
//! Only the packets the proxy actually constructs at the typed layer are
//! re-exported from each module (login/disconnect on the connection side, plus
//! the limbo joingame/respawn/keepalive/abilities/position/heldItem/pluginMessage
//! /chat set).

pub mod v1_12_x;
pub mod v1_16_x;
pub mod v1_19_x;
pub mod v1_20_x;
pub mod v1_21_x;
pub mod v1_6_x;
pub mod v1_7_x;
pub mod v1_8_x;
