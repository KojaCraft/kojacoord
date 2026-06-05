use dashmap::DashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
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
