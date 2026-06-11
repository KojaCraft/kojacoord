//! Active-passive failover for backend groups.
//!
//! Operators define groups of (primary, standby₁, standby₂, …) in the
//! config. The monitor task watches the health_probe flag on each
//! member and swaps `current_active` to the first healthy standby
//! when the primary goes unhealthy. If `auto_failback = true` and the
//! primary recovers, traffic moves back automatically — otherwise the
//! standby stays active until manually reset.
//!
//! Routing reads `current_active`, so the actual swap is a single
//! string write under the group's write lock. Active connections stay
//! on their original backend until they disconnect — we never
//! force-migrate mid-session.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::server::ServerRegistry;

/// Runtime mirror of one config-defined failover group plus the live
/// `current_active` pointer the routing layer reads. The standbys are
/// ordered — the monitor walks them top-to-bottom looking for a
/// healthy fallback.
#[derive(Debug, Clone)]
pub struct FailoverGroupState {
    pub name: String,
    pub primary: String,
    pub standbys: Vec<String>,
    pub auto_failback: bool,
    /// Whichever backend (primary or one of the standbys) is currently
    /// receiving new sessions. Mutated by the monitor task; read by
    /// the routing layer.
    pub current_active: String,
}

/// Owner of the failover group table + the background monitor loop.
/// Cheap to clone (everything inside is `Arc`-wrapped).
pub struct FailoverManager {
    groups: Arc<RwLock<HashMap<String, FailoverGroupState>>>,
    server_registry: Arc<ServerRegistry>,
}

impl FailoverManager {
    pub fn new(server_registry: Arc<ServerRegistry>) -> Self {
        Self {
            groups: Arc::new(RwLock::new(HashMap::new())),
            server_registry,
        }
    }

    /// Replace the entire group table with one built from config.
    /// `current_active` starts at the primary on every load — operators
    /// who SIGHUP during an active failover will momentarily route to
    /// the (possibly still-unhealthy) primary until the next monitor
    /// tick swings it back.
    pub async fn load_groups(&self, groups: Vec<kojacoord_config::FailoverGroup>) {
        let mut groups_map = self.groups.write().await;
        groups_map.clear();

        for group in groups {
            let name = group.name.clone();
            let primary = group.primary.clone();
            let state = FailoverGroupState {
                name: name.clone(),
                primary: primary.clone(),
                standbys: group.standbys,
                auto_failback: group.auto_failback,
                current_active: primary.clone(),
            };
            groups_map.insert(name.clone(), state);
            tracing::info!(group = %name, primary = %primary, "Loaded failover group");
        }
    }

    /// Routing entry point: returns whichever backend the named group
    /// currently considers active. `None` if the group doesn't exist.
    pub async fn get_active_server(&self, group_name: &str) -> Option<String> {
        let groups = self.groups.read().await;
        groups.get(group_name).map(|g| g.current_active.clone())
    }

    /// Reverse lookup: given a backend name, which (if any) failover
    /// group lists it as primary or standby. Used by management
    /// commands so an operator can ask "what group does `lobby-3`
    /// belong to" without scanning the config by hand.
    pub async fn get_group_for_server(&self, server_name: &str) -> Option<String> {
        let groups = self.groups.read().await;
        for (name, group) in groups.iter() {
            if group.primary == server_name || group.standbys.contains(&server_name.to_string()) {
                return Some(name.clone());
            }
        }
        None
    }

    /// Run one pass over every group, swapping `current_active` if the
    /// active server fell over or (when `auto_failback`) the primary
    /// recovered. The monitor loop calls this every 5 seconds.
    pub async fn check_and_failover(&self) {
        let groups = self.groups.read().await;
        let group_names: Vec<String> = groups.keys().cloned().collect();
        drop(groups);

        for group_name in group_names {
            self.check_group_failover(&group_name).await;
        }
    }

    async fn check_group_failover(&self, group_name: &str) {
        let mut groups = self.groups.write().await;
        let Some(group) = groups.get_mut(group_name) else {
            return;
        };

        let primary_server = self.server_registry.get(&group.primary);
        let current_server = self.server_registry.get(&group.current_active);

        // Check if current active server is unhealthy
        let current_unhealthy = current_server
            .map(|s| {
                s.health_unhealthy
                    .load(std::sync::atomic::Ordering::Relaxed)
            })
            .unwrap_or(false);

        // Check if primary is healthy
        let primary_healthy = primary_server
            .map(|s| {
                !s.health_unhealthy
                    .load(std::sync::atomic::Ordering::Relaxed)
            })
            .unwrap_or(false);

        if current_unhealthy {
            // Current server is unhealthy, try to failover
            if group.current_active == group.primary {
                // Primary is unhealthy, fail to first standby
                if let Some(standby) = group.standbys.first() {
                    let standby_server = self.server_registry.get(standby);
                    if standby_server.is_some() {
                        let standby_healthy = standby_server
                            .map(|s| {
                                !s.health_unhealthy
                                    .load(std::sync::atomic::Ordering::Relaxed)
                            })
                            .unwrap_or(false);

                        if standby_healthy {
                            group.current_active = standby.clone();
                            tracing::warn!(
                                group = %group_name,
                                from = %group.primary,
                                to = %standby,
                                "Failover: primary unhealthy, switched to standby"
                            );
                        }
                    }
                }
            } else {
                // Current standby is unhealthy, try next standby
                if let Some(current_idx) = group
                    .standbys
                    .iter()
                    .position(|s| s == &group.current_active)
                {
                    // Try next standby
                    for standby in group.standbys.iter().skip(current_idx + 1) {
                        let standby_server = self.server_registry.get(standby);
                        if standby_server.is_some() {
                            let standby_healthy = standby_server
                                .map(|s| {
                                    !s.health_unhealthy
                                        .load(std::sync::atomic::Ordering::Relaxed)
                                })
                                .unwrap_or(false);

                            if standby_healthy {
                                group.current_active = standby.clone();
                                tracing::warn!(
                                    group = %group_name,
                                    from = %group.current_active,
                                    to = %standby,
                                    "Failover: current standby unhealthy, switched to next standby"
                                );
                                break;
                            }
                        }
                    }
                }
            }
        } else if group.auto_failback && group.current_active != group.primary && primary_healthy {
            // Auto-failback to primary if enabled and primary is healthy
            group.current_active = group.primary.clone();
            tracing::info!(
                group = %group_name,
                to = %group.primary,
                "Auto-failback: primary recovered, switched back to primary"
            );
        }
    }

    /// Spawn the every-5-seconds monitor loop. Runs for the lifetime
    /// of the proxy; never exits.
    pub fn start_monitoring(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
            loop {
                interval.tick().await;
                self.check_and_failover().await;
            }
        });
    }
}
