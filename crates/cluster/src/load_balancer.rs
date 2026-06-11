//! Routing decisions across the cluster.
//!
//! Reads a snapshot of [`ServiceDiscovery`] and picks the best
//! [`ClusterNode`] to send a new player to. Currently does
//! least-connections; the algorithm is intentionally simple — the
//! per-node backend pool inside each proxy does the finer-grained
//! load shedding.

use crate::discovery::ServiceDiscovery;
use crate::node::ClusterNode;
use std::sync::Arc;
use uuid::Uuid;

pub struct LoadBalancer {
    discovery: Arc<ServiceDiscovery>,
    strategy: LoadBalancingStrategy,
}

#[derive(Debug, Clone, Copy)]
pub enum LoadBalancingStrategy {
    RoundRobin,
    LeastConnections,
    Random,
}

impl LoadBalancer {
    pub fn new(discovery: Arc<ServiceDiscovery>, strategy: LoadBalancingStrategy) -> Self {
        Self {
            discovery,
            strategy,
        }
    }

    pub fn select_node(&self, protocol_version: u32) -> Option<ClusterNode> {
        let available = self.discovery.get_available_nodes();

        let compatible: Vec<_> = available
            .into_iter()
            .filter(|n| {
                n.capabilities
                    .supports_protocol_versions
                    .contains(&protocol_version)
            })
            .collect();

        if compatible.is_empty() {
            return None;
        }

        match self.strategy {
            LoadBalancingStrategy::RoundRobin => self.round_robin(&compatible),
            LoadBalancingStrategy::LeastConnections => self.least_connections(&compatible),
            LoadBalancingStrategy::Random => self.random(&compatible),
        }
    }

    fn round_robin(&self, nodes: &[ClusterNode]) -> Option<ClusterNode> {
        let index = (chrono::Utc::now().timestamp() as usize) % nodes.len();
        nodes.get(index).cloned()
    }

    fn least_connections(&self, nodes: &[ClusterNode]) -> Option<ClusterNode> {
        nodes.iter().min_by_key(|n| n.player_count).cloned()
    }

    fn random(&self, nodes: &[ClusterNode]) -> Option<ClusterNode> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        Uuid::new_v4().hash(&mut hasher);
        let index = (hasher.finish() as usize) % nodes.len();
        nodes.get(index).cloned()
    }

    pub fn set_strategy(&mut self, strategy: LoadBalancingStrategy) {
        self.strategy = strategy;
    }
}
