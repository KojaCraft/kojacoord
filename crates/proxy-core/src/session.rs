use std::net::IpAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionState {
    Handshaking,
    Status,
    Login,
    Configuration,
    Play,
}

pub struct PlayerSession {
    pub uuid: Uuid,
    pub username: String,
    pub client_ip: IpAddr,
    pub protocol_version: u32,
    pub state: ConnectionState,
    pub current_server: Option<String>,
    pub properties: Vec<kojacoord_auth::ProfileProperty>,
    pub locale: Option<String>,
    pub view_distance: Option<u8>,

    pub rank: String,
}

pub type SharedSession = Arc<RwLock<PlayerSession>>;
