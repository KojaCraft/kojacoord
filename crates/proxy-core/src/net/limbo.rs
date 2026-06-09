use kojacoord_protocol::{
    build_default_registry,
    codec::Encode,
    registry::{Direction, PacketRegistry, ProtocolState},
    types::VarInt,
    ProtocolVersion, VersionRegistry,
};
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::time::{interval, Duration};
use uuid::Uuid;

use crate::{
    connection::McStream, error::ConnectionError, modloader, proxy::ProxyState,
    session::SharedSession,
};

lazy_static::lazy_static! {
    static ref REGISTRY: PacketRegistry = build_default_registry();
}

const POLL_INTERVAL: Duration = Duration::from_secs(3);
const KEEPALIVE_INTERVAL: Duration = Duration::from_secs(10);
const BOSSBAR_UUID: &str = "12345678-1234-1234-1234-123456789abc";

const LIMBO_X: f64 = 0.0;
const LIMBO_Y: f64 = 256.0;
const LIMBO_Z: f64 = 0.0;

pub struct LimboHandler<'a> {
    stream: &'a mut McStream,
    state: Arc<ProxyState>,
    session: SharedSession,
    protocol_version: u32,
    compression_threshold: i32,
    ml_kind: modloader::ModloaderKind,

    target_server: Option<String>,
}

impl<'a> LimboHandler<'a> {
    pub fn new(
        stream: &'a mut McStream,
        state: Arc<ProxyState>,
        session: SharedSession,
        protocol_version: u32,
        compression_threshold: i32,
        ml_kind: modloader::ModloaderKind,
    ) -> Self {
        Self {
            stream,
            state,
            session,
            protocol_version,
            compression_threshold,
            ml_kind,
            target_server: None,
        }
    }

    /// Pin limbo to a specific backend (used by live server switching).
    /// Without this, limbo connects to whatever the routing rules currently pick.
    pub fn set_target(&mut self, server: String) {
        self.target_server = Some(server);
    }

    pub async fn run(&mut self) -> Result<TcpStream, ConnectionError> {
        let username = self.session.read().await.username.clone();
        tracing::info!(
            player = %username,
            protocol = self.protocol_version,
            version = ?self.ver(),
            ml_kind = ?self.ml_kind,
            "Entering limbo mode"
        );

        let teleport_id = 1_i32;

        tracing::debug!(player = %username, "Sending JoinGame/Login packet");
        self.send_login_play().await?;

        if self.protocol_version >= 47 {
            tracing::debug!(player = %username, ml_kind = ?self.ml_kind, "Sending modloader brand");
            self.send_plugin_brand().await?;
        }

        tracing::debug!(player = %username, "Sending PlayerAbilities packet");
        self.send_player_abilities().await?;

        tracing::debug!(player = %username, "Sending HeldItemChange packet");
        self.send_held_item_change().await?;

        tracing::debug!(player = %username, "Sending PlayerPosition packet");
        self.send_player_position(teleport_id).await?;

        tracing::debug!(player = %username, "Sending limbo chat message");
        self.send_limbo_chat().await?;

        tracing::debug!(player = %username, "Sending note block sound");
        self.send_note_sound().await?;

        let has_bossbar = self.protocol_version >= 107;
        if has_bossbar {
            tracing::debug!(player = %username, "Sending BossBar add packet");
            self.send_bossbar_add().await?;
        }

        let mut poll = interval(POLL_INTERVAL);
        let mut keepalive = interval(KEEPALIVE_INTERVAL);
        let mut ka_id: i64 = 0;
        let mut poll_count = 0u64;

        tracing::info!(
            player = %username,
            poll_interval_sec = POLL_INTERVAL.as_secs(),
            keepalive_interval_sec = KEEPALIVE_INTERVAL.as_secs(),
            "Limbo loop started"
        );

        loop {
            tokio::select! {
                _ = poll.tick() => {
                    poll_count += 1;
                    tracing::trace!(player = %username, poll_count, "Polling for backend");
                    if let Some(backend) = self.try_connect_backend().await {
                        tracing::info!(
                            player = %username,
                            poll_attempts = poll_count,
                            "Backend online - leaving limbo"
                        );
                        if has_bossbar {
                            tracing::debug!(player = %username, "Sending BossBar remove packet");
                            self.send_bossbar_remove().await?;
                        }
                        tracing::debug!(player = %username, "Sending Respawn to transition out of limbo");
                        self.send_respawn().await?;
                        return Ok(backend);
                    }
                }
                _ = keepalive.tick() => {
                    ka_id = ka_id.wrapping_add(1);
                    tracing::trace!(player = %username, keepalive_id = ka_id, "Sending keepalive");
                    self.send_keepalive(ka_id).await?;
                }
                result = self.read_and_discard() => {
                    match result {
                        Ok(_) => tracing::trace!(player = %username, "Discarded client packet in limbo"),
                        Err(e) => return Err(e),
                    }
                }
            }
        }
    }

