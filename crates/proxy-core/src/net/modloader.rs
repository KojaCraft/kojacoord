use bytes::{BufMut, Bytes, BytesMut};
use kojacoord_protocol::codec::Encode;
use kojacoord_protocol::types::VarInt;
use tracing::{debug, info, warn};

pub const FML1_HS: &str = "FML|HS";
pub const FML1_CHAN: &str = "FML";
pub const FML1_MP: &str = "FML|MP";
pub const FML1_FORGE: &str = "FORGE";
pub const FML_REGISTER: &str = "REGISTER";
pub const FML_UNREGISTER: &str = "UNREGISTER";

pub const FML1_PLAY_CHANNELS: &[&str] = &[
    FML1_HS,
    FML1_CHAN,
    FML1_MP,
    FML1_FORGE,
    FML_REGISTER,
    FML_UNREGISTER,
];

pub const FML3_LOGIN_WRAPPER: &str = "fml:loginwrapper";
pub const FML3_HANDSHAKE: &str = "fml:handshake";

pub const MC_REGISTER: &str = "minecraft:register";
pub const MC_UNREGISTER: &str = "minecraft:unregister";
pub const MC_NETWORK: &str = "minecraft:network";

pub const COMMON_VERSION: &str = "c:version";
pub const COMMON_REGISTER: &str = "c:register";

pub const NEO_REGISTER: &str = "neoforge:register";
pub const NEO_TIER_SORTING: &str = "neoforge:tier_sorting";
pub const NEO_ADVANCED_ADDON: &str = "neoforge:advanced_addon";
pub const NEO_SETUP_FAILED: &str = "neoforge:modded_network_setup_failed";

pub const FABRIC_REGISTER: &str = "fabric-networking-api-v1:register_channel";
pub const FABRIC_UNREGISTER: &str = "fabric-networking-api-v1:unregister_channel";

// Quilt is a Fabric fork. The runtime ships its own networking lib (QSL) on
// top of Fabric's, and clients sometimes advertise both. We sniff any of these
// to flag a connection as Quilt rather than plain Fabric.
pub const QUILT_INIT_CHANNEL: &str = "quilt:init";
pub const QUILT_REGISTER: &str = "quilt:register";
pub const QUILT_QSL_NETWORKING: &str = "qsl:networking";
pub const QUILTED_FABRIC_API_REGISTER: &str = "quilted_fabric_networking_api_v1:register_channel";

pub const CONFIG_PING_ID: u8 = 0x01;
pub const CONFIG_PONG_ID: u8 = 0x00;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FmlDiscriminator {
    ServerHello,
    ClientHello,
    ModList,
    RegistryData,
    HandshakeAck,
    HandshakeReset,
    Unknown(u8),
}

