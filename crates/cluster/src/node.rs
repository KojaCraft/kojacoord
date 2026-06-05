use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterNode {
    pub id: Uuid,
    pub address: SocketAddr,
    pub role: NodeRole,
    pub state: NodeState,
    pub player_count: usize,
    pub last_heartbeat: chrono::DateTime<chrono::Utc>,
    pub capabilities: NodeCapabilities,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeRole {
    Leader,
    Follower,
    Standalone,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeState {
    Starting,
    Ready,
    Degraded,
    Draining,
    ShuttingDown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeCapabilities {
    pub max_players: usize,
    pub supports_protocol_versions: Vec<u32>,
    pub supports_mods: bool,
}

impl ClusterNode {
    pub fn new(address: SocketAddr, role: NodeRole, max_players: usize) -> Self {
        Self {
            id: Uuid::new_v4(),
            address,
            role,
            state: NodeState::Starting,
            player_count: 0,
            last_heartbeat: chrono::Utc::now(),
            capabilities: NodeCapabilities {
                max_players,
                supports_protocol_versions: vec![
                    47, 107, 340, 393, 401, 477, 573, 759, 761, 762, 763,
                ],
                supports_mods: true,
            },
        }
    }

    pub fn is_healthy(&self) -> bool {
        let timeout = chrono::Duration::seconds(5);
        chrono::Utc::now().signed_duration_since(self.last_heartbeat) < timeout
            && matches!(self.state, NodeState::Ready | NodeState::Degraded)
    }

    pub fn can_accept_players(&self) -> bool {
        self.is_healthy() && self.player_count < self.capabilities.max_players
    }
}
