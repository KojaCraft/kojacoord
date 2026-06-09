use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// New connections allowed per IP before a temporary ban.
/// Default tuned to tolerate shared/CGNAT addresses; operators
/// can override via `proxy.max_connections_per_ip`. `0` disables throttling.
const DEFAULT_MAX_CONNECTIONS_PER_IP: u32 = 8;
const REFILL_RATE_PER_SEC: f32 = 2.0;
const CAPACITY: f32 = 8.0;
const TEMP_BAN_DURATION: Duration = Duration::from_secs(120);

#[derive(Debug)]
struct IpRecord {
    tokens: f32,
    last_update: Instant,
    banned_until: Option<Instant>,
}

/// Per-IP token-bucket throttle with automatic temp-ban on exhaustion.
#[derive(Clone, Debug)]
pub struct ConnectionThrottle {
    records: Arc<Mutex<HashMap<IpAddr, IpRecord>>>,
    max_per_ip: u32,
}

impl Default for ConnectionThrottle {
    fn default() -> Self {
        Self::new()
    }
}

impl ConnectionThrottle {
    pub fn new() -> Self {
        Self::with_max_per_ip(DEFAULT_MAX_CONNECTIONS_PER_IP)
    }

    /// Build a throttle with a custom per-IP limit. A value of `0` disables
    /// throttling entirely (every connection is allowed).
    pub fn with_max_per_ip(max_per_ip: u32) -> Self {
        Self {
            records: Arc::new(Mutex::new(HashMap::new())),
            max_per_ip,
        }
    }

    pub async fn check(&self, ip: IpAddr) -> Result<(), &'static str> {
        if self.max_per_ip == 0 {
            return Ok(());
        }

        let mut map = self.records.lock().await;
        let now = Instant::now();

        let rec = map.entry(ip).or_insert_with(|| IpRecord {
            tokens: CAPACITY,
            last_update: now,
            banned_until: None,
        });

        if let Some(until) = rec.banned_until {
            if now < until {
                tracing::warn!(%ip, "throttle: rejecting temp-banned IP");
                return Err("temporarily banned");
            }
            rec.banned_until = None;
            rec.tokens = CAPACITY;
        }

        let elapsed = now.duration_since(rec.last_update).as_secs_f32();
        rec.tokens = (rec.tokens + elapsed * REFILL_RATE_PER_SEC).min(CAPACITY);
        rec.last_update = now;

        if rec.tokens < 1.0 {
            rec.banned_until = Some(now + TEMP_BAN_DURATION);
            tracing::warn!(%ip, "throttle: token bucket empty — temp-banning");
            return Err("too many connections");
        }

        rec.tokens -= 1.0;

        Ok(())
    }

    pub async fn evict_stale(&self) {
        let mut map = self.records.lock().await;
        let now = Instant::now();
        map.retain(|_, rec| {
            let active = rec.banned_until.is_some_and(|u| now < u);
            let not_full = rec.tokens < CAPACITY;
            let recent = now.duration_since(rec.last_update) < Duration::from_secs(10);
            active || not_full || recent
        });
    }
}
