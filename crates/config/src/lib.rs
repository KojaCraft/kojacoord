#![deny(clippy::all)]

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    pub proxy: ProxySection,

    pub listeners: ListenersSection,

    pub forwarding: ForwardingSection,

    #[serde(default)]
    pub anticheat: AnticheatConfig,

    #[serde(default)]
    pub servers: Vec<ServerEntry>,

    #[serde(default)]
    pub database: DatabaseConfig,

    #[serde(default)]
    pub server_management: ServerManagementConfig,

    #[serde(default)]
    pub http_api: HttpApiConfig,

    #[serde(default)]
    pub cluster: ClusterConfig,

    #[serde(default)]
    pub plugins: PluginConfig,

    #[serde(default)]
    pub metrics: MetricsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxySection {
    #[serde(default = "default_bind")]
    pub bind: String,

    #[serde(default = "default_online_mode")]
    pub online_mode: bool,

    #[serde(default = "default_ip_forward")]
    pub ip_forward: bool,

    #[serde(default = "default_compression_threshold")]
    pub compression_threshold: i32,

    #[serde(default = "default_max_players")]
    pub max_players: usize,

    #[serde(default)]
    pub prevent_proxy_connections: bool,

    #[serde(default = "default_session_timeout")]
    pub session_timeout_secs: u64,

    #[serde(default = "default_lobby_name")]
    pub lobby_server_name: String,
    #[serde(default)]
    pub lobby_server_protocol: u32,

    #[serde(default)]
    pub server_id: String,

    #[serde(default)]
    pub eula_accepted: bool,

    #[serde(default = "default_auth_url")]
    pub auth_url: String,
}

