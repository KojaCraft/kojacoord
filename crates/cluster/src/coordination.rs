use crate::discovery::ServiceDiscovery;
use crate::node::{ClusterNode, NodeRole};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

pub struct ClusterCoordinator {
    discovery: Arc<ServiceDiscovery>,
    leader_id: Arc<RwLock<Option<Uuid>>>,
    local_node_id: Uuid,
}

impl ClusterCoordinator {
    pub fn new(discovery: Arc<ServiceDiscovery>, local_node_id: Uuid) -> Self {
        Self {
            discovery,
            leader_id: Arc::new(RwLock::new(None)),
            local_node_id,
        }
    }

    pub async fn initialize(
        &self,
        local_address: SocketAddr,
        max_players: usize,
    ) -> anyhow::Result<()> {
        let local_node = ClusterNode::new(local_address, NodeRole::Standalone, max_players);
        self.discovery.register_node(local_node);

        self.elect_leader().await?;

        Ok(())
    }

    pub async fn elect_leader(&self) -> anyhow::Result<()> {
        let nodes = self.discovery.get_healthy_nodes();

        if nodes.is_empty() {
            *self.leader_id.write().await = None;
            return Ok(());
        }

        let leader = nodes
            .into_iter()
            .min_by_key(|n| n.id)
            .expect("At least one node exists");

        *self.leader_id.write().await = Some(leader.id);

        for node in self.discovery.get_all_nodes() {
            let is_leader = node.id == leader.id;
            let mut updated = node.clone();
            updated.role = if is_leader {
                NodeRole::Leader
            } else {
                NodeRole::Follower
            };
            self.discovery.register_node(updated);
        }

        tracing::info!("Leader elected: {}", leader.id);
        Ok(())
    }

    pub async fn is_leader(&self) -> bool {
        let leader_id = self.leader_id.read().await;
        *leader_id == Some(self.local_node_id)
    }

    pub async fn get_leader(&self) -> Option<ClusterNode> {
        let leader_id = self.leader_id.read().await;
        leader_id.and_then(|id| self.discovery.get_node(id))
    }

    pub async fn step_down(&self) -> anyhow::Result<()> {
        if self.is_leader().await {
            *self.leader_id.write().await = None;
            self.elect_leader().await?;
        }
        Ok(())
    }

    pub async fn on_node_join(&self, node: ClusterNode) -> anyhow::Result<()> {
        self.discovery.register_node(node);
        self.elect_leader().await?;
        Ok(())
    }

    pub async fn on_node_leave(&self, node_id: Uuid) -> anyhow::Result<()> {
        self.discovery.unregister_node(node_id);

        if let Some(leader) = self.get_leader().await {
            if leader.id == node_id {
                self.elect_leader().await?;
            }
        }
        Ok(())
    }
}
