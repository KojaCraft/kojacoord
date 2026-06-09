use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProtocolState {
    Handshake,
    Status,
    Login,
    Configuration,
    Play,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    Serverbound,
    Clientbound,
}

#[derive(Debug, Clone)]
pub struct PacketMeta {
    pub id: u8,
    pub name: &'static str,
}

pub struct PacketRegistry {
    map: HashMap<ProtocolState, HashMap<Direction, Vec<(u32, PacketMeta)>>>,
}

impl PacketRegistry {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn register(
        &mut self,
        proto: u32,
        state: ProtocolState,
        dir: Direction,
        name: &'static str,
        id: u8,
    ) {
        let state_map = self.map.entry(state).or_default();
        let dir_vec = state_map.entry(dir).or_default();
        if let Some((_, meta)) = dir_vec
            .iter_mut()
            .find(|(p, meta)| *p == proto && meta.name == name)
        {
            meta.id = id;
        } else {
            dir_vec.push((proto, PacketMeta { id, name }));
        }
    }

    pub fn get_id(
        &self,
        proto: u32,
        state: ProtocolState,
        dir: Direction,
        name: &'static str,
    ) -> Option<u8> {
        let state_map = self.map.get(&state)?;
        let dir_vec = state_map.get(&dir)?;
        dir_vec
            .iter()
            .find(|(p, meta)| *p == proto && meta.name == name)
            .map(|(_, meta)| meta.id)
    }

    /// Look up a packet id for the given protocol, with fallback to the
    /// highest-numbered version that registered the packet at `proto` or
    /// lower. Lets us register an ID once per protocol bump and have it apply
    /// to every subversion in between.
    pub fn get_id_for_version(
        &self,
        proto: u32,
        state: ProtocolState,
        dir: Direction,
        name: &'static str,
    ) -> Option<u8> {
        let state_map = self.map.get(&state)?;
        let dir_vec = state_map.get(&dir)?;

        if let Some((_, meta)) = dir_vec
            .iter()
            .find(|(p, meta)| *p == proto && meta.name == name)
        {
            return Some(meta.id);
        }

        let mut best_proto: Option<u32> = None;
        let mut best_id: Option<u8> = None;
        for (p, meta) in dir_vec {
            if meta.name == name && *p <= proto && best_proto.map_or(true, |bp| *p > bp) {
                best_proto = Some(*p);
                best_id = Some(meta.id);
            }
        }
        best_id
    }

    pub fn get_name_from_id(
        &self,
        proto: u32,
        state: ProtocolState,
        dir: Direction,
        id: u8,
    ) -> Option<&'static str> {
        let state_map = self.map.get(&state)?;
        let dir_vec = state_map.get(&dir)?;

        if let Some((_, meta)) = dir_vec
            .iter()
            .find(|(p, meta)| *p == proto && meta.id == id)
        {
            return Some(meta.name);
        }

        let mut best_proto: Option<u32> = None;
        let mut best_name: Option<&'static str> = None;
        for (p, meta) in dir_vec {
            if meta.id == id && *p <= proto && best_proto.map_or(true, |bp| *p > bp) {
                best_proto = Some(*p);
                best_name = Some(meta.name);
            }
        }
        best_name
    }
}