    /// Returns the [`ProtocolVersion`] whose typed-packet module limbo should
    /// use for this connection. Routed through `canonical_typed_packet_version`
    /// so every subversion (1.9, 1.10, 1.13, 1.14, …) falls onto one of the
    /// concrete variants the match arms below already handle. Without this,
    /// any modern subversion would silently fall through `_ => Ok(())` and the
    /// client would land in limbo without a JoinGame and time out.
    fn ver(&self) -> ProtocolVersion {
        VersionRegistry::nearest(self.protocol_version)
            .canonical_typed_packet_version()
            .as_protocol_version()
    }

    fn play_id(&self, name: &'static str) -> u8 {
        let id = REGISTRY
            .get_id_for_version(
                self.protocol_version,
                ProtocolState::Play,
                Direction::Clientbound,
                name,
            )
            .unwrap_or_else(|| {
                tracing::warn!(
                    packet_name = name,
                    protocol = self.protocol_version,
                    version = ?self.ver(),
                    "Packet ID not found in registry, using 0xFF"
                );
                0xFF
            });
        tracing::trace!(
            packet_name = name,
            packet_id = id,
            protocol = self.protocol_version,
            "Resolved packet ID"
        );
        id
    }

    async fn send_login_play(&mut self) -> Result<(), ConnectionError> {
        let ver = self.ver();
        tracing::debug!(version = ?ver, protocol = self.protocol_version, "Building JoinGame/Login packet");

        match ver {
            ProtocolVersion::V1_6_4 => Ok(()),
            ProtocolVersion::V1_7_10 | ProtocolVersion::V1_8 => {
                use kojacoord_protocol::versions::v1_8_x::play::ClientboundJoinGame;
                let pid = self.play_id("ClientboundJoinGame");
                tracing::debug!(packet_id = pid, version = ?ver, "Using 1.8 JoinGame format");
                let pkt = ClientboundJoinGame {
                    entity_id: 0,
                    game_mode: 0x03,
                    dimension: 0,
                    difficulty: 0,
                    max_players: 20,
                    level_type: "flat".to_string(),
                    reduced_debug_info: false,
                };
                self.write_play_packet(pid, &pkt).await
            },
            ProtocolVersion::V1_12_2 => {
                use kojacoord_protocol::versions::v1_12_x::play::ClientboundJoinGame;
                let pid = self.play_id("ClientboundJoinGame");
                tracing::debug!(packet_id = pid, version = ?ver, "Using 1.12.2 JoinGame format");
                let pkt = ClientboundJoinGame {
                    entity_id: 0,
                    gamemode: 0x03,
                    dimension: 0,
                    difficulty: 0,
                    max_players: 20,
                    level_type: "flat".to_string(),
                    reduced_debug_info: false,
                };
                self.write_play_packet(pid, &pkt).await
            },
            ProtocolVersion::V1_16_5 => {
                use kojacoord_protocol::versions::v1_16_x::play::ClientboundJoinGame;
                let pid = self.play_id("ClientboundJoinGame");
                tracing::debug!(packet_id = pid, version = ?ver, "Using 1.16.5 JoinGame format");
                let pkt = ClientboundJoinGame {
                    entity_id: 0,
                    is_hardcore: false,
                    game_mode: 3,
                    previous_game_mode: -1,
                    world_names: vec!["minecraft:overworld".to_owned()],
                    dimension: "minecraft:overworld".to_owned(),
                    world_name: "minecraft:overworld".to_owned(),
                    hashed_seed: 0,
                    max_players: VarInt(20),
                    view_distance: VarInt(8),
                    reduced_debug_info: false,
                    enable_respawn_screen: true,
                    is_debug: false,
                    is_flat: true,
                };
                self.write_play_packet(pid, &pkt).await
            },
            ProtocolVersion::V1_19_4 => {
                use kojacoord_protocol::versions::v1_19_x::play::ClientboundLogin;
                let pid = self.play_id("ClientboundLogin");
                tracing::debug!(packet_id = pid, version = ?ver, "Using 1.19.4 Login format");
                let pkt = ClientboundLogin {
                    entity_id: 0,
                    is_hardcore: false,
                    game_mode: 3,
                    previous_game_mode: -1,
                    dimensions: vec!["minecraft:overworld".to_owned()],
                    registry_codec: vec![],
                    dimension_type: "minecraft:overworld".to_owned(),
                    dimension_name: "minecraft:overworld".to_owned(),
                    hashed_seed: 0,
                    max_players: VarInt(20),
                    chunk_radius: VarInt(8),
                    simulation_distance: VarInt(8),
                    reduced_debug_info: false,
                    enable_respawn_screen: true,
                    is_debug: false,
                    is_flat: true,
                    death_location: None,
                };
                self.write_play_packet(pid, &pkt).await
            },
            ProtocolVersion::V1_20_4 => {
                use kojacoord_protocol::versions::v1_20_x::play::ClientboundLogin;
                let pid = self.play_id("ClientboundLogin");
                tracing::debug!(packet_id = pid, version = ?ver, "Using 1.20.4 Login format");
                let pkt = ClientboundLogin {
                    entity_id: 0,
                    is_hardcore: false,
                    dimension_names: vec!["minecraft:overworld".to_owned()],
                    max_players: VarInt(20),
                    view_distance: VarInt(8),
                    simulation_distance: VarInt(8),
                    reduced_debug_info: false,
                    enable_respawn_screen: true,
                    do_limited_crafting: false,
                    dimension_type: VarInt(0),
                    dimension_name: "minecraft:overworld".to_owned(),
                    hashed_seed: 0,
                    game_mode: 3,
                    previous_game_mode: -1,
                    is_debug: false,
                    is_flat: true,
                    death_location: None,
                    portal_cooldown: VarInt(0),
                };
                self.write_play_packet(pid, &pkt).await
            },
            ProtocolVersion::V1_21 => {
                use kojacoord_protocol::versions::v1_21_x::play::ClientboundLogin;
                let pid = self.play_id("ClientboundLogin");
                tracing::debug!(packet_id = pid, version = ?ver, "Using 1.21 Login format");
                let pkt = ClientboundLogin {
                    entity_id: 0,
                    is_hardcore: false,
                    dimension_names: vec!["minecraft:overworld".to_owned()],
                    max_players: VarInt(20),
                    view_distance: VarInt(8),
                    simulation_distance: VarInt(8),
                    reduced_debug_info: false,
                    enable_respawn_screen: true,
                    do_limited_crafting: false,
                    dimension_type: VarInt(0),
                    dimension_name: "minecraft:overworld".to_owned(),
                    hashed_seed: 0,
                    game_mode: 3,
                    previous_game_mode: -1,
                    is_debug: false,
                    is_flat: true,
                    death_location: None,
                    portal_cooldown: VarInt(0),
                    sea_level: VarInt(0),
                };
                self.write_play_packet(pid, &pkt).await
            },
            _ => Ok(()),
        }
    }

