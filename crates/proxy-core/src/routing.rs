//! Backend selection logic.
//!
//! The proxy evaluates a list of [`RouteRule`]s in order against the player's
//! name and connecting IP; the first match's `target` server is used. If no
//! rule matches (or the matched server is offline), the configured default
//! server is tried, then any online server as a last resort.
//!
//! Both matchers are dependency-free:
//!   * `name_glob` — simple case-insensitive glob with `*` wildcards
//!   * `client_cidrs` — IPv4 / IPv6 CIDR strings (e.g. `"10.0.0.0/8"` or
//!     `"2001:db8::/32"`)
//!
//! A rule with neither matcher set matches everything (and so acts as a
//! catch-all if placed last).

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;

use kojacoord_config::{GroupStrategy, ServerGroupConfig};

use crate::region_selector::{select_lobby_by_region, GeoIpResolver};
use crate::server::{BackendServer, ServerRegistry};

/// A single routing rule. All matchers that ARE set must match; any matcher
/// left as `None` / empty is treated as "any". See [`module-level
/// docs`](self).
#[derive(Debug, Clone, Default)]
pub struct RouteRule {
    /// Human-readable label, surfaced in logs and metrics.
    pub label: String,
    /// Case-insensitive glob against the player username. `*` matches any
    /// run of characters. `None` = match any name.
    pub name_glob: Option<String>,
    /// CIDR ranges the client IP must fall within (any one matches). Empty
    /// list = match any IP.
    pub client_cidrs: Vec<String>,
    /// Backend server name (matches `[[servers]].name` in the config).
    pub target: String,
}

#[derive(Debug)]
pub struct RoutingRules {
    pub default_server: String,
    pub rules: Vec<RouteRule>,
    geoip: GeoIpResolver,
    /// Named backend pools, keyed by `ServerGroupConfig.name`. A
    /// `RouteRule.target` / `default_server` that names a group resolves
    /// through [`Self::resolve_target`] instead of a literal server lookup.
    server_groups: HashMap<String, ServerGroupConfig>,
}

impl RoutingRules {
    pub fn new(default_server: String) -> Self {
        Self {
            default_server,
            rules: Vec::new(),
            geoip: GeoIpResolver::disabled(),
            server_groups: HashMap::new(),
        }
    }

    pub fn with_rules(default_server: String, rules: Vec<RouteRule>) -> Self {
        Self {
            default_server,
            rules,
            geoip: GeoIpResolver::disabled(),
            server_groups: HashMap::new(),
        }
    }

    /// Full constructor: region-aware lobby selection backed by a loaded
    /// (or absent) GeoIP database, plus named backend-pool resolution.
    pub fn with_rules_geoip_and_groups(
        default_server: String,
        rules: Vec<RouteRule>,
        geoip: GeoIpResolver,
        server_groups: Vec<ServerGroupConfig>,
    ) -> Self {
        Self {
            default_server,
            rules,
            geoip,
            server_groups: server_groups
                .into_iter()
                .map(|g| (g.name.clone(), g))
                .collect(),
        }
    }

    /// Resolve a `RouteRule.target` / `default_server` value to a live
    /// backend: if `name` names a `[[server_groups]]` entry, pick a member
    /// per its configured strategy; otherwise treat it as a literal
    /// `[[servers]].name` (the pre-existing, still-fully-supported
    /// behaviour — this only adds a lookup, never removes one).
    fn resolve_target(&self, name: &str, registry: &ServerRegistry) -> Option<Arc<BackendServer>> {
        match self.server_groups.get(name) {
            Some(group) => select_from_group(group, registry),
            None => registry.get(name).filter(|s| s.is_online()),
        }
    }

    /// Legacy default-only selection (used when player context isn't known
    /// yet — e.g. for the routing-rule fallback or the first lobby pass).
    pub fn select(&self, registry: &ServerRegistry) -> Option<Arc<BackendServer>> {
        if let Some(s) = self.resolve_target(&self.default_server, registry) {
            return Some(s);
        }
        registry.all().into_iter().find(|s| s.is_online())
    }

    /// Select with region-aware fallback for lobby servers
    pub fn select_with_region(
        &self,
        registry: &ServerRegistry,
        client_ip: Option<IpAddr>,
    ) -> Option<Arc<BackendServer>> {
        // First try normal routing rules
        if let Some(ip) = client_ip {
            // For lobby selection, use region-aware selection
            let available_servers = registry.all();
            let region_mappings = std::collections::HashMap::new();

            if let Some(region_server_name) =
                select_lobby_by_region(ip, &available_servers, &self.geoip, &region_mappings)
            {
                if let Some(server) = registry.get(&region_server_name) {
                    if server.is_online() {
                        tracing::debug!(selected = %region_server_name, "Region-aware lobby selection");
                        return Some(server);
                    }
                }
            }
        }

        // Fallback to normal selection
        self.select(registry)
    }

