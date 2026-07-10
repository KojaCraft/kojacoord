//! IP reputation / hosting-provider blocklist.
//!
//! Two layers, checked in `accept_loop` right after `connection_throttle`
//! (before any per-connection work starts):
//!
//!   1. **Static CIDR blocklist** — always available, zero I/O, checked
//!      synchronously against `[ip_reputation].blocklist_cidrs`.
//!   2. **Optional external reputation provider** — an *outbound* HTTP GET
//!      this proxy makes to `[ip_reputation].provider_url` (the proxy
//!      acting as an HTTP client, not exposing an inbound endpoint). Verdicts
//!      are cached per-IP with a TTL so repeat connections don't re-query,
//!      bounded by a short timeout, and **fail open** on any error/timeout —
//!      a slow or unreachable third-party provider must never become a way
//!      to stall or deny-of-service the accept loop.

use std::net::IpAddr;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use kojacoord_config::IpReputationConfig;

use crate::routing::cidr_matches;

/// Cached verdicts are dropped past this count even if their TTL hasn't
/// expired yet, so a burst from a huge IPv6 range can't grow this
/// unboundedly between `evict_stale` sweeps.
const MAX_CACHE_ENTRIES: usize = 500_000;

pub struct IpReputationFilter {
    blocklist_cidrs: Vec<String>,
    provider_url: Option<String>,
    api_key: Option<String>,
    cache_ttl: Duration,
    timeout: Duration,
    cache: DashMap<IpAddr, (bool, Instant)>,
    http: reqwest::Client,
}

impl IpReputationFilter {
    pub fn new(config: &IpReputationConfig, http: reqwest::Client) -> Self {
        Self {
            blocklist_cidrs: config.blocklist_cidrs.clone(),
            provider_url: config
                .provider_url
                .clone()
                .filter(|url| !url.trim().is_empty()),
            api_key: config.api_key.clone().filter(|k| !k.trim().is_empty()),
            cache_ttl: Duration::from_secs(config.cache_ttl_secs.max(1)),
            timeout: Duration::from_millis(config.timeout_ms.max(1)),
            cache: DashMap::new(),
            http,
        }
    }

    /// Returns `Err(reason)` if `ip` should be rejected before any
    /// per-connection work starts.
    pub async fn check(&self, ip: IpAddr) -> Result<(), &'static str> {
        if self
            .blocklist_cidrs
            .iter()
            .any(|cidr| cidr_matches(cidr, ip))
        {
            return Err("IP in static reputation blocklist");
        }

        let Some(url) = &self.provider_url else {
            return Ok(());
        };

        if let Some(cached) = self.cache.get(&ip) {
            let (blocked, checked_at) = *cached;
            if checked_at.elapsed() < self.cache_ttl {
                return if blocked {
                    Err("IP flagged by reputation provider (cached)")
                } else {
                    Ok(())
                };
            }
        }

        let blocked = self.query_provider(url, ip).await;

        if self.cache.len() < MAX_CACHE_ENTRIES {
            self.cache.insert(ip, (blocked, Instant::now()));
        }

        if blocked {
            Err("IP flagged by reputation provider")
        } else {
            Ok(())
        }
    }

    /// Query the configured provider. Any failure — network error, timeout,
    /// non-2xx status, unparseable body — degrades to "not blocked" (fail
    /// open) rather than rejecting the connection or propagating an error.
    async fn query_provider(&self, url: &str, ip: IpAddr) -> bool {
        let mut request = self.http.get(url).query(&[("ip", ip.to_string())]);
        if let Some(key) = &self.api_key {
            request = request.bearer_auth(key);
        }

        let response = match tokio::time::timeout(self.timeout, request.send()).await {
            Ok(Ok(resp)) => resp,
            Ok(Err(e)) => {
                tracing::debug!(%ip, error = %e, "IP reputation provider request failed — failing open");
                return false;
            },
            Err(_) => {
                tracing::debug!(%ip, timeout_ms = ?self.timeout, "IP reputation provider timed out — failing open");
                return false;
            },
        };

        if !response.status().is_success() {
            tracing::debug!(%ip, status = %response.status(), "IP reputation provider returned non-success — failing open");
            return false;
        }

        #[derive(serde::Deserialize)]
        struct Verdict {
            #[serde(default)]
            blocked: bool,
        }

        match response.json::<Verdict>().await {
            Ok(v) => v.blocked,
            Err(e) => {
                tracing::debug!(%ip, error = %e, "IP reputation provider returned unparseable body — failing open");
                false
            },
        }
    }

    /// Drop cached verdicts past their TTL so the cache doesn't grow
    /// unbounded. Call periodically from a background task, same pattern as
    /// `ConnectionThrottle::evict_stale` / `PluginChannelRateLimiter::evict_stale`.
    pub fn evict_stale(&self) {
        self.cache
            .retain(|_, (_, checked_at)| checked_at.elapsed() < self.cache_ttl);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(blocklist: &[&str]) -> IpReputationConfig {
        IpReputationConfig {
            blocklist_cidrs: blocklist.iter().map(|s| s.to_string()).collect(),
            provider_url: None,
            api_key: None,
            cache_ttl_secs: 3600,
            timeout_ms: 300,
        }
    }

    #[tokio::test]
    async fn static_blocklist_rejects_matching_ip() {
        let filter = IpReputationFilter::new(&config(&["10.0.0.0/8"]), reqwest::Client::new());
        assert!(filter.check("10.1.2.3".parse().unwrap()).await.is_err());
        assert!(filter.check("8.8.8.8".parse().unwrap()).await.is_ok());
    }

    #[tokio::test]
    async fn no_provider_configured_allows_everything_not_in_static_list() {
        let filter = IpReputationFilter::new(&config(&[]), reqwest::Client::new());
        assert!(filter.check("1.2.3.4".parse().unwrap()).await.is_ok());
        assert!(filter.check("::1".parse().unwrap()).await.is_ok());
    }

    #[tokio::test]
    async fn unreachable_provider_fails_open() {
        // Port 1 is reserved/unused — this connection attempt fails fast.
        let mut cfg = config(&[]);
        cfg.provider_url = Some("http://127.0.0.1:1/reputation".to_string());
        cfg.timeout_ms = 200;
        let filter = IpReputationFilter::new(&cfg, reqwest::Client::new());
        assert!(filter.check("1.2.3.4".parse().unwrap()).await.is_ok());
    }
}