    pub async fn send_respawn(&mut self) -> Result<(), ConnectionError> {
        let ver = self.ver();
        tracing::debug!(version = ?ver, "Sending Respawn packet to transition client out of limbo");

        match ver {
            ProtocolVersion::V1_6_4 => {
                use kojacoord_protocol::versions::v1_6_x::play::ClientboundRespawn;
                let pid = self.play_id("ClientboundRespawn");
                if pid == 0xFF {
                    return Ok(());
                }
                let pkt = ClientboundRespawn {
                    dimension: 0,
                    difficulty: 0,
                    gamemode: 0,
                    world_height: 256,
                };
                self.write_play_packet(pid, &pkt).await
            },
            ProtocolVersion::V1_7_10 | ProtocolVersion::V1_8 => {
                use kojacoord_protocol::versions::v1_8_x::play::ClientboundRespawn;
                let pid = self.play_id("ClientboundRespawn");
                if pid == 0xFF {
                    return Ok(());
                }
                let pkt = ClientboundRespawn {
                    dimension: 0,
                    difficulty: 0,
                    game_mode: 0,
                    level_type: "flat".to_string(),
                };
                self.write_play_packet(pid, &pkt).await
            },
            ProtocolVersion::V1_12_2 => {
                use kojacoord_protocol::versions::v1_12_x::play::ClientboundRespawn;
                let pid = self.play_id("ClientboundRespawn");
                if pid == 0xFF {
                    return Ok(());
                }
                let pkt = ClientboundRespawn {
                    dimension: 0,
                    difficulty: 0,
                    game_mode: 0,
                    level_type: "flat".to_string(),
                };
                self.write_play_packet(pid, &pkt).await
            },
            ProtocolVersion::V1_16_5 => {
                use kojacoord_protocol::versions::v1_16_x::play::ClientboundRespawn;
                let pid = self.play_id("ClientboundRespawn");
                if pid == 0xFF {
                    return Ok(());
                }
                let pkt = ClientboundRespawn {
                    dimension: "minecraft:overworld".to_owned(),
                    world_name: "limbo".to_owned(),
                    hashed_seed: 0,
                    game_mode: 0,
                    previous_game_mode: -1,
                    is_debug: false,
                    is_flat: true,
                    copy_metadata: false,
                };
                self.write_play_packet(pid, &pkt).await
            },
            ProtocolVersion::V1_19_4 => {
                use kojacoord_protocol::versions::v1_19_x::play::ClientboundRespawn;
                let pid = self.play_id("ClientboundRespawn");
                if pid == 0xFF {
                    return Ok(());
                }
                let pkt = ClientboundRespawn {
                    dimension_type: "minecraft:overworld".to_owned(),
                    dimension_name: "minecraft:overworld".to_owned(),
                    hashed_seed: 0,
                    game_mode: 0,
                    previous_game_mode: -1,
                    is_debug: false,
                    is_flat: true,
                    data_kept: 0,
                    death_location: None,
                };
                self.write_play_packet(pid, &pkt).await
            },
            ProtocolVersion::V1_20_4 => {
                use kojacoord_protocol::versions::v1_20_x::play::ClientboundRespawn;
                let pid = self.play_id("ClientboundRespawn");
                if pid == 0xFF {
                    return Ok(());
                }
                let pkt = ClientboundRespawn {
                    dimension_type: VarInt(0),
                    dimension_name: "minecraft:overworld".to_owned(),
                    hashed_seed: 0,
                    game_mode: 0,
                    previous_game_mode: -1,
                    is_debug: false,
                    is_flat: true,
                    data_kept: 0,
                    death_location: None,
                    portal_cooldown: VarInt(0),
                };
                self.write_play_packet(pid, &pkt).await
            },
            ProtocolVersion::V1_21 => {
                use kojacoord_protocol::versions::v1_21_x::play::ClientboundRespawn;
                let pid = self.play_id("ClientboundRespawn");
                if pid == 0xFF {
                    return Ok(());
                }
                let pkt = ClientboundRespawn {
                    dimension_type: VarInt(0),
                    dimension_name: "minecraft:overworld".to_owned(),
                    hashed_seed: 0,
                    game_mode: 0,
                    previous_game_mode: -1,
                    is_debug: false,
                    is_flat: true,
                    data_kept: 0,
                    death_location: None,
                    portal_cooldown: VarInt(0),
                    sea_level: VarInt(0),
                };
                self.write_play_packet(pid, &pkt).await
            },
            _ => Ok(()),
        }
    }

