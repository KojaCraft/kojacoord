#![deny(clippy::all)]

pub mod coordination;
pub mod discovery;
pub mod load_balancer;
pub mod node;

pub use coordination::ClusterCoordinator;
pub use discovery::ServiceDiscovery;
pub use load_balancer::LoadBalancer;
pub use node::{ClusterNode, NodeRole, NodeState};