impl From<u8> for FmlDiscriminator {
    fn from(b: u8) -> Self {
        match b {
            0x00 => Self::ServerHello,
            0x01 => Self::ClientHello,
            0x02 => Self::ModList,
            0x03 => Self::RegistryData,
            0xFF => Self::HandshakeAck,
            0xFE => Self::HandshakeReset,
            other => Self::Unknown(other),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ModloaderKind {
    #[default]
    Unknown,
    Fml1,
    Fml2,
    Fml3,
    NeoForge,
    Fabric,
    Quilt,
    Vanilla,
}

#[derive(Debug, Default)]
pub struct ModloaderSession {
    pub kind: ModloaderKind,
    pub client_mods: Vec<(String, String)>,
    pub server_mods: Vec<(String, String)>,
    pub client_channels: Vec<String>,
    pub server_channels: Vec<String>,
    pub handshake_complete: bool,
}

impl ModloaderSession {
    pub fn new() -> Self {
        Self::default()
    }
}

pub fn has_fml_marker(addr: &str) -> bool {
    addr.contains("\0FML\0") || addr.contains("\0FML2\0") || addr.contains("\0FML3\0")
}

pub fn apply_fml_marker(addr: &str, kind: ModloaderKind) -> String {
    let clean = addr
        .trim_end_matches('\0')
        .split('\0')
        .next()
        .unwrap_or(addr);
    match kind {
        ModloaderKind::Fml1 => format!("{}\0FML\0", clean),
        ModloaderKind::Fml2 => format!("{}\0FML2\0", clean),
        ModloaderKind::Fml3 => format!("{}\0FML3\0", clean),
        _ => clean.to_owned(),
    }
}

pub fn detect_from_address(addr: &str) -> ModloaderKind {
    if addr.contains("\0FML3\0") {
        ModloaderKind::Fml3
    } else if addr.contains("\0FML2\0") {
        ModloaderKind::Fml2
    } else if addr.contains("\0FML\0") {
        ModloaderKind::Fml1
    } else {
        ModloaderKind::Unknown
    }
}

#[inline]
pub fn is_fml1_play_channel(channel: &str) -> bool {
    FML1_PLAY_CHANNELS.contains(&channel)
}

pub fn parse_fml1_discriminator(body: &[u8]) -> Option<FmlDiscriminator> {
    body.first().copied().map(FmlDiscriminator::from)
}

pub fn build_fml1_handshake_reset(plugin_msg_id: u8) -> Bytes {
    let mut payload = BytesMut::new();
    VarInt(plugin_msg_id as i32).encode(&mut payload).unwrap();
    FML1_HS.to_owned().encode(&mut payload).unwrap();
    payload.put_u8(0xFE);
    frame_bytes(payload.freeze())
}

#[inline]
pub fn is_fml3_login_channel(channel: &str) -> bool {
    channel == FML3_LOGIN_WRAPPER
        || channel == FML3_HANDSHAKE
        || channel.starts_with("fml:")
        || channel.starts_with("forge:")
}

#[inline]
pub fn is_neo_config_channel(channel: &str) -> bool {
    matches!(
        channel,
        MC_REGISTER
            | MC_UNREGISTER
            | MC_NETWORK
            | COMMON_VERSION
            | COMMON_REGISTER
            | NEO_REGISTER
            | NEO_TIER_SORTING
            | NEO_ADVANCED_ADDON
            | NEO_SETUP_FAILED
            | FABRIC_REGISTER
            | FABRIC_UNREGISTER
            | QUILT_INIT_CHANNEL
            | QUILT_REGISTER
            | QUILT_QSL_NETWORKING
            | QUILTED_FABRIC_API_REGISTER
    ) || channel.starts_with("neoforge:")
        || channel.starts_with("fml:")
        || channel.starts_with("forge:")
        || channel.starts_with("quilt:")
        || channel.starts_with("qsl:")
        || channel.starts_with("quilted_fabric_")
}

/// Returns true if the channel name unambiguously identifies a Quilt client.
/// Used by the config-phase sniffer to promote a connection from
/// `Fabric`/`Unknown` to `Quilt`.
#[inline]
pub fn is_quilt_channel(channel: &str) -> bool {
    channel == QUILT_INIT_CHANNEL
        || channel == QUILT_REGISTER
        || channel == QUILT_QSL_NETWORKING
        || channel == QUILTED_FABRIC_API_REGISTER
        || channel.starts_with("quilt:")
        || channel.starts_with("qsl:")
        || channel.starts_with("quilted_fabric_")
}

fn frame_bytes(payload: Bytes) -> Bytes {
    let mut frame = BytesMut::new();
    VarInt(payload.len() as i32).encode(&mut frame).unwrap();
    frame.extend_from_slice(&payload);
    frame.freeze()
}

#[allow(dead_code)]
fn frame_payload_bytes(payload: &[u8]) -> Bytes {
    let mut frame = BytesMut::with_capacity(5 + payload.len());
    VarInt(payload.len() as i32).encode(&mut frame).unwrap();
    frame.extend_from_slice(payload);
    frame.freeze()
}

pub fn log_fml1_packet(channel: &str, body: &[u8], direction: &str, proto: u32) {
    if channel == FML1_HS {
        let disc = body.first().copied().map(FmlDiscriminator::from);
        debug!(proto, direction, discriminator = ?disc, body_bytes = body.len(), "FML1 FML|HS packet");
    } else if channel == FML_REGISTER || channel == FML_UNREGISTER {
        let channels = parse_nul_list(body);
        debug!(proto, direction, channel, registered_channels = ?channels, "FML1 channel list");
    } else {
        debug!(
            proto,
            direction,
            channel,
            body_bytes = body.len(),
            "FML1 play packet"
        );
    }
}

pub fn log_fml3_login_packet(channel: &str, body: &[u8], direction: &str, proto: u32) {
    debug!(
        proto,
        direction,
        channel,
        body_bytes = body.len(),
        "FML3 login-phase relay"
    );
}

pub fn log_neo_config_packet(channel: &str, body: &[u8], direction: &str, proto: u32) {
    match channel {
        NEO_REGISTER | MC_REGISTER | FABRIC_REGISTER | QUILTED_FABRIC_API_REGISTER => {
            let channels = parse_nul_list(body);
            info!(proto, direction, channel, registered_channels = ?channels, "Modloader config-phase channel registration");
        },
        COMMON_VERSION => {
            debug!(proto, direction, "Modloader c:version negotiation");
        },
        COMMON_REGISTER => {
            debug!(proto, direction, "Modloader c:register negotiation");
        },
        QUILT_INIT_CHANNEL | QUILT_REGISTER | QUILT_QSL_NETWORKING => {
            info!(proto, direction, channel, "Quilt modloader plugin channel");
        },
        _ => {
            debug!(
                proto,
                direction,
                channel,
                body_bytes = body.len(),
                "Modloader config-phase packet"
            );
        },
    }
}

/// Inspect a config-phase REGISTER body's NUL-separated channel list and
/// return the strongest modloader signal it contains. Used during the
/// configuration-phase relay to upgrade `Fabric` → `Quilt` when the client
/// advertises Quilt-specific channels.
pub fn detect_kind_from_channel_list(body: &[u8]) -> Option<ModloaderKind> {
    let channels = parse_nul_list(body);
    if channels.iter().any(|c| is_quilt_channel(c)) {
        return Some(ModloaderKind::Quilt);
    }
    if channels
        .iter()
        .any(|c| c.starts_with("neoforge:") || c == NEO_REGISTER || c == NEO_TIER_SORTING)
    {
        return Some(ModloaderKind::NeoForge);
    }
    if channels
        .iter()
        .any(|c| c.starts_with("fabric-") || c == FABRIC_REGISTER)
    {
        return Some(ModloaderKind::Fabric);
    }
    None
}

pub fn parse_nul_list(data: &[u8]) -> Vec<String> {
    data.split(|&b| b == 0)
        .filter(|s| !s.is_empty())
        .map(|s| match std::str::from_utf8(s) {
            Ok(v) => v.to_owned(),
            Err(_) => {
                let lossy = String::from_utf8_lossy(s).into_owned();
                warn!(channel = %lossy, "non-UTF-8 bytes in modloader channel name");
                lossy
            },
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quilt_channels_are_recognized() {
        assert!(is_quilt_channel(QUILT_INIT_CHANNEL));
        assert!(is_quilt_channel("qsl:networking"));
        assert!(is_quilt_channel("quilt:something_custom"));
        assert!(is_quilt_channel("quilted_fabric_api_v1:event"));
        assert!(!is_quilt_channel(
            "fabric-networking-api-v1:register_channel"
        ));
        assert!(!is_quilt_channel("neoforge:register"));
    }

    #[test]
    fn quilt_channels_also_count_as_neo_config_channels() {
        // Quilt channels need to flow through the same config-phase relay
        // code path that Fabric/NeoForge channels use.
        assert!(is_neo_config_channel(QUILT_INIT_CHANNEL));
        assert!(is_neo_config_channel("quilt:foo"));
        assert!(is_neo_config_channel("qsl:networking"));
    }

    #[test]
    fn register_list_with_quilt_channel_is_detected() {
        // A REGISTER body is a NUL-separated list of channel names.
        let mut body = Vec::new();
        body.extend_from_slice(b"fabric-networking-api-v1:event");
        body.push(0);
        body.extend_from_slice(b"qsl:networking");
        body.push(0);
        body.extend_from_slice(b"minecraft:brand");
        assert_eq!(
            detect_kind_from_channel_list(&body),
            Some(ModloaderKind::Quilt)
        );
    }

    #[test]
    fn register_list_with_only_fabric_is_fabric() {
        let mut body = Vec::new();
        body.extend_from_slice(b"fabric-networking-api-v1:event");
        body.push(0);
        body.extend_from_slice(b"minecraft:brand");
        assert_eq!(
            detect_kind_from_channel_list(&body),
            Some(ModloaderKind::Fabric)
        );
    }

    #[test]
    fn quilt_kind_does_not_get_fml_marker_appended() {
        let result = apply_fml_marker("server.example.com", ModloaderKind::Quilt);
        assert_eq!(result, "server.example.com");
        assert!(!result.contains('\0'));
    }
}
