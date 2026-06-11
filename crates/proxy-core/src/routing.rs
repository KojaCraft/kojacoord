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

use std::net::IpAddr;
use std::sync::Arc;

use crate::region_selector::select_lobby_by_region;
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
}

impl RoutingRules {
    pub fn new(default_server: String) -> Self {
        Self {
            default_server,
            rules: Vec::new(),
        }
    }

    pub fn with_rules(default_server: String, rules: Vec<RouteRule>) -> Self {
        Self {
            default_server,
            rules,
        }
    }

    /// Legacy default-only selection (used when player context isn't known
    /// yet — e.g. for the routing-rule fallback or the first lobby pass).
    pub fn select(&self, registry: &ServerRegistry) -> Option<Arc<BackendServer>> {
        select_default_then_any(&self.default_server, registry)
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
                select_lobby_by_region(ip, &available_servers, &region_mappings)
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
            if let Some(s) = registry.get(&rule.target) {
                if s.is_online() {
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
                    "routing rule matched but target offline; trying next"
                );
            }
        }
        self.select(registry)
    }
}

fn select_default_then_any(default: &str, registry: &ServerRegistry) -> Option<Arc<BackendServer>> {
    if let Some(s) = registry.get(default) {
        if s.is_online() {
            return Some(s);
        }
    }
    registry.all().into_iter().find(|s| s.is_online())
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
/// Bad input returns `false` rather than panicking.
fn cidr_matches(cidr: &str, ip: IpAddr) -> bool {
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
}