impl Default for ProxySection {
    fn default() -> Self {
        Self {
            bind: default_bind(),
            online_mode: default_online_mode(),
            compression_threshold: default_compression_threshold(),
            ip_forward: default_ip_forward(),
            max_players: default_max_players(),
            prevent_proxy_connections: false,
            session_timeout_secs: default_session_timeout(),
            lobby_server_name: default_lobby_name(),
            lobby_server_protocol: 47,
            server_id: generate_server_id(),
            eula_accepted: false,
            auth_url: default_auth_url(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListenersSection {
    #[serde(default = "default_motd")]
    pub motd: String,

    #[serde(default)]
    pub motd_json: Option<serde_json::Value>,

    #[serde(default)]
    pub server_lore: Option<String>,

    #[serde(default)]
    pub tab_list: TabListMode,
}

impl Default for ListenersSection {
    fn default() -> Self {
        Self {
            motd: default_motd(),
            motd_json: None,
            server_lore: None,
            tab_list: TabListMode::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TabListMode {
    #[default]
    GlobalPing,
    ServerPing,
    Hidden,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForwardingSection {
    #[serde(default)]
    pub mode: ForwardingMode,

    #[serde(default)]
    pub velocity_secret: String,
}

impl Default for ForwardingSection {
    fn default() -> Self {
        Self {
            mode: ForwardingMode::None,
            velocity_secret: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ForwardingMode {
    #[default]
    None,
    Bungeecord,
    Velocity,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum BackendType {
    #[default]
    Spigot,
    Forge,
    Hybrid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerEntry {
    pub name: String,

    pub address: String,

    #[serde(default)]
    pub restricted: bool,

    #[serde(default)]
    pub forwarding_override: Option<ForwardingMode>,

    #[serde(default)]
    pub max_players: Option<usize>,

    #[serde(default)]
    pub display_name: Option<String>,

    #[serde(default)]
    pub motd: Option<String>,

    #[serde(default)]
    pub modpack: Option<String>,

    #[serde(default)]
    pub modpack_version: Option<String>,

    #[serde(default)]
    pub game_type: Option<String>,

    #[serde(default)]
    pub backend_protocol: u32,

    #[serde(default)]
    pub backend_type: BackendType,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DatabaseConfig {
    #[serde(default)]
    pub url: String,

    #[serde(default = "default_db_pool_size")]
    pub max_connections: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ClusterConfig {
    #[serde(default)]
    pub enabled: bool,

    #[serde(default)]
    pub node_address: String,

    #[serde(default)]
    pub seed_nodes: Vec<String>,

    #[serde(default = "default_max_players")]
    pub max_players: usize,

    #[serde(default)]
    pub load_balancing_strategy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginConfig {
    #[serde(default)]
    pub enabled: bool,

    #[serde(default)]
    pub plugin_dir: String,

    #[serde(default)]
    pub configs: std::collections::HashMap<String, std::collections::HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MetricsConfig {
    #[serde(default)]
    pub enabled: bool,

    #[serde(default = "default_metrics_bind")]
    pub bind: String,

    #[serde(default)]
    pub retention_hours: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpApiConfig {
    #[serde(default = "bool_true")]
    pub enabled: bool,

    #[serde(default = "default_http_bind")]
    pub bind: String,

    #[serde(default = "default_http_token")]
    pub auth_token: String,
}

impl Default for HttpApiConfig {
    fn default() -> Self {
        Self {
            enabled: bool_true(),
            bind: default_http_bind(),
            auth_token: default_http_token(),
        }
    }
}

fn default_http_bind() -> String {
    "127.0.0.1:8081".into()
}
fn default_http_token() -> String {
    "changeme".into()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServerManagementConfig {
    #[serde(default = "default_management_enabled")]
    pub enabled: bool,

    #[serde(default = "default_management_bind")]
    pub bind: String,

    #[serde(default = "default_management_auth_token")]
    pub auth_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnticheatConfig {
    #[serde(default = "bool_true")]
    pub enabled: bool,

    #[serde(default = "default_max_speed")]
    pub max_speed_blocks_per_tick: f64,

    #[serde(default = "default_max_cps")]
    pub max_cps: u32,

    pub bridge_endpoint: Option<String>,

    #[serde(default = "bool_true")]
    pub store_violations: bool,
}

impl Default for AnticheatConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_speed_blocks_per_tick: default_max_speed(),
            max_cps: default_max_cps(),
            bridge_endpoint: None,
            store_violations: true,
        }
    }
}

fn default_bind() -> String {
    "0.0.0.0:25565".into()
}
fn default_online_mode() -> bool {
    true
}
fn default_ip_forward() -> bool {
    false
}
fn default_compression_threshold() -> i32 {
    256
}
fn default_max_players() -> usize {
    1000
}
fn default_motd() -> String {
    "KojacoordNetwork".into()
}
fn default_session_timeout() -> u64 {
    5
}
fn default_db_pool_size() -> u32 {
    10
}
fn default_max_speed() -> f64 {
    0.7
}
fn default_max_cps() -> u32 {
    20
}
fn bool_true() -> bool {
    true
}

fn default_lobby_name() -> String {
    "lobby".into()
}
fn default_management_enabled() -> bool {
    true
}
fn default_management_bind() -> String {
    "127.0.0.1:25566".into()
}
fn default_management_auth_token() -> String {
    "changeme".into()
}

fn default_auth_url() -> String {
    "https://sessionserver.mojang.com/session/minecraft/hasJoined".into()
}
fn default_metrics_bind() -> String {
    "127.0.0.1:9090".into()
}

fn generate_server_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

impl ProxyConfig {
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, anyhow::Error> {
        use figment::{
            providers::{Env, Format, Toml},
            Figment,
        };
        let config = Figment::new()
            .merge(Toml::file(path.as_ref()))
            .merge(Env::prefixed("KOJA_").global())
            .extract()?;
        Ok(config)
    }
}

pub const DEFAULT_CONFIG: &str = include_str!("../default_config.toml");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_defaults() {
        let cfg: ProxyConfig = toml::from_str(DEFAULT_CONFIG).unwrap();
        assert_eq!(cfg.proxy.bind, "0.0.0.0:25565");
        assert!(cfg.proxy.online_mode);
        assert_eq!(cfg.proxy.compression_threshold, 256);
    }
}