    async fn send_plugin_brand(&mut self) -> Result<(), ConnectionError> {
        let ver = self.ver();

        let brand: &str = match self.ml_kind {
            modloader::ModloaderKind::Fml1 | modloader::ModloaderKind::Fml2 => "fml,bukkit",
            modloader::ModloaderKind::Fml3 => "forge",
            modloader::ModloaderKind::NeoForge => "neoforge",
            modloader::ModloaderKind::Fabric => "fabric",
            // Quilt clients accept "fabric" as the brand without complaint —
            // QSL piggybacks on Fabric's brand handshake.
            modloader::ModloaderKind::Quilt => "quilt",
            modloader::ModloaderKind::Unknown | modloader::ModloaderKind::Vanilla => "Kojacoord",
        };

        let brand_bytes = {
            let mut b = bytes::BytesMut::new();
            VarInt(brand.len() as i32).encode(&mut b)?;
            b.extend_from_slice(brand.as_bytes());
            b.to_vec()
        };

        match ver {
            ProtocolVersion::V1_6_4 => Ok(()),
            ProtocolVersion::V1_7_10 | ProtocolVersion::V1_8 => {
                use kojacoord_protocol::versions::v1_8_x::play::ClientboundPluginMessage;
                let pid = self.play_id("ClientboundPluginMessage");
                let pkt = ClientboundPluginMessage {
                    channel: "MC|Brand".to_owned(),
                    data: brand_bytes,
                };
                self.write_play_packet(pid, &pkt).await
            },
            ProtocolVersion::V1_12_2
            | ProtocolVersion::V1_16_5
            | ProtocolVersion::V1_19_4
            | ProtocolVersion::V1_20_4
            | ProtocolVersion::V1_21 => {
                use kojacoord_protocol::versions::v1_20_x::play::ClientboundPluginMessage;
                let pid = self.play_id("ClientboundPluginMessage");
                let pkt = ClientboundPluginMessage {
                    channel: "minecraft:brand".to_owned(),
                    data: brand_bytes,
                };
                self.write_play_packet(pid, &pkt).await
            },
            _ => Ok(()),
        }
    }

    async fn send_player_abilities(&mut self) -> Result<(), ConnectionError> {
        let ver = self.ver();
        tracing::debug!(version = ?ver, "Building PlayerAbilities packet");

        let pid = self.play_id("ClientboundPlayerAbilities");
        if pid == 0xFF {
            return Ok(());
        }

        match ver {
            ProtocolVersion::V1_6_4 => {
                use kojacoord_protocol::versions::v1_6_x::play::ClientboundPlayerAbilities;
                let pkt = ClientboundPlayerAbilities {
                    flags: 0x06,
                    flying_speed: 0.0,
                    walking_speed: 0.0,
                };
                self.write_play_packet(pid, &pkt).await
            },
            ProtocolVersion::V1_7_10 => {
                use kojacoord_protocol::versions::v1_7_x::play::ClientboundPlayerAbilities;
                let pkt = ClientboundPlayerAbilities {
                    flags: 0x06,
                    flying_speed: 0.0,
                    field_of_view_modifier: 0.0,
                };
                self.write_play_packet(pid, &pkt).await
            },
            ProtocolVersion::V1_8 => {
                use kojacoord_protocol::versions::v1_8_x::play::ClientboundPlayerAbilities;
                let pkt = ClientboundPlayerAbilities {
                    flags: 0x06,
                    flying_speed: 0.0,
                    field_of_view_modifier: 0.0,
                };
                self.write_play_packet(pid, &pkt).await
            },
            ProtocolVersion::V1_12_2 => {
                use kojacoord_protocol::versions::v1_12_x::play::ClientboundPlayerAbilities;
                let pkt = ClientboundPlayerAbilities {
                    flags: 0x06,
                    flying_speed: 0.0,
                    walking_speed: 0.0,
                };
                self.write_play_packet(pid, &pkt).await
            },
            ProtocolVersion::V1_16_5 => {
                use kojacoord_protocol::versions::v1_16_x::play::ClientboundPlayerAbilities;
                let pkt = ClientboundPlayerAbilities {
                    flags: 0x06,
                    flying_speed: 0.0,
                    walking_speed: 0.0,
                };
                self.write_play_packet(pid, &pkt).await
            },
            ProtocolVersion::V1_19_4 => {
                use kojacoord_protocol::versions::v1_19_x::play::ClientboundPlayerAbilities;
                let pkt = ClientboundPlayerAbilities {
                    flags: 0x06,
                    flying_speed: 0.0,
                    walking_speed: 0.0,
                };
                self.write_play_packet(pid, &pkt).await
            },
            ProtocolVersion::V1_20_4 => {
                use kojacoord_protocol::versions::v1_20_x::play::ClientboundPlayerAbilities;
                let pkt = ClientboundPlayerAbilities {
                    flags: 0x06,
                    flying_speed: 0.0,
                    walking_speed: 0.0,
                };
                self.write_play_packet(pid, &pkt).await
            },
            ProtocolVersion::V1_21 => {
                use kojacoord_protocol::versions::v1_21_x::play::ClientboundPlayerAbilities;
                let pkt = ClientboundPlayerAbilities {
                    raw: vec![0x06, 0, 0],
                };
                self.write_play_packet(pid, &pkt).await
            },
            _ => Ok(()),
        }
    }