impl Default for PacketRegistry {
    fn default() -> Self {
        build_default_registry()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Registry tables — IDs verified against https://minecraft.wiki packet pages.
//
// Only the packet names the proxy actually looks up are listed. Registering
// happens at the protocol version where each ID first appeared; subversions in
// between (e.g. 1.9.1/1.9.2/1.9.4 between 1.9=107 and 1.10=210) inherit via
// `get_id_for_version`'s nearest-lower-proto fallback.
//
// Sources used:
//   * https://minecraft.wiki/w/Java_Edition_protocol/Packets (current)
//   * https://minecraft.wiki/w/Java_Edition_protocol_history (per-version diff)
//   * https://minecraft.wiki/w/Java_Edition_protocol — older revisions for
//     pre-1.13 packets.
// ─────────────────────────────────────────────────────────────────────────────

type Entry = (u32, ProtocolState, Direction, &'static str, u8);

/// Pre-netty (1.6.x) — uses single-byte hardcoded IDs, no varint framing.
/// Kept here as a compatibility shim; the actual netty proxy never speaks this
/// state machine but downstream code still queries the names.
const PRE_NETTY: &[Entry] = &[
    (
        78,
        ProtocolState::Login,
        Direction::Serverbound,
        "HandshakeC2S",
        0x02,
    ),
    (
        78,
        ProtocolState::Login,
        Direction::Clientbound,
        "EncryptionKeyRequestS2C",
        0xFD,
    ),
    (
        78,
        ProtocolState::Login,
        Direction::Serverbound,
        "EncryptionKeyResponseC2S",
        0xFC,
    ),
    (
        78,
        ProtocolState::Login,
        Direction::Clientbound,
        "LoginRequestS2C",
        0x01,
    ),
];

/// Handshake state: same for every netty version (1.7+) — single packet.
const HANDSHAKE: &[Entry] = &[(
    4,
    ProtocolState::Handshake,
    Direction::Serverbound,
    "ServerboundHandshake",
    0x00,
)];

/// Status state: stable across every netty version (1.7+).
const STATUS: &[Entry] = &[
    (
        4,
        ProtocolState::Status,
        Direction::Serverbound,
        "ServerboundStatusRequest",
        0x00,
    ),
    (
        4,
        ProtocolState::Status,
        Direction::Serverbound,
        "ServerboundPingRequest",
        0x01,
    ),
    (
        4,
        ProtocolState::Status,
        Direction::Clientbound,
        "ClientboundStatusResponse",
        0x00,
    ),
    (
        4,
        ProtocolState::Status,
        Direction::Clientbound,
        "ClientboundPongResponse",
        0x01,
    ),
];

/// Login state evolution.
///   1.7.x (4/5):   LoginStart 0x00, EncryptionResp 0x01,
///                  Disconnect 0x00, EncryptionReq 0x01, LoginSuccess 0x02
///   1.8 (47):      SetCompression 0x03 added
///   1.13 (393):    LoginPluginRequest 0x04 + LoginPluginResponse 0x02 added
///   1.20.2 (764):  LoginAcknowledged 0x03 added
const LOGIN: &[Entry] = &[
    (
        4,
        ProtocolState::Login,
        Direction::Serverbound,
        "ServerboundLoginStart",
        0x00,
    ),
    (
        4,
        ProtocolState::Login,
        Direction::Serverbound,
        "ServerboundEncryptionResponse",
        0x01,
    ),
    (
        4,
        ProtocolState::Login,
        Direction::Clientbound,
        "ClientboundLoginDisconnect",
        0x00,
    ),
    (
        4,
        ProtocolState::Login,
        Direction::Clientbound,
        "ClientboundEncryptionRequest",
        0x01,
    ),
    (
        4,
        ProtocolState::Login,
        Direction::Clientbound,
        "ClientboundLoginSuccess",
        0x02,
    ),
    (
        47,
        ProtocolState::Login,
        Direction::Clientbound,
        "ClientboundSetCompression",
        0x03,
    ),
    (
        393,
        ProtocolState::Login,
        Direction::Clientbound,
        "ClientboundLoginPluginRequest",
        0x04,
    ),
    (
        393,
        ProtocolState::Login,
        Direction::Serverbound,
        "ServerboundLoginPluginResponse",
        0x02,
    ),
    (
        764,
        ProtocolState::Login,
        Direction::Serverbound,
        "ServerboundLoginAcknowledged",
        0x03,
    ),
];

/// Configuration state — introduced in 1.20.2 (proto 764). 1.20.5+ shifted
/// IDs slightly because new packets were added.
const CONFIGURATION: &[Entry] = &[
    (
        764,
        ProtocolState::Configuration,
        Direction::Clientbound,
        "ClientboundPluginMessage",
        0x00,
    ),
    (
        764,
        ProtocolState::Configuration,
        Direction::Clientbound,
        "ClientboundDisconnect",
        0x01,
    ),
    (
        764,
        ProtocolState::Configuration,
        Direction::Clientbound,
        "FinishConfiguration",
        0x02,
    ),
    (
        764,
        ProtocolState::Configuration,
        Direction::Clientbound,
        "ClientboundKeepAlive",
        0x03,
    ),
    (
        764,
        ProtocolState::Configuration,
        Direction::Clientbound,
        "ClientboundRegistryData",
        0x05,
    ),
    (
        764,
        ProtocolState::Configuration,
        Direction::Serverbound,
        "AcknowledgeFinishConfiguration",
        0x02,
    ),
    // 1.20.5 / 1.21 shifted these by one because "Cookie Request" was inserted at 0x00.
    (
        766,
        ProtocolState::Configuration,
        Direction::Clientbound,
        "ClientboundPluginMessage",
        0x01,
    ),
    (
        766,
        ProtocolState::Configuration,
        Direction::Clientbound,
        "ClientboundDisconnect",
        0x02,
    ),
    (
        766,
        ProtocolState::Configuration,
        Direction::Clientbound,
        "FinishConfiguration",
        0x03,
    ),
    (
        766,
        ProtocolState::Configuration,
        Direction::Clientbound,
        "ClientboundKeepAlive",
        0x04,
    ),
    (
        766,
        ProtocolState::Configuration,
        Direction::Clientbound,
        "ClientboundRegistryData",
        0x07,
    ),
];

/// Play state. The proxy needs this subset for limbo + connection disconnects:
///   ClientboundJoinGame   – 1.6.x–1.18.2; called Login from 1.19+
///   ClientboundLogin      – modern name for JoinGame from 1.19+
///   ClientboundRespawn
///   ClientboundKeepAlive
///   ClientboundChatMessage     – 1.6.x–1.18.2 only
///   ClientboundSystemChat      – 1.19+
///   ClientboundPluginMessage   – the original name through 1.18
///   ClientboundCustomPayload   – alias used by 1.19+ codepaths
///   ClientboundDisconnect
///   ClientboundPlayerAbilities
///   ClientboundPlayerPosition
///   ClientboundSetCarriedItem  – modern; legacy alias `SetHeldItem` registered too
///   ClientboundSound + ClientboundNamedSoundEffect
///   ClientboundBossBar
///   ClientboundLevelChunkWithLight
/// Plus C2S keepalive/chat/plugin-message for relay-time disambiguation.
#[rustfmt::skip]
const PLAY: &[Entry] = &[
    // ── 1.7.10 (proto 5) — netty era begins ──────────────────────────────────
    (5, ProtocolState::Play, Direction::Clientbound, "ClientboundKeepAlive",       0x00),
    (5, ProtocolState::Play, Direction::Clientbound, "ClientboundJoinGame",        0x01),
    (5, ProtocolState::Play, Direction::Clientbound, "ClientboundChatMessage",     0x02),
    (5, ProtocolState::Play, Direction::Clientbound, "ClientboundRespawn",         0x07),
    (5, ProtocolState::Play, Direction::Clientbound, "ClientboundPlayerPosition",  0x08),
    (5, ProtocolState::Play, Direction::Clientbound, "ClientboundSetHeldItem",     0x09),
    (5, ProtocolState::Play, Direction::Clientbound, "ClientboundSetCarriedItem",  0x09),
    (5, ProtocolState::Play, Direction::Clientbound, "ClientboundNamedSoundEffect",0x29),
    (5, ProtocolState::Play, Direction::Clientbound, "ClientboundSound",           0x29),
    (5, ProtocolState::Play, Direction::Clientbound, "ClientboundPlayerAbilities", 0x39),
    (5, ProtocolState::Play, Direction::Clientbound, "ClientboundPluginMessage",   0x3F),
    (5, ProtocolState::Play, Direction::Clientbound, "ClientboundCustomPayload",   0x3F),
    (5, ProtocolState::Play, Direction::Clientbound, "ClientboundDisconnect",      0x40),
    (5, ProtocolState::Play, Direction::Serverbound, "ServerboundKeepAlive",       0x00),
    (5, ProtocolState::Play, Direction::Serverbound, "ServerboundChatMessage",     0x01),
    (5, ProtocolState::Play, Direction::Serverbound, "ServerboundPluginMessage",   0x17),
    (5, ProtocolState::Play, Direction::Serverbound, "ServerboundCustomPayload",   0x17),

    // ── 1.8 (proto 47) — same shape as 1.7.10 for our subset, plus BossBar
    //    didn't exist yet (that's 1.9).
    (47, ProtocolState::Play, Direction::Clientbound, "ClientboundKeepAlive",       0x00),
    (47, ProtocolState::Play, Direction::Clientbound, "ClientboundJoinGame",        0x01),
    (47, ProtocolState::Play, Direction::Clientbound, "ClientboundLoginPlay",       0x01),
    (47, ProtocolState::Play, Direction::Clientbound, "ClientboundChatMessage",     0x02),
    (47, ProtocolState::Play, Direction::Clientbound, "ClientboundRespawn",         0x07),
    (47, ProtocolState::Play, Direction::Clientbound, "ClientboundPlayerPosition",  0x08),
    (47, ProtocolState::Play, Direction::Clientbound, "ClientboundSetHeldItem",     0x09),
    (47, ProtocolState::Play, Direction::Clientbound, "ClientboundSetCarriedItem",  0x09),
    (47, ProtocolState::Play, Direction::Clientbound, "ClientboundNamedSoundEffect",0x29),
    (47, ProtocolState::Play, Direction::Clientbound, "ClientboundSound",           0x29),
    (47, ProtocolState::Play, Direction::Clientbound, "ClientboundPlayerAbilities", 0x39),
    (47, ProtocolState::Play, Direction::Clientbound, "ClientboundPluginMessage",   0x3F),
    (47, ProtocolState::Play, Direction::Clientbound, "ClientboundCustomPayload",   0x3F),
    (47, ProtocolState::Play, Direction::Clientbound, "ClientboundDisconnect",      0x40),

    // ── 1.9 (proto 107) — large renumbering. BossBar added.
    (107, ProtocolState::Play, Direction::Clientbound, "ClientboundKeepAlive",       0x1F),
    (107, ProtocolState::Play, Direction::Clientbound, "ClientboundJoinGame",        0x23),
    (107, ProtocolState::Play, Direction::Clientbound, "ClientboundChatMessage",     0x0F),
    (107, ProtocolState::Play, Direction::Clientbound, "ClientboundRespawn",         0x33),
    (107, ProtocolState::Play, Direction::Clientbound, "ClientboundPlayerPosition",  0x2E),
    (107, ProtocolState::Play, Direction::Clientbound, "ClientboundSetHeldItem",     0x37),
    (107, ProtocolState::Play, Direction::Clientbound, "ClientboundSetCarriedItem",  0x37),
    (107, ProtocolState::Play, Direction::Clientbound, "ClientboundNamedSoundEffect",0x19),
    (107, ProtocolState::Play, Direction::Clientbound, "ClientboundSound",           0x19),
    (107, ProtocolState::Play, Direction::Clientbound, "ClientboundPlayerAbilities", 0x2B),
    (107, ProtocolState::Play, Direction::Clientbound, "ClientboundPluginMessage",   0x18),
    (107, ProtocolState::Play, Direction::Clientbound, "ClientboundCustomPayload",   0x18),
    (107, ProtocolState::Play, Direction::Clientbound, "ClientboundDisconnect",      0x1A),
    (107, ProtocolState::Play, Direction::Clientbound, "ClientboundBossBar",         0x0C),
    (107, ProtocolState::Play, Direction::Serverbound, "ServerboundKeepAlive",       0x0B),
    (107, ProtocolState::Play, Direction::Serverbound, "ServerboundChatMessage",     0x02),
    (107, ProtocolState::Play, Direction::Serverbound, "ServerboundPluginMessage",   0x09),
    (107, ProtocolState::Play, Direction::Serverbound, "ServerboundCustomPayload",   0x09),

    // ── 1.12 (proto 335) — Respawn / PlayerPosition / Abilities shifted.
    (335, ProtocolState::Play, Direction::Clientbound, "ClientboundRespawn",         0x35),
    (335, ProtocolState::Play, Direction::Clientbound, "ClientboundPlayerPosition",  0x2F),
    (335, ProtocolState::Play, Direction::Clientbound, "ClientboundSetHeldItem",     0x3A),
    (335, ProtocolState::Play, Direction::Clientbound, "ClientboundSetCarriedItem",  0x3A),
    (335, ProtocolState::Play, Direction::Clientbound, "ClientboundPlayerAbilities", 0x2C),
    // 1.12.2 (340) is identical to 1.12 for our subset — fallback handles it.

    // ── 1.13 (proto 393) — flattening boundary; many IDs shifted.
    (393, ProtocolState::Play, Direction::Clientbound, "ClientboundKeepAlive",       0x21),
    (393, ProtocolState::Play, Direction::Clientbound, "ClientboundJoinGame",        0x25),
    (393, ProtocolState::Play, Direction::Clientbound, "ClientboundChatMessage",     0x0E),
    (393, ProtocolState::Play, Direction::Clientbound, "ClientboundRespawn",         0x38),
    (393, ProtocolState::Play, Direction::Clientbound, "ClientboundPlayerPosition",  0x32),
    (393, ProtocolState::Play, Direction::Clientbound, "ClientboundSetHeldItem",     0x3D),
    (393, ProtocolState::Play, Direction::Clientbound, "ClientboundSetCarriedItem",  0x3D),
    (393, ProtocolState::Play, Direction::Clientbound, "ClientboundNamedSoundEffect",0x1A),
    (393, ProtocolState::Play, Direction::Clientbound, "ClientboundSound",           0x1A),
    (393, ProtocolState::Play, Direction::Clientbound, "ClientboundPlayerAbilities", 0x2E),
    (393, ProtocolState::Play, Direction::Clientbound, "ClientboundPluginMessage",   0x19),
    (393, ProtocolState::Play, Direction::Clientbound, "ClientboundCustomPayload",   0x19),
    (393, ProtocolState::Play, Direction::Clientbound, "ClientboundDisconnect",      0x1B),
    (393, ProtocolState::Play, Direction::Clientbound, "ClientboundBossBar",         0x0C),
    (393, ProtocolState::Play, Direction::Serverbound, "ServerboundKeepAlive",       0x0E),
    (393, ProtocolState::Play, Direction::Serverbound, "ServerboundChatMessage",     0x02),
    (393, ProtocolState::Play, Direction::Serverbound, "ServerboundPluginMessage",   0x0A),
    (393, ProtocolState::Play, Direction::Serverbound, "ServerboundCustomPayload",   0x0A),

    // ── 1.14 (proto 477) — villages & pillage; major sound rework.
    (477, ProtocolState::Play, Direction::Clientbound, "ClientboundKeepAlive",       0x20),
    (477, ProtocolState::Play, Direction::Clientbound, "ClientboundChatMessage",     0x0E),
    (477, ProtocolState::Play, Direction::Clientbound, "ClientboundRespawn",         0x3A),
    (477, ProtocolState::Play, Direction::Clientbound, "ClientboundPlayerPosition",  0x35),
    (477, ProtocolState::Play, Direction::Clientbound, "ClientboundSetHeldItem",     0x3F),
    (477, ProtocolState::Play, Direction::Clientbound, "ClientboundSetCarriedItem",  0x3F),
    (477, ProtocolState::Play, Direction::Clientbound, "ClientboundNamedSoundEffect",0x52),
    (477, ProtocolState::Play, Direction::Clientbound, "ClientboundSound",           0x52),
    (477, ProtocolState::Play, Direction::Clientbound, "ClientboundPlayerAbilities", 0x32),
    (477, ProtocolState::Play, Direction::Clientbound, "ClientboundPluginMessage",   0x18),
    (477, ProtocolState::Play, Direction::Clientbound, "ClientboundCustomPayload",   0x18),
    (477, ProtocolState::Play, Direction::Clientbound, "ClientboundDisconnect",      0x1A),
    (477, ProtocolState::Play, Direction::Clientbound, "ClientboundBossBar",         0x0D),
    (477, ProtocolState::Play, Direction::Clientbound, "ClientboundLevelChunkWithLight", 0x21),
    (477, ProtocolState::Play, Direction::Serverbound, "ServerboundChatMessage",     0x03),
    (477, ProtocolState::Play, Direction::Serverbound, "ServerboundPluginMessage",   0x0B),
    (477, ProtocolState::Play, Direction::Serverbound, "ServerboundCustomPayload",   0x0B),

    // ── 1.15 (proto 573) — small shifts.
    (573, ProtocolState::Play, Direction::Clientbound, "ClientboundKeepAlive",       0x21),
    (573, ProtocolState::Play, Direction::Clientbound, "ClientboundJoinGame",        0x26),
    (573, ProtocolState::Play, Direction::Clientbound, "ClientboundChatMessage",     0x0F),
    (573, ProtocolState::Play, Direction::Clientbound, "ClientboundRespawn",         0x3B),
    (573, ProtocolState::Play, Direction::Clientbound, "ClientboundPlayerPosition",  0x36),
    (573, ProtocolState::Play, Direction::Clientbound, "ClientboundSetHeldItem",     0x40),
    (573, ProtocolState::Play, Direction::Clientbound, "ClientboundSetCarriedItem",  0x40),
    (573, ProtocolState::Play, Direction::Clientbound, "ClientboundPlayerAbilities", 0x32),
    (573, ProtocolState::Play, Direction::Clientbound, "ClientboundPluginMessage",   0x19),
    (573, ProtocolState::Play, Direction::Clientbound, "ClientboundCustomPayload",   0x19),
    (573, ProtocolState::Play, Direction::Clientbound, "ClientboundDisconnect",      0x1B),

    // ── 1.16 (proto 735) — nether update.
    (735, ProtocolState::Play, Direction::Clientbound, "ClientboundKeepAlive",       0x1F),
    (735, ProtocolState::Play, Direction::Clientbound, "ClientboundJoinGame",        0x24),
    (735, ProtocolState::Play, Direction::Clientbound, "ClientboundChatMessage",     0x0E),
    (735, ProtocolState::Play, Direction::Clientbound, "ClientboundRespawn",         0x39),
    (735, ProtocolState::Play, Direction::Clientbound, "ClientboundPlayerPosition",  0x34),
    (735, ProtocolState::Play, Direction::Clientbound, "ClientboundSetHeldItem",     0x3F),
    (735, ProtocolState::Play, Direction::Clientbound, "ClientboundSetCarriedItem",  0x3F),
    (735, ProtocolState::Play, Direction::Clientbound, "ClientboundNamedSoundEffect",0x50),
    (735, ProtocolState::Play, Direction::Clientbound, "ClientboundSound",           0x50),
    (735, ProtocolState::Play, Direction::Clientbound, "ClientboundPlayerAbilities", 0x30),
    (735, ProtocolState::Play, Direction::Clientbound, "ClientboundPluginMessage",   0x17),
    (735, ProtocolState::Play, Direction::Clientbound, "ClientboundCustomPayload",   0x17),
    (735, ProtocolState::Play, Direction::Clientbound, "ClientboundDisconnect",      0x19),
    (735, ProtocolState::Play, Direction::Clientbound, "ClientboundBossBar",         0x0D),
    (735, ProtocolState::Play, Direction::Clientbound, "ClientboundLevelChunkWithLight", 0x20),
    (735, ProtocolState::Play, Direction::Serverbound, "ServerboundChatMessage",     0x03),
    (735, ProtocolState::Play, Direction::Serverbound, "ServerboundPluginMessage",   0x0B),
    (735, ProtocolState::Play, Direction::Serverbound, "ServerboundCustomPayload",   0x0B),
    // 1.16.5 (754) is identical to 1.16.2 for our subset.

    // ── 1.17 (proto 755) — caves & cliffs part 1.
    (755, ProtocolState::Play, Direction::Clientbound, "ClientboundKeepAlive",       0x21),
    (755, ProtocolState::Play, Direction::Clientbound, "ClientboundJoinGame",        0x26),
    (755, ProtocolState::Play, Direction::Clientbound, "ClientboundChatMessage",     0x0F),
    (755, ProtocolState::Play, Direction::Clientbound, "ClientboundRespawn",         0x3D),
    (755, ProtocolState::Play, Direction::Clientbound, "ClientboundPlayerPosition",  0x38),
    (755, ProtocolState::Play, Direction::Clientbound, "ClientboundSetHeldItem",     0x48),
    (755, ProtocolState::Play, Direction::Clientbound, "ClientboundSetCarriedItem",  0x48),
    (755, ProtocolState::Play, Direction::Clientbound, "ClientboundNamedSoundEffect",0x5C),
    (755, ProtocolState::Play, Direction::Clientbound, "ClientboundSound",           0x5C),
    (755, ProtocolState::Play, Direction::Clientbound, "ClientboundPlayerAbilities", 0x32),
    (755, ProtocolState::Play, Direction::Clientbound, "ClientboundPluginMessage",   0x18),
    (755, ProtocolState::Play, Direction::Clientbound, "ClientboundCustomPayload",   0x18),
    (755, ProtocolState::Play, Direction::Clientbound, "ClientboundDisconnect",      0x1A),

    // ── 1.19 (proto 759) — chat signing; SystemChat introduced.
    (759, ProtocolState::Play, Direction::Clientbound, "ClientboundKeepAlive",       0x1E),
    (759, ProtocolState::Play, Direction::Clientbound, "ClientboundJoinGame",        0x23),
    (759, ProtocolState::Play, Direction::Clientbound, "ClientboundLogin",           0x23),
    (759, ProtocolState::Play, Direction::Clientbound, "ClientboundSystemChat",      0x5F),
    (759, ProtocolState::Play, Direction::Clientbound, "ClientboundRespawn",         0x39),
    (759, ProtocolState::Play, Direction::Clientbound, "ClientboundPlayerPosition",  0x37),
    (759, ProtocolState::Play, Direction::Clientbound, "ClientboundSetHeldItem",     0x47),
    (759, ProtocolState::Play, Direction::Clientbound, "ClientboundSetCarriedItem",  0x47),
    (759, ProtocolState::Play, Direction::Clientbound, "ClientboundSound",           0x5D),
    (759, ProtocolState::Play, Direction::Clientbound, "ClientboundPlayerAbilities", 0x2F),
    (759, ProtocolState::Play, Direction::Clientbound, "ClientboundPluginMessage",   0x15),
    (759, ProtocolState::Play, Direction::Clientbound, "ClientboundCustomPayload",   0x15),
    (759, ProtocolState::Play, Direction::Clientbound, "ClientboundDisconnect",      0x17),
    (759, ProtocolState::Play, Direction::Serverbound, "ServerboundChatMessage",     0x04),
    (759, ProtocolState::Play, Direction::Serverbound, "ServerboundChatCommand",     0x03),
    (759, ProtocolState::Play, Direction::Serverbound, "ServerboundPluginMessage",   0x0C),
    (759, ProtocolState::Play, Direction::Serverbound, "ServerboundCustomPayload",   0x0C),

    // ── 1.19.4 (proto 762).
    (762, ProtocolState::Play, Direction::Clientbound, "ClientboundKeepAlive",       0x23),
    (762, ProtocolState::Play, Direction::Clientbound, "ClientboundJoinGame",        0x28),
    (762, ProtocolState::Play, Direction::Clientbound, "ClientboundLogin",           0x28),
    (762, ProtocolState::Play, Direction::Clientbound, "ClientboundSystemChat",      0x64),
    (762, ProtocolState::Play, Direction::Clientbound, "ClientboundRespawn",         0x41),
    (762, ProtocolState::Play, Direction::Clientbound, "ClientboundPlayerPosition",  0x3C),
    (762, ProtocolState::Play, Direction::Clientbound, "ClientboundSetHeldItem",     0x4D),
    (762, ProtocolState::Play, Direction::Clientbound, "ClientboundSetCarriedItem",  0x4D),
    (762, ProtocolState::Play, Direction::Clientbound, "ClientboundSound",           0x62),
    (762, ProtocolState::Play, Direction::Clientbound, "ClientboundPlayerAbilities", 0x34),
    (762, ProtocolState::Play, Direction::Clientbound, "ClientboundPluginMessage",   0x17),
    (762, ProtocolState::Play, Direction::Clientbound, "ClientboundCustomPayload",   0x17),
    (762, ProtocolState::Play, Direction::Clientbound, "ClientboundDisconnect",      0x1A),

    // ── 1.20.4 (proto 765) — configuration phase fully in play; play ids
    //    shifted because configuration packets occupy low IDs in their own
    //    state, but play state itself didn't change much from 1.20 (763).
    (765, ProtocolState::Play, Direction::Clientbound, "ClientboundKeepAlive",       0x24),
    (765, ProtocolState::Play, Direction::Clientbound, "ClientboundJoinGame",        0x29),
    (765, ProtocolState::Play, Direction::Clientbound, "ClientboundLogin",           0x29),
    (765, ProtocolState::Play, Direction::Clientbound, "ClientboundSystemChat",      0x69),
    (765, ProtocolState::Play, Direction::Clientbound, "ClientboundRespawn",         0x45),
    (765, ProtocolState::Play, Direction::Clientbound, "ClientboundPlayerPosition",  0x3E),
    (765, ProtocolState::Play, Direction::Clientbound, "ClientboundSetHeldItem",     0x51),
    (765, ProtocolState::Play, Direction::Clientbound, "ClientboundSetCarriedItem",  0x51),
    (765, ProtocolState::Play, Direction::Clientbound, "ClientboundSound",           0x66),
    (765, ProtocolState::Play, Direction::Clientbound, "ClientboundPlayerAbilities", 0x36),
    (765, ProtocolState::Play, Direction::Clientbound, "ClientboundPluginMessage",   0x18),
    (765, ProtocolState::Play, Direction::Clientbound, "ClientboundCustomPayload",   0x18),
    (765, ProtocolState::Play, Direction::Clientbound, "ClientboundDisconnect",      0x1B),
    (765, ProtocolState::Play, Direction::Serverbound, "ServerboundChatMessage",     0x05),
    (765, ProtocolState::Play, Direction::Serverbound, "ServerboundChatCommand",     0x04),
    (765, ProtocolState::Play, Direction::Serverbound, "ServerboundKeepAlive",       0x14),
    (765, ProtocolState::Play, Direction::Serverbound, "ServerboundPluginMessage",   0x0F),
    (765, ProtocolState::Play, Direction::Serverbound, "ServerboundCustomPayload",   0x0F),

    // ── 1.21 (proto 767) — most play IDs shifted because cookies & transfers
    //    were added.
    (767, ProtocolState::Play, Direction::Clientbound, "ClientboundKeepAlive",       0x26),
    (767, ProtocolState::Play, Direction::Clientbound, "ClientboundJoinGame",        0x2B),
    (767, ProtocolState::Play, Direction::Clientbound, "ClientboundLogin",           0x2B),
    (767, ProtocolState::Play, Direction::Clientbound, "ClientboundSystemChat",      0x6C),
    (767, ProtocolState::Play, Direction::Clientbound, "ClientboundRespawn",         0x47),
    (767, ProtocolState::Play, Direction::Clientbound, "ClientboundPlayerPosition",  0x40),
    (767, ProtocolState::Play, Direction::Clientbound, "ClientboundSetHeldItem",     0x53),
    (767, ProtocolState::Play, Direction::Clientbound, "ClientboundSetCarriedItem",  0x53),
    (767, ProtocolState::Play, Direction::Clientbound, "ClientboundSound",           0x68),
    (767, ProtocolState::Play, Direction::Clientbound, "ClientboundPlayerAbilities", 0x38),
    (767, ProtocolState::Play, Direction::Clientbound, "ClientboundPluginMessage",   0x19),
    (767, ProtocolState::Play, Direction::Clientbound, "ClientboundCustomPayload",   0x19),
    (767, ProtocolState::Play, Direction::Clientbound, "ClientboundDisconnect",      0x1D),
    (767, ProtocolState::Play, Direction::Serverbound, "ServerboundChatMessage",     0x07),
    (767, ProtocolState::Play, Direction::Serverbound, "ServerboundChatCommand",     0x05),
    (767, ProtocolState::Play, Direction::Serverbound, "ServerboundKeepAlive",       0x18),
    (767, ProtocolState::Play, Direction::Serverbound, "ServerboundPluginMessage",   0x13),
    (767, ProtocolState::Play, Direction::Serverbound, "ServerboundCustomPayload",   0x13),
];

const ALL_TABLES: &[&[Entry]] = &[PRE_NETTY, HANDSHAKE, STATUS, LOGIN, CONFIGURATION, PLAY];

pub fn build_default_registry() -> PacketRegistry {
    let mut r = PacketRegistry::new();
    for table in ALL_TABLES {
        for &(proto, state, dir, name, id) in *table {
            r.register(proto, state, dir, name, id);
        }
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keepalive_v1_12_2_is_0x1f() {
        let r = build_default_registry();
        // 1.12.2 (proto 340) inherits from 1.9 (107) via nearest-lookup —
        // 1.12 (335) didn't change KeepAlive id. Confirms the long-standing
        // 0x00 bug is gone.
        assert_eq!(
            r.get_id_for_version(
                340,
                ProtocolState::Play,
                Direction::Clientbound,
                "ClientboundKeepAlive"
            ),
            Some(0x1F)
        );
    }

    #[test]
    fn play_disconnect_v1_12_2_is_0x1a() {
        let r = build_default_registry();
        assert_eq!(
            r.get_id_for_version(
                340,
                ProtocolState::Play,
                Direction::Clientbound,
                "ClientboundDisconnect"
            ),
            Some(0x1A)
        );
    }

    #[test]
    fn finish_configuration_v1_20_4_is_0x02() {
        let r = build_default_registry();
        assert_eq!(
            r.get_id_for_version(
                765,
                ProtocolState::Configuration,
                Direction::Clientbound,
                "FinishConfiguration"
            ),
            Some(0x02)
        );
    }

    #[test]
    fn subversion_fallback_works() {
        let r = build_default_registry();
        // 1.9.4 (110) inherits from 1.9 (107).
        assert_eq!(
            r.get_id_for_version(
                110,
                ProtocolState::Play,
                Direction::Clientbound,
                "ClientboundJoinGame"
            ),
            Some(0x23)
        );
        // 1.16.5 (754) inherits from 1.16 (735).
        assert_eq!(
            r.get_id_for_version(
                754,
                ProtocolState::Play,
                Direction::Clientbound,
                "ClientboundJoinGame"
            ),
            Some(0x24)
        );
        // 1.21.4 (769) inherits from 1.21 (767).
        assert_eq!(
            r.get_id_for_version(
                769,
                ProtocolState::Play,
                Direction::Clientbound,
                "ClientboundSystemChat"
            ),
            Some(0x6C)
        );
    }

    #[test]
    fn login_success_stable_across_eras() {
        let r = build_default_registry();
        for proto in [5, 47, 340, 393, 762, 765, 767] {
            assert_eq!(
                r.get_id_for_version(
                    proto,
                    ProtocolState::Login,
                    Direction::Clientbound,
                    "ClientboundLoginSuccess"
                ),
                Some(0x02),
                "LoginSuccess should be 0x02 for proto {proto}"
            );
        }
    }

    #[test]
    fn login_acknowledged_only_1_20_2_plus() {
        let r = build_default_registry();
        assert_eq!(
            r.get_id_for_version(
                762,
                ProtocolState::Login,
                Direction::Serverbound,
                "ServerboundLoginAcknowledged"
            ),
            None,
            "1.19.4 has no LoginAcknowledged"
        );
        assert_eq!(
            r.get_id_for_version(
                765,
                ProtocolState::Login,
                Direction::Serverbound,
                "ServerboundLoginAcknowledged"
            ),
            Some(0x03)
        );
    }
}
