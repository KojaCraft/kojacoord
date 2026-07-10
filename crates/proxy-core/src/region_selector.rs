//! Region-aware lobby selection.
//!
//! Picks the lobby backend closest to the connecting player so they
//! don't get bounced to a transatlantic server when their region is
//! online. Region is a coarse `"us-east" / "us-west" / "eu-west" / "asia" /
//! "global"` bucket — the proxy is opinionated about regions on purpose, so
//! the config stays readable.
//!
//! Geolocation is a MaxMind GeoIP2/GeoLite2 lookup via [`GeoIpResolver`],
//! configured with an operator-supplied `.mmdb` file path
//! (`[geoip].database_path`). With no path configured (the zero-config
//! default) every IP resolves to `"global"` — the exact behaviour this
//! replaced (an IPv4-first-octet heuristic) had for any address outside
//! the legacy class-A ranges anyway, so this is a strict accuracy
//! improvement with no config required to keep working.

use std::collections::HashMap;
use std::net::IpAddr;

/// Loads (once, at startup) and queries an optional MaxMind `.mmdb`
/// database to bucket a client IP into a coarse region key.
///
/// A GeoLite2-Country database gives continent/country only, which is
/// enough to bucket into `eu-west`/`asia`/`us-east`. A GeoLite2-**City**
/// database additionally carries coordinates, which lets North American
/// IPs split into `us-east`/`us-west` by longitude — country-level data
/// alone can't make that distinction, so it's treated as `us-east`.
pub struct GeoIpResolver {
    reader: Option<maxminddb::Reader<Vec<u8>>>,
}

impl std::fmt::Debug for GeoIpResolver {
    // `maxminddb::Reader` doesn't implement `Debug`; just report whether a
    // database is loaded.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GeoIpResolver")
            .field("loaded", &self.reader.is_some())
            .finish()
    }
}

impl GeoIpResolver {
    /// Load the `.mmdb` at `database_path`, if any. A missing path, a
    /// missing file, or a file that fails to parse all degrade to "no
    /// resolver" (every IP buckets to `"global"`) rather than failing
    /// proxy startup — region routing is an optimization, not a
    /// correctness requirement.
    pub fn load(database_path: Option<&str>) -> Self {
        let reader = database_path.filter(|p| !p.is_empty()).and_then(|path| {
            match maxminddb::Reader::open_readfile(path) {
                Ok(reader) => {
                    tracing::info!(path, "loaded GeoIP database for region routing");
                    Some(reader)
                },
                Err(e) => {
                    tracing::warn!(
                        path,
                        error = %e,
                        "failed to load GeoIP database — region routing will treat every IP as \"global\""
                    );
                    None
                },
            }
        });
        Self { reader }
    }

    /// No database configured: every IP resolves to `"global"`, matching
    /// this proxy's zero-config default before a GeoIP database existed.
    pub fn disabled() -> Self {
        Self { reader: None }
    }

    /// Bucket a client IP into a region key. Returns `"global"` when no
    /// database is loaded, the IP has no record, or the record carries no
    /// continent code (e.g. reserved/private ranges).
    pub fn region_for(&self, ip: IpAddr) -> String {
        let Some(reader) = &self.reader else {
            return "global".to_string();
        };
        let city = match reader
            .lookup(ip)
            .and_then(|result| result.decode::<maxminddb::geoip2::City>())
        {
            Ok(Some(city)) => city,
            Ok(None) => return "global".to_string(),
            Err(e) => {
                tracing::debug!(%ip, error = %e, "GeoIP lookup failed");
                return "global".to_string();
            },
        };

        match city.continent.code {
            Some("EU") => "eu-west".to_string(),
            Some("AS") | Some("OC") => "asia".to_string(),
            // Africa has no dedicated bucket in this proxy's taxonomy;
            // eu-west is the closest available region.
            Some("AF") => "eu-west".to_string(),
            Some("NA") | Some("SA") => {
                // Split North America east/west by longitude when the
                // database is City-level (carries coordinates); Country-level
                // databases leave `location.longitude` unset, so everything
                // in the Americas defaults to us-east.
                match city.location.longitude {
                    Some(lon) if lon <= -100.0 => "us-west".to_string(),
                    _ => "us-east".to_string(),
                }
            },
            _ => "global".to_string(),
        }
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
    geoip: &GeoIpResolver,
    _region_mappings: &HashMap<String, Vec<String>>,
) -> Option<String> {
    let client_region = geoip.region_for(client_ip);
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

#[cfg(test)]
mod tests {
    use super::*;

    // No fixture .mmdb ships with this repo (MaxMind requires a free account
    // to download GeoLite2), so these only exercise the "no database"
    // degrade path — the continent-bucket `match` itself is a straight
    // lookup table with no branching logic worth a mock for.

    #[test]
    fn disabled_resolver_is_always_global() {
        let resolver = GeoIpResolver::disabled();
        assert_eq!(resolver.region_for("8.8.8.8".parse().unwrap()), "global");
        assert_eq!(
            resolver.region_for("2001:db8::1".parse().unwrap()),
            "global"
        );
    }

    #[test]
    fn missing_path_degrades_to_disabled() {
        let resolver = GeoIpResolver::load(None);
        assert_eq!(resolver.region_for("8.8.8.8".parse().unwrap()), "global");
    }

    #[test]
    fn empty_path_degrades_to_disabled() {
        let resolver = GeoIpResolver::load(Some(""));
        assert_eq!(resolver.region_for("8.8.8.8".parse().unwrap()), "global");
    }

    #[test]
    fn nonexistent_file_degrades_to_disabled_not_a_panic() {
        let resolver = GeoIpResolver::load(Some("/no/such/file.mmdb"));
        assert_eq!(resolver.region_for("8.8.8.8".parse().unwrap()), "global");
    }
}