    async fn send_held_item_change(&mut self) -> Result<(), ConnectionError> {
        let ver = self.ver();
        tracing::debug!(version = ?ver, "Building HeldItemChange/SetCarriedItem packet");

        let pid = self.play_id("ClientboundSetCarriedItem");
        if pid == 0xFF {
            return Ok(());
        }

        match ver {
            ProtocolVersion::V1_6_4 => {
                use kojacoord_protocol::versions::v1_6_x::play::ClientboundHeldItemChange;
                let pkt = ClientboundHeldItemChange { slot: 0 };
                self.write_play_packet(pid, &pkt).await
            },
            ProtocolVersion::V1_7_10 | ProtocolVersion::V1_8 => {
                use kojacoord_protocol::versions::v1_8_x::play::ClientboundSetHeldItem;
                let pkt = ClientboundSetHeldItem { slot: 0 };
                self.write_play_packet(pid, &pkt).await
            },
            ProtocolVersion::V1_12_2 => {
                use kojacoord_protocol::versions::v1_12_x::play::ClientboundSetCarriedItem;
                let pkt = ClientboundSetCarriedItem { slot: 0 };
                self.write_play_packet(pid, &pkt).await
            },
            ProtocolVersion::V1_16_5 => {
                use kojacoord_protocol::versions::v1_16_x::play::ClientboundHeldItemChange;
                let pkt = ClientboundHeldItemChange { slot: 0 };
                self.write_play_packet(pid, &pkt).await
            },
            ProtocolVersion::V1_19_4 => {
                use kojacoord_protocol::versions::v1_19_x::play::ClientboundSetCarriedItem;
                let pkt = ClientboundSetCarriedItem { slot: 0 };
                self.write_play_packet(pid, &pkt).await
            },
            ProtocolVersion::V1_20_4 => {
                use kojacoord_protocol::versions::v1_20_x::play::ClientboundSetCarriedItem;
                let pkt = ClientboundSetCarriedItem { slot: 0 };
                self.write_play_packet(pid, &pkt).await
            },
            ProtocolVersion::V1_21 => {
                use kojacoord_protocol::versions::v1_21_x::play::ClientboundSetCarriedItem;
                let pkt = ClientboundSetCarriedItem { raw: vec![0] };
                self.write_play_packet(pid, &pkt).await
            },
            _ => Ok(()),
        }
    }

