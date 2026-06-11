//! Multi-node proxy clustering.
//!
//! When `[cluster] enabled = true`, proxies discover each other,
//! elect a leader, and share player counts so routing decisions can
//! balance load across the fleet. `coordination` is the leader
//! election + heartbeat loop, `discovery` is the gossip layer,
//! `load_balancer` picks targets given a snapshot of node health, and
//! `node` is the per-peer state record.
//!
//! Cluster mode is optional — single-node deployments ignore this
//! crate entirely.

#![deny(clippy::all)]

pub mod coordination;
pub mod discovery;
pub mod load_balancer;
pub mod node;

pub use coordination::ClusterCoordinator;
pub use discovery::ServiceDiscovery;
pub use load_balancer::LoadBalancer;
pub use node::{ClusterNode, NodeRole, NodeState};