    /// Pick a backend for the given player. Evaluates [`RouteRule`]s in
    /// order; first match wins. Falls back to [`Self::select`] if no rule
    /// matches or the matched server is offline.
    pub fn select_for(
        &self,
        player_name: &str,
        client_ip: Option<IpAddr>,
        registry: &ServerRegistry,
    ) -> Option<Arc<BackendServer>> {
        for rule in &self.rules {
            if !rule_matches(rule, player_name, client_ip) {
                continue;
            }
            if let Some(s) = self.resolve_target(&rule.target, registry) {
                tracing::debug!(
                    rule = %rule.label,
                    target = %rule.target,
                    player = %player_name,
                    "routing rule matched"
                );
                return Some(s);
            }
            tracing::trace!(
                rule = %rule.label,
                target = %rule.target,
                "routing rule matched but target unavailable; trying next"
            );
        }
        self.select(registry)
    }
}

/// Pick a member of `group` per its configured strategy. Only online,
/// non-full (`BackendServer::is_full`) members are eligible — a full
/// member is exactly what `net::queue` exists to handle via the group's
/// *individual* servers, not by silently overflowing onto another member.
fn select_from_group(
    group: &ServerGroupConfig,
    registry: &ServerRegistry,
) -> Option<Arc<BackendServer>> {
    let candidates: Vec<Arc<BackendServer>> = group
        .members
        .iter()
        .filter_map(|name| registry.get(name))
        .filter(|s| s.is_online() && !s.is_full())
        .collect();

    match group.strategy {
        GroupStrategy::LeastConnections => candidates.into_iter().min_by_key(|s| s.player_count()),
        GroupStrategy::Latency => {
            // Members with no latency reading yet (0 = "unknown", either no
            // successful probe yet or health probing disabled) sort last —
            // prefer a server we've actually measured over an unknown one.
            candidates.into_iter().min_by_key(|s| {
                let latency = s.latency_ms();
                if latency == 0 {
                    u64::MAX
                } else {
                    latency
                }
            })
        },
        GroupStrategy::Weighted => weighted_pick(&candidates),
    }
}

/// Weighted-random pick: each candidate's chance is proportional to its
/// `weight` (default 1 — equal odds). A member with `weight == 0` is never
/// picked. Falls back to a uniform pick if every candidate is weight 0
/// (misconfiguration) rather than returning `None` outright.
fn weighted_pick(candidates: &[Arc<BackendServer>]) -> Option<Arc<BackendServer>> {
    if candidates.is_empty() {
        return None;
    }
    let total_weight: u64 = candidates.iter().map(|s| s.weight as u64).sum();
    if total_weight == 0 {
        return candidates.first().cloned();
    }
    let mut roll = {
        use rand::Rng;
        rand::thread_rng().gen_range(0..total_weight)
    };
    for candidate in candidates {
        let w = candidate.weight as u64;
        if roll < w {
            return Some(candidate.clone());
        }
        roll -= w;
    }
    // Unreachable given the sum/roll invariant above, but fall back to the
    // last candidate rather than panicking if float/int edge cases ever
    // let the loop fall through.
    candidates.last().cloned()
}

fn rule_matches(rule: &RouteRule, player_name: &str, client_ip: Option<IpAddr>) -> bool {
    if let Some(glob) = &rule.name_glob {
        if !glob_match_ci(glob, player_name) {
            return false;
        }
    }
    if !rule.client_cidrs.is_empty() {
        let Some(ip) = client_ip else {
            return false;
        };
        if !rule.client_cidrs.iter().any(|cidr| cidr_matches(cidr, ip)) {
            return false;
        }
    }
    true
}