    async fn send_player_position(&mut self, teleport_id: i32) -> Result<(), ConnectionError> {
        let ver = self.ver();
        tracing::debug!(version = ?ver, teleport_id, "Building PlayerPosition packet");

        match ver {
            ProtocolVersion::V1_6_4 => {
                use kojacoord_protocol::versions::v1_6_x::play::ClientboundPlayerPosition;
                let pid = self.play_id("ClientboundPlayerPosition");
                let pkt = ClientboundPlayerPosition {
                    x: LIMBO_X,
                    y: LIMBO_Y,
                    stance: LIMBO_Y + 1.62,
                    z: LIMBO_Z,
                    yaw: 0.0,
                    pitch: 0.0,
                    on_ground: true,
                };
                self.write_play_packet(pid, &pkt).await
            },
            ProtocolVersion::V1_7_10 | ProtocolVersion::V1_8 => {
                use kojacoord_protocol::versions::v1_8_x::play::ClientboundPlayerPosition;
                let pid = self.play_id("ClientboundPlayerPosition");
                let pkt = ClientboundPlayerPosition {
                    x: LIMBO_X,
                    head_y: LIMBO_Y,
                    z: LIMBO_Z,
                    yaw: 0.0,
                    pitch: 0.0,
                    flags: 0,
                };
                self.write_play_packet(pid, &pkt).await
            },
            ProtocolVersion::V1_12_2 => {
                use kojacoord_protocol::versions::v1_12_x::play::ClientboundPlayerPosition;
                let pid = self.play_id("ClientboundPlayerPosition");
                let pkt = ClientboundPlayerPosition {
                    x: LIMBO_X,
                    y: LIMBO_Y,
                    z: LIMBO_Z,
                    yaw: 0.0,
                    pitch: 0.0,
                    flags: 0,
                    teleport_id: VarInt(teleport_id),
                };
                self.write_play_packet(pid, &pkt).await
            },
            ProtocolVersion::V1_16_5 => {
                use kojacoord_protocol::versions::v1_16_x::play::ClientboundPlayerPosition;
                let pid = self.play_id("ClientboundPlayerPosition");
                let pkt = ClientboundPlayerPosition {
                    x: LIMBO_X,
                    y: LIMBO_Y,
                    z: LIMBO_Z,
                    yaw: 0.0,
                    pitch: 0.0,
                    flags: 0,
                    teleport_id: VarInt(teleport_id),
                };
                self.write_play_packet(pid, &pkt).await
            },
            ProtocolVersion::V1_19_4 => {
                use kojacoord_protocol::versions::v1_19_x::play::ClientboundPlayerPosition;
                let pid = self.play_id("ClientboundPlayerPosition");
                let pkt = ClientboundPlayerPosition {
                    x: LIMBO_X,
                    y: LIMBO_Y,
                    z: LIMBO_Z,
                    yaw: 0.0,
                    pitch: 0.0,
                    flags: 0,
                    teleport_id: VarInt(teleport_id),
                };
                self.write_play_packet(pid, &pkt).await
            },
            ProtocolVersion::V1_20_4 => {
                use kojacoord_protocol::versions::v1_20_x::play::ClientboundPlayerPosition;
                let pid = self.play_id("ClientboundPlayerPosition");
                let pkt = ClientboundPlayerPosition {
                    x: LIMBO_X,
                    y: LIMBO_Y,
                    z: LIMBO_Z,
                    yaw: 0.0,
                    pitch: 0.0,
                    flags: 0,
                    teleport_id: VarInt(teleport_id),
                };
                self.write_play_packet(pid, &pkt).await
            },
            ProtocolVersion::V1_21 => {
                use kojacoord_protocol::versions::v1_21_x::play::ClientboundPlayerPosition;
                let pid = self.play_id("ClientboundPlayerPosition");
                let pkt = ClientboundPlayerPosition {
                    x: LIMBO_X,
                    y: LIMBO_Y,
                    z: LIMBO_Z,
                    yaw: 0.0,
                    pitch: 0.0,
                    flags: 0,
                    teleport_id: VarInt(teleport_id),
                };
                self.write_play_packet(pid, &pkt).await
            },
            _ => Ok(()),
        }
    }

    async fn send_limbo_chat(&mut self) -> Result<(), ConnectionError> {
        let ver = self.ver();
        let msg_json = r#"{"text":"The server is currently offline. You have been placed in limbo and will be connected automatically when it comes back online.","color":"yellow"}"#;

        match ver {
            ProtocolVersion::V1_6_4 => {
                use kojacoord_protocol::versions::v1_6_x::play::ClientboundChatMessage;
                let pid = self.play_id("ClientboundChatMessage");
                if pid == 0xFF {
                    return Ok(());
                }
                let pkt = ClientboundChatMessage {
                    message: msg_json.to_owned(),
                };
                self.write_play_packet(pid, &pkt).await
            },
            ProtocolVersion::V1_7_10 | ProtocolVersion::V1_8 => {
                use kojacoord_protocol::versions::v1_8_x::play::ClientboundChatMessage;
                let pid = self.play_id("ClientboundChatMessage");
                if pid == 0xFF {
                    return Ok(());
                }
                let pkt = ClientboundChatMessage {
                    json_message: msg_json.to_owned(),
                    position: 1,
                };
                self.write_play_packet(pid, &pkt).await
            },
            ProtocolVersion::V1_12_2 => {
                use kojacoord_protocol::versions::v1_12_x::play::ClientboundChatMessage;
                let pid = self.play_id("ClientboundChatMessage");
                if pid == 0xFF {
                    return Ok(());
                }
                let pkt = ClientboundChatMessage {
                    json_message: msg_json.to_owned(),
                    position: 1,
                };
                self.write_play_packet(pid, &pkt).await
            },
            ProtocolVersion::V1_16_5 => {
                use kojacoord_protocol::versions::v1_16_x::play::ClientboundChatMessage;
                let pid = self.play_id("ClientboundChatMessage");
                if pid == 0xFF {
                    return Ok(());
                }
                let pkt = ClientboundChatMessage {
                    json_message: msg_json.to_owned(),
                    position: 1,
                    sender: Uuid::nil(),
                };
                self.write_play_packet(pid, &pkt).await
            },
            ProtocolVersion::V1_19_4 => {
                use kojacoord_protocol::versions::v1_19_x::play::ClientboundSystemChat;
                let pid = self.play_id("ClientboundSystemChat");
                if pid == 0xFF {
                    return Ok(());
                }
                let pkt = ClientboundSystemChat {
                    content: msg_json.to_owned(),
                    overlay: false,
                };
                self.write_play_packet(pid, &pkt).await
            },
            ProtocolVersion::V1_20_4 => {
                use kojacoord_protocol::versions::v1_20_x::play::ClientboundSystemChat;
                let pid = self.play_id("ClientboundSystemChat");
                if pid == 0xFF {
                    return Ok(());
                }
                let pkt = ClientboundSystemChat {
                    content: msg_json.to_owned(),
                    overlay: false,
                };
                self.write_play_packet(pid, &pkt).await
            },
            ProtocolVersion::V1_21 => {
                use kojacoord_protocol::versions::v1_21_x::play::ClientboundSystemChat;
                let pid = self.play_id("ClientboundSystemChat");
                if pid == 0xFF {
                    return Ok(());
                }
                let pkt = ClientboundSystemChat {
                    json_message: msg_json.to_owned(),
                    overlay: false,
                };
                self.write_play_packet(pid, &pkt).await
            },
            _ => Ok(()),
        }
    }

