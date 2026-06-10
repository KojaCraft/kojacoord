//! Configuration-phase shimming for the 1.20.2 boundary.
//!
//! 1.20.2 introduced a Configuration state between Login and Play —
//! the server pushes registries, tags, and resource-pack info before
//! handing control to Play. Pre-1.20.2 servers don't run that phase at
//! all; modern clients will sit forever waiting for `FinishConfiguration`
//! if we just forward the legacy `LoginSuccess` straight through.
//!
//! When the client is 1.20.2+ and the backend isn't, we synthesise the
//! single `FinishConfiguration` packet here so the client transitions
//! into Play and starts accepting the backend's legacy join sequence.
//! The reverse direction (legacy client → modern backend) is handled
//! by swallowing the backend's config-phase packets in the relay.

use bytes::BytesMut;
use kojacoord_protocol::{codec::Encode, types::VarInt, Epoch, ProtocolVersion};

/// True for 1.20.2+ (proto 764+) — versions that run the configuration
/// state between Login and Play.
pub fn has_configuration_phase(protocol_version: u32) -> bool {
    ProtocolVersion::from_id(protocol_version).has_configuration_phase()
}

/// True when the client expects a config phase the backend won't
/// produce. Inverse direction is handled by swallowing packets in the
/// relay, not by synthesis.
pub fn needs_synthesis(client_protocol: u32, backend_protocol: u32) -> bool {
    has_configuration_phase(client_protocol) && !has_configuration_phase(backend_protocol)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SynthesisMode {
    /// Both ends agree; relay forwards untouched.
    None,
    /// Client expects config phase, backend doesn't — proxy injects
    /// `FinishConfiguration` toward the client after we swallow its
    /// `LoginAcknowledged`.
    ClientSide,
    /// Backend expects config phase, client doesn't — relay swallows
    /// the backend's config-phase packets and replies on the client's
    /// behalf.
    BackendSide,
}

/// Pick a [`SynthesisMode`] from the client/backend version pair.
pub fn determine_synthesis_mode(client_protocol: u32, backend_protocol: u32) -> SynthesisMode {
    match (
        has_configuration_phase(client_protocol),
        has_configuration_phase(backend_protocol),
    ) {
        (true, false) => SynthesisMode::ClientSide,
        (false, true) => SynthesisMode::BackendSide,
        _ => SynthesisMode::None,
    }
}

/// Encode the bare clientbound `FinishConfiguration` packet (no body —
/// the packet is just an id signalling "config done, transition to
/// play"). Returns `Err` if called on a pre-1.20.2 version where the
/// packet doesn't exist.
pub fn build_cfg_finish_packet(protocol_version: u32) -> Result<Vec<u8>, String> {
    let canonical = ProtocolVersion::from_id(protocol_version);

    if !has_configuration_phase(protocol_version) {
        return Err("Protocol version does not have configuration phase".into());
    }

    let mut payload = BytesMut::new();

    // FinishConfiguration packet id (clientbound):
    //   1.20.2 – 1.20.4 (protos 764, 765): 0x02
    //   1.20.5 / 1.20.6 (proto 766):        0x03
    //   1.21+           (proto >= 767):     0x03
    let packet_id = match canonical.epoch() {
        Epoch::V1_20 => {
            if canonical.id() >= 766 {
                0x03
            } else {
                0x02
            }
        },
        Epoch::V1_21Plus => 0x03,
        _ => return Err("config synthesis only valid for 1.20.2+".into()),
    };

    VarInt(packet_id)
        .encode(&mut payload)
        .map_err(|e| format!("encode packet id: {}", e))?;

    Ok(payload.to_vec())
}