/// Case-insensitive glob with `*` wildcards. No other metacharacters.
fn glob_match_ci(pattern: &str, name: &str) -> bool {
    let pl = pattern.to_ascii_lowercase();
    let nl = name.to_ascii_lowercase();
    let parts: Vec<&str> = pl.split('*').collect();
    if parts.len() == 1 {
        return pl == nl;
    }
    let mut idx = 0usize;
    // First segment must be a prefix unless the pattern starts with `*`.
    if !parts[0].is_empty() {
        if !nl.starts_with(parts[0]) {
            return false;
        }
        idx = parts[0].len();
    }
    // Last segment must be a suffix unless the pattern ends with `*`.
    let last = parts[parts.len() - 1];
    if !last.is_empty() && !nl[idx..].ends_with(last) {
        return false;
    }
    // Middle segments must appear in order.
    let middle_end = nl.len().saturating_sub(last.len());
    for seg in &parts[1..parts.len() - 1] {
        if seg.is_empty() {
            continue;
        }
        match nl[idx..middle_end].find(*seg) {
            Some(off) => idx += off + seg.len(),
            None => return false,
        }
    }
    true
}

/// CIDR membership check. Accepts `"<ip>/<prefix>"` for either IPv4 or IPv6.
/// Bad input returns `false` rather than panicking. `pub(crate)` so
/// `net::ip_reputation`'s static blocklist can reuse the same matcher
/// instead of re-implementing CIDR parsing.
pub(crate) fn cidr_matches(cidr: &str, ip: IpAddr) -> bool {
    let Some((net_str, prefix_str)) = cidr.split_once('/') else {
        // Bare IP — treat as a /32 or /128 host match.
        if let Ok(parsed) = net_str_parse(cidr) {
            return parsed == ip;
        }
        return false;
    };
    let Ok(prefix) = prefix_str.parse::<u8>() else {
        return false;
    };
    let Ok(net) = net_str_parse(net_str) else {
        return false;
    };
    match (net, ip) {
        (IpAddr::V4(n), IpAddr::V4(host)) => prefix_match_v4(n.octets(), host.octets(), prefix),
        (IpAddr::V6(n), IpAddr::V6(host)) => prefix_match_v6(n.octets(), host.octets(), prefix),
        _ => false,
    }
}

fn net_str_parse(s: &str) -> Result<IpAddr, std::net::AddrParseError> {
    s.parse::<IpAddr>()
}

fn prefix_match_v4(net: [u8; 4], host: [u8; 4], prefix: u8) -> bool {
    if prefix > 32 {
        return false;
    }
    let net_u = u32::from_be_bytes(net);
    let host_u = u32::from_be_bytes(host);
    let mask = if prefix == 0 {
        0
    } else {
        u32::MAX << (32 - prefix)
    };
    (net_u & mask) == (host_u & mask)
}