    async fn send_note_sound(&mut self) -> Result<(), ConnectionError> {
        let ver = self.ver();

        match ver {
            ProtocolVersion::V1_6_4 => Ok(()),
            ProtocolVersion::V1_7_10 | ProtocolVersion::V1_8 => {
                use kojacoord_protocol::versions::v1_8_x::play::ClientboundSound;
                let pid = self.play_id("ClientboundSound");
                if pid == 0xFF {
                    return Ok(());
                }
                let pkt = ClientboundSound {
                    sound_name: "records.cat".to_owned(),
                    x: LIMBO_X as i32,
                    y: LIMBO_Y as i32,
                    z: LIMBO_Z as i32,
                    volume: 1.0,
                    pitch: 63,
                };
                self.write_play_packet(pid, &pkt).await
            },
            ProtocolVersion::V1_12_2 => {
                use kojacoord_protocol::versions::v1_12_x::play::ClientboundSound;
                let pid = self.play_id("ClientboundSound");
                if pid == 0xFF {
                    return Ok(());
                }

                let pkt = ClientboundSound {
                    sound_id: VarInt(2257),
                    sound_category: VarInt(2),
                    effect_pos_x: (LIMBO_X * 8.0) as i32,
                    effect_pos_y: (LIMBO_Y * 8.0) as i32,
                    effect_pos_z: (LIMBO_Z * 8.0) as i32,
                    volume: 1.0,
                    pitch: 1.0,
                };
                self.write_play_packet(pid, &pkt).await
            },
            ProtocolVersion::V1_16_5 => {
                use kojacoord_protocol::versions::v1_16_x::play::ClientboundNamedSoundEffect;
                let pid = self.play_id("ClientboundNamedSoundEffect");
                if pid == 0xFF {
                    return Ok(());
                }
                let pkt = ClientboundNamedSoundEffect {
                    sound_name: "minecraft:records.cat".to_owned(),
                    sound_category: VarInt(2),
                    effect_position_x: (LIMBO_X * 8.0) as i32,
                    effect_position_y: (LIMBO_Y * 8.0) as i32,
                    effect_position_z: (LIMBO_Z * 8.0) as i32,
                    volume: 1.0,
                    pitch: 1.0,
                };
                self.write_play_packet(pid, &pkt).await
            },

            ProtocolVersion::V1_19_4 | ProtocolVersion::V1_20_4 | ProtocolVersion::V1_21 => {
                use kojacoord_protocol::versions::v1_21_x::play::ClientboundSound;
                let pid = self.play_id("ClientboundSound");
                if pid == 0xFF {
                    return Ok(());
                }
                let pkt = ClientboundSound {
                    sound_name: "minecraft:music_disc.cat".to_owned(),
                    sound_category: VarInt(2),
                    sound_type: VarInt(0),
                    effect_pos_x: (LIMBO_X * 8.0) as i32,
                    effect_pos_y: (LIMBO_Y * 8.0) as i32,
                    effect_pos_z: (LIMBO_Z * 8.0) as i32,
                    volume: 1.0,
                    pitch: 1.0,
                    seed: 0i64,
                };
                self.write_play_packet(pid, &pkt).await
            },
            _ => Ok(()),
        }
    }

    async fn send_bossbar_add(&mut self) -> Result<(), ConnectionError> {
        use kojacoord_protocol::versions::v1_20_x::play::{BossBarAction, ClientboundBossBar};
        let pid = self.play_id("ClientboundBossBar");
        if pid == 0xFF {
            return Ok(());
        }
        let pkt = ClientboundBossBar {
            uuid: Uuid::parse_str(BOSSBAR_UUID).unwrap(),
            action: BossBarAction::Add {
                title: r#"{"text":"Waiting for server...","color":"yellow"}"#.to_owned(),
                health: 1.0,
                color: VarInt(1),
                division: VarInt(0),
                flags: 0,
            },
        };
        self.write_play_packet(pid, &pkt).await
    }

    async fn send_bossbar_remove(&mut self) -> Result<(), ConnectionError> {
        use kojacoord_protocol::versions::v1_20_x::play::{BossBarAction, ClientboundBossBar};
        let pid = self.play_id("ClientboundBossBar");
        if pid == 0xFF {
            return Ok(());
        }
        let pkt = ClientboundBossBar {
            uuid: Uuid::parse_str(BOSSBAR_UUID).unwrap(),
            action: BossBarAction::Remove,
        };
        self.write_play_packet(pid, &pkt).await
    }

