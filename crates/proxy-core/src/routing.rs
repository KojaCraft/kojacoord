use std::sync::Arc;

use crate::server::{BackendServer, ServerRegistry};

pub struct RoutingRules {
    pub default_server: String,
}

impl RoutingRules {
    pub fn new(default_server: String) -> Self {
        Self { default_server }
    }

    pub fn select(&self, registry: &ServerRegistry) -> Option<Arc<BackendServer>> {
        if let Some(s) = registry.get(&self.default_server) {
            if s.is_online() {
                return Some(s);
            }
        }
        registry.all().into_iter().find(|s| s.is_online())
    }
}