fn prefix_match_v6(net: [u8; 16], host: [u8; 16], prefix: u8) -> bool {
    if prefix > 128 {
        return false;
    }
    let net_u = u128::from_be_bytes(net);
    let host_u = u128::from_be_bytes(host);
    let mask = if prefix == 0 {
        0
    } else {
        u128::MAX << (128 - prefix)
    };
    (net_u & mask) == (host_u & mask)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn glob_exact_match() {
        assert!(glob_match_ci("Notch", "notch"));
        assert!(!glob_match_ci("Notch", "Steve"));
    }

    #[test]
    fn glob_prefix_suffix() {
        assert!(glob_match_ci("Steve*", "SteveLikesCake"));
        assert!(glob_match_ci("*_test", "alice_test"));
        assert!(!glob_match_ci("Steve*", "Alice"));
    }

    #[test]
    fn glob_anywhere() {
        assert!(glob_match_ci("*pvp*", "MyPvpServer"));
        assert!(!glob_match_ci("*pvp*", "MySurvivalServer"));
    }

    #[test]
    fn cidr_v4_basic() {
        let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 1, 5));
        assert!(cidr_matches("10.0.0.0/8", ip));
        assert!(cidr_matches("10.0.1.0/24", ip));
        assert!(!cidr_matches("10.0.2.0/24", ip));
    }

    #[test]
    fn cidr_v4_zero_prefix_matches_all() {
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1));
        assert!(cidr_matches("0.0.0.0/0", ip));
    }

    #[test]
    fn cidr_v6_basic() {
        let ip: IpAddr = "2001:db8::1".parse().unwrap();
        assert!(cidr_matches("2001:db8::/32", ip));
        assert!(!cidr_matches("2001:beef::/32", ip));
    }

    #[test]
    fn cidr_rejects_cross_family() {
        let v4: IpAddr = "10.0.0.1".parse().unwrap();
        assert!(!cidr_matches("::/0", v4));
    }

    #[test]
    fn rule_matches_default_catchall() {
        let rule = RouteRule {
            label: "default".into(),
            name_glob: None,
            client_cidrs: vec![],
            target: "lobby".into(),
        };
        assert!(rule_matches(&rule, "anyone", None));
    }

    #[test]
    fn rule_matches_combines_with_and() {
        let rule = RouteRule {
            label: "vip".into(),
            name_glob: Some("vip_*".into()),
            client_cidrs: vec!["10.0.0.0/8".into()],
            target: "vip-lobby".into(),
        };
        let internal: IpAddr = "10.1.2.3".parse().unwrap();
        let external: IpAddr = "8.8.8.8".parse().unwrap();
        assert!(rule_matches(&rule, "vip_alice", Some(internal)));
        assert!(!rule_matches(&rule, "vip_alice", Some(external)));
        assert!(!rule_matches(&rule, "alice", Some(internal)));
        // Missing IP context with a CIDR-restricted rule = no match.
        assert!(!rule_matches(&rule, "vip_alice", None));
    }

    #[allow(clippy::too_many_arguments)]
    fn test_backend(
        name: &str,
        weight: u32,
        latency_ms: u64,
        player_count: usize,
        max_players: usize,
    ) -> Arc<BackendServer> {
        use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize};
        Arc::new(BackendServer {
            name: name.to_string(),
            address: "127.0.0.1:25565".parse().unwrap(),
            restricted: false,
            forwarding_override: None,
            player_count: Arc::new(AtomicUsize::new(player_count)),
            online: Arc::new(AtomicBool::new(true)),
            connection_pool: None,
            backend_type: Default::default(),
            compression_threshold: 0,
            cipher_suites: String::new(),
            health_probe_interval_secs: 0,
            health_probe_timeout_secs: 5,
            health_probe_fail_threshold: 3,
            health_fail_count: Arc::new(AtomicU32::new(0)),
            health_unhealthy: Arc::new(AtomicBool::new(false)),
            region: String::new(),
            max_players,
            weight,
            last_latency_ms: Arc::new(AtomicU64::new(latency_ms)),
        })
    }

    fn group(members: &[&str], strategy: GroupStrategy) -> ServerGroupConfig {
        ServerGroupConfig {
            name: "test-group".into(),
            members: members.iter().map(|s| s.to_string()).collect(),
            strategy,
        }
    }

    /// Build a registry directly from pre-built backends, bypassing
    /// `ServerRegistry::register`'s async connection-pool setup — irrelevant
    /// for these pure-selection-logic tests (`connection_pool: None` above).
    fn registry_of(servers: Vec<Arc<BackendServer>>) -> ServerRegistry {
        let registry = ServerRegistry::new();
        for s in servers {
            registry.insert_raw(s);
        }
        registry
    }

    #[test]
    fn select_from_group_least_connections_picks_emptiest_member() {
        let registry = registry_of(vec![
            test_backend("a", 1, 0, 10, 0),
            test_backend("b", 1, 0, 2, 0),
        ]);
        let g = group(&["a", "b"], GroupStrategy::LeastConnections);
        let picked = select_from_group(&g, &registry).unwrap();
        assert_eq!(picked.name, "b");
    }

    #[test]
    fn select_from_group_latency_prefers_measured_and_lower() {
        // Members with 0 latency (unmeasured) must never win over a
        // measured one, and among measured members the lowest wins.
        let registry = registry_of(vec![
            test_backend("a", 1, 0, 0, 0), // unmeasured
            test_backend("b", 1, 50, 0, 0),
            test_backend("c", 1, 10, 0, 0),
        ]);
        let g = group(&["a", "b", "c"], GroupStrategy::Latency);
        let picked = select_from_group(&g, &registry).unwrap();
        assert_eq!(picked.name, "c");
    }

    #[test]
    fn select_from_group_skips_full_members() {
        let registry = registry_of(vec![
            test_backend("full", 1, 0, 5, 5),
            test_backend("ok", 1, 0, 0, 0),
        ]);
        let g = group(&["full", "ok"], GroupStrategy::LeastConnections);
        let picked = select_from_group(&g, &registry).unwrap();
        assert_eq!(picked.name, "ok");
    }

    #[test]
    fn select_from_group_weighted_never_picks_zero_weight_when_alternative_exists() {
        let registry = registry_of(vec![
            test_backend("zero", 0, 0, 0, 0),
            test_backend("some", 5, 0, 0, 0),
        ]);
        let g = group(&["zero", "some"], GroupStrategy::Weighted);
        for _ in 0..20 {
            let picked = select_from_group(&g, &registry).unwrap();
            assert_eq!(picked.name, "some");
        }
    }
}