    async fn send_keepalive(&mut self, id: i64) -> Result<(), ConnectionError> {
        let ver = self.ver();
        let pid = self.play_id("ClientboundKeepAlive");
        tracing::trace!(
            packet_id = pid,
            keepalive_id = id,
            version = ?ver,
            "Building KeepAlive packet"
        );

        match ver {
            ProtocolVersion::V1_6_4 => {
                use kojacoord_protocol::versions::v1_6_x::play::ClientboundKeepAlive;
                let pkt = ClientboundKeepAlive {
                    keep_alive_id: id as i32,
                };
                self.write_play_packet(pid, &pkt).await
            },
            ProtocolVersion::V1_7_10 | ProtocolVersion::V1_8 => {
                use kojacoord_protocol::versions::v1_8_x::play::ClientboundKeepAlive;
                let pkt = ClientboundKeepAlive {
                    keep_alive_id: VarInt(id as i32),
                };
                self.write_play_packet(pid, &pkt).await
            },
            ProtocolVersion::V1_12_2 => {
                use kojacoord_protocol::versions::v1_12_x::play::ClientboundKeepAlive;
                let pkt = ClientboundKeepAlive { keep_alive_id: id };
                self.write_play_packet(pid, &pkt).await
            },
            ProtocolVersion::V1_16_5 => {
                use kojacoord_protocol::versions::v1_16_x::play::ClientboundKeepAlive;
                let pkt = ClientboundKeepAlive { keep_alive_id: id };
                self.write_play_packet(pid, &pkt).await
            },
            ProtocolVersion::V1_19_4 => {
                use kojacoord_protocol::versions::v1_19_x::play::ClientboundKeepAlive;
                let pkt = ClientboundKeepAlive { id };
                self.write_play_packet(pid, &pkt).await
            },
            ProtocolVersion::V1_20_4 => {
                use kojacoord_protocol::versions::v1_20_x::play::ClientboundKeepAlive;
                let pkt = ClientboundKeepAlive { id };
                self.write_play_packet(pid, &pkt).await
            },
            ProtocolVersion::V1_21 => {
                use kojacoord_protocol::versions::v1_21_x::play::ClientboundKeepAlive;
                let pkt = ClientboundKeepAlive { keep_alive_id: id };
                self.write_play_packet(pid, &pkt).await
            },
            _ => Ok(()),
        }
    }

    async fn try_connect_backend(&self) -> Option<TcpStream> {
        let username = self.session.read().await.username.clone();

        let backend = match &self.target_server {
            Some(name) => {
                let b = self.state.server_registry.get(name)?;
                if !b.is_online() {
                    return None;
                }
                b
            },
            None => self.state.routing.select(&self.state.server_registry)?,
        };
        tracing::debug!(
            player = %username,
            server = %backend.name,
            address = %backend.address,
            "Trying backend connection (limbo poll)"
        );

        let result = if let Some(pool) = &backend.connection_pool {
            match tokio::time::timeout(Duration::from_millis(1500), pool.acquire()).await {
                Ok(Ok(stream)) => Ok(stream),
                Ok(Err(e)) => Err(e),
                Err(_) => Err(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "pool acquire timed out",
                )),
            }
        } else {
            match tokio::time::timeout(
                Duration::from_millis(1500),
                TcpStream::connect(&backend.address),
            )
            .await
            {
                Ok(Ok(stream)) => Ok(stream),
                Ok(Err(e)) => Err(e),
                Err(_) => Err(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "connection timed out",
                )),
            }
        };

        match result {
            Ok(stream) => {
                tracing::info!(
                    player = %username,
                    server = %backend.name,
                    "Backend connection successful (limbo)"
                );
                Some(stream)
            },
            Err(e) => {
                tracing::trace!(
                    player = %username,
                    server = %backend.name,
                    error = %e,
                    "Backend connection failed (limbo)"
                );
                None
            },
        }
    }

    async fn write_play_packet<T: Encode>(
        &mut self,
        pid: u8,
        pkt: &T,
    ) -> Result<(), ConnectionError> {
        let mut body = bytes::BytesMut::new();
        pkt.encode(&mut body)?;

        let mut p = bytes::BytesMut::new();
        VarInt(pid as i32).encode(&mut p)?;

        let full_type_name = std::any::type_name::<T>();
        let struct_name = full_type_name.split("::").last().unwrap_or(full_type_name);

        tracing::debug!(
            pid = %pid,
            packet_name = %struct_name,
            protocol = self.protocol_version,
            "Sending limbo packet"
        );

        p.extend_from_slice(&body);

        self.write_frame(&p).await
    }

    async fn write_frame(&mut self, payload: &bytes::BytesMut) -> Result<(), ConnectionError> {
        crate::packet_io::write_packet(&mut *self.stream, payload, self.compression_threshold).await
    }

    async fn read_and_discard(&mut self) -> Result<(), ConnectionError> {
        crate::packet_io::read_frame(&mut *self.stream).await?;
        Ok(())
    }
}
