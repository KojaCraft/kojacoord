//! Region-aware lobby selection.
//!
//! Picks the lobby backend closest to the connecting player so they
//! don't get bounced to a transatlantic server when their region is
//! online. Region is a coarse `"us-east" / "eu-west" / "asia" / "global"`
//! bucket — the proxy is opinionated about regions on purpose, so the
//! config stays readable.
//!
//! Geolocation here is a placeholder: a 4-bucket IPv4-octet heuristic
//! that's wrong for anywhere outside the legacy class-A allocations
//! (CGNAT, IPv6, BYOIP, anything moved through ARIN transfers). Swap
//! [`get_region_from_ip`] for a MaxMind GeoLite reader before treating
//! the routing decisions as authoritative.

use std::collections::HashMap;
use std::net::IpAddr;

/// Bucket a client IP into a region key. Returns `"global"` when the IP
/// doesn't match a known bucket — caller treats that as "anything goes".
///
/// PLACEHOLDER: bands match the historical class-A allocations and
/// won't survive contact with reality. See module docs.
pub fn get_region_from_ip(ip: IpAddr) -> String {
    match ip {
        IpAddr::V4(ipv4) => {
            let first = ipv4.octets()[0];
            if (1..=126).contains(&first) {
                "us-east".to_string()
            } else if (128..=191).contains(&first) {
                "eu-west".to_string()
            } else if (192..=223).contains(&first) {
                "asia".to_string()
            } else {
                "global".to_string()
            }
        },
        // IPv6 has no useful per-octet heuristic; treat as region-less.
        IpAddr::V6(_) => "global".to_string(),
    }
}

/// Preferred region + ordered fallbacks. Fallback ordering is opinionated
/// (US-East → US-West → EU → Asia, etc.) and tuned so cross-continental
/// hops are taken last.
pub struct RegionPriority {
    pub preferred: String,
    pub fallbacks: Vec<String>,
}

impl RegionPriority {
    pub fn new(preferred: String) -> Self {
        let fallbacks = match preferred.as_str() {
            "us-east" => vec![
                "us-west".to_string(),
                "eu-west".to_string(),
                "asia".to_string(),
            ],
            "us-west" => vec![
                "us-east".to_string(),
                "eu-west".to_string(),
                "asia".to_string(),
            ],
            "eu-west" => vec![
                "us-east".to_string(),
                "asia".to_string(),
                "us-west".to_string(),
            ],
            "asia" => vec![
                "us-east".to_string(),
                "eu-west".to_string(),
                "us-west".to_string(),
            ],
            _ => vec![
                "us-east".to_string(),
                "eu-west".to_string(),
                "asia".to_string(),
            ],
        };
        Self {
            preferred,
            fallbacks,
        }
    }
}

/// Pick the lobby with the fewest players in the preferred region,
/// falling back through [`RegionPriority::fallbacks`], then any healthy
/// server, then the first server registered. `_region_mappings` is
/// reserved for a future operator-supplied override table.
pub fn select_lobby_by_region(
    client_ip: IpAddr,
    available_servers: &[std::sync::Arc<crate::server::BackendServer>],
    _region_mappings: &HashMap<String, Vec<String>>,
) -> Option<String> {
    let client_region = get_region_from_ip(client_ip);
    let priority = RegionPriority::new(client_region);

    let is_healthy = |s: &std::sync::Arc<crate::server::BackendServer>| {
        !s.health_unhealthy
            .load(std::sync::atomic::Ordering::Relaxed)
    };

    // Preferred region first; "global" matches every server.
    let in_region: Vec<&std::sync::Arc<crate::server::BackendServer>> = available_servers
        .iter()
        .filter(|s| priority.preferred == "global" || s.region == priority.preferred)
        .filter(|s| is_healthy(s))
        .collect();
    if let Some(best) = in_region.iter().min_by_key(|s| s.player_count()) {
        return Some(best.name.clone());
    }

    for fallback in &priority.fallbacks {
        let in_fallback: Vec<&std::sync::Arc<crate::server::BackendServer>> = available_servers
            .iter()
            .filter(|s| &s.region == fallback)
            .filter(|s| is_healthy(s))
            .collect();
        if let Some(best) = in_fallback.iter().min_by_key(|s| s.player_count()) {
            return Some(best.name.clone());
        }
    }

    // Last resort: any healthy server, otherwise anything at all.
    let healthy: Vec<&std::sync::Arc<crate::server::BackendServer>> =
        available_servers.iter().filter(|s| is_healthy(s)).collect();
    if let Some(best) = healthy.iter().min_by_key(|s| s.player_count()) {
        return Some(best.name.clone());
    }
    available_servers.first().map(|s| s.name.clone())
}
