//! Backend server registry.
//!
//! [`BackendServer`] is the runtime view of one entry in
//! `[[servers]]` — address, current player count, health state, pool
//! handles. [`ServerRegistry`] is a `DashMap` keyed by server name so
//! the routing layer can resolve names to `Arc<BackendServer>`
//! without taking a lock.
//!
//! Atomic counters (`player_count`, `health_fail_count`, …) live
//! inside `Arc` so the health probe and the relay can mutate them
//! from different tasks without coordination.

use dashmap::DashMap;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::Arc;

use kojacoord_config::{BackendType, ForwardingMode};

use crate::connection_pool::BackendConnectionPool;

pub struct BackendServer {
    pub name: String,
    pub address: std::net::SocketAddr,
    pub restricted: bool,
    pub forwarding_override: Option<ForwardingMode>,
    pub player_count: Arc<AtomicUsize>,
    pub online: Arc<AtomicBool>,
    pub connection_pool: Option<Arc<BackendConnectionPool>>,
    pub backend_type: BackendType,
    /// Per-server compression threshold. -1 disables compression, 0 uses global default.
    pub compression_threshold: i32,
    /// Cipher suite pinning for TLS connections (if using TLS).
    pub cipher_suites: String,
    /// Health probe interval in seconds (0 = disabled)
    pub health_probe_interval_secs: u64,
    /// Health probe timeout in seconds
    pub health_probe_timeout_secs: u64,
    /// Consecutive failures before marking unhealthy
    pub health_probe_fail_threshold: u32,
    /// Current consecutive failure count
    pub health_fail_count: Arc<AtomicU32>,
    /// Whether the server is marked as unhealthy by health probes
    pub health_unhealthy: Arc<AtomicBool>,
    /// Region for this server (e.g., "us-east", "eu-west", "asia")
    pub region: String,
}

impl BackendServer {
    pub fn player_count(&self) -> usize {
        self.player_count.load(Ordering::Relaxed)
    }

    pub fn is_online(&self) -> bool {
        self.online.load(Ordering::Relaxed)
    }
}

pub struct ServerRegistry {
    servers: Arc<DashMap<String, Arc<BackendServer>>>,
}

impl ServerRegistry {
    pub fn new() -> Self {
        Self {
            servers: Arc::new(DashMap::new()),
        }
    }

    pub async fn register(&self, mut server: BackendServer) {
        let pool = Arc::new(BackendConnectionPool::new(server.address, 10));
        server.connection_pool = Some(pool);
        self.servers.insert(server.name.clone(), Arc::new(server));
    }

    pub fn get(&self, name: &str) -> Option<Arc<BackendServer>> {
        self.servers.get(name).map(|r| r.value().clone())
    }

    pub fn all(&self) -> Vec<Arc<BackendServer>> {
        self.servers.iter().map(|r| r.value().clone()).collect()
    }

    pub fn remove(&self, name: &str) {
        self.servers.remove(name);
    }
}

impl Default for ServerRegistry {
    fn default() -> Self {
        Self::new()
    }
}
