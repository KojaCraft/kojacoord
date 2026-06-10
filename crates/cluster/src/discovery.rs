//! Peer registry — the shared view of "who else is in the cluster".
//!
//! Backed by a `DashMap<Uuid, ClusterNode>` so updates from the
//! heartbeat task and reads from the load balancer don't contend on
//! a single lock. Entries that haven't checked in inside the
//! configured liveness window get evicted by the coordinator.

use crate::node::ClusterNode;
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;

pub struct ServiceDiscovery {
    nodes: Arc<DashMap<Uuid, ClusterNode>>,
    local_node_id: Uuid,
}

impl ServiceDiscovery {
    pub fn new(local_node_id: Uuid) -> Self {
        Self {
            nodes: Arc::new(DashMap::new()),
            local_node_id,
        }
    }

    pub fn register_node(&self, node: ClusterNode) {
        let node_id = node.id;
        self.nodes.insert(node_id, node);
        tracing::info!("Registered node: {}", node_id);
    }

    pub fn unregister_node(&self, node_id: Uuid) {
        self.nodes.remove(&node_id);
        tracing::info!("Unregistered node: {}", node_id);
    }

    pub fn update_heartbeat(&self, node_id: Uuid, player_count: usize) {
        if let Some(mut node) = self.nodes.get_mut(&node_id) {
            node.last_heartbeat = chrono::Utc::now();
            node.player_count = player_count;
        }
    }

    pub fn get_node(&self, node_id: Uuid) -> Option<ClusterNode> {
        self.nodes.get(&node_id).map(|n| n.clone())
    }

    pub fn get_healthy_nodes(&self) -> Vec<ClusterNode> {
        self.nodes
            .iter()
            .filter(|n| n.is_healthy())
            .map(|n| n.clone())
            .collect()
    }

    pub fn get_available_nodes(&self) -> Vec<ClusterNode> {
        self.nodes
            .iter()
            .filter(|n| n.can_accept_players())
            .map(|n| n.clone())
            .collect()
    }

    pub fn get_local_node(&self) -> Option<ClusterNode> {
        self.nodes.get(&self.local_node_id).map(|n| n.clone())
    }

    pub fn get_all_nodes(&self) -> Vec<ClusterNode> {
        self.nodes.iter().map(|n| n.clone()).collect()
    }

    pub async fn health_check(&self) {
        let now = chrono::Utc::now();
        let timeout = chrono::Duration::seconds(10);

        let mut to_remove = Vec::new();
        for entry in self.nodes.iter() {
            if now.signed_duration_since(entry.last_heartbeat) > timeout {
                to_remove.push(entry.id);
            }
        }

        for node_id in to_remove {
            self.unregister_node(node_id);
            tracing::warn!("Removed unhealthy node: {}", node_id);
        }
    }
}
