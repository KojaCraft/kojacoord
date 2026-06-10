//! Per-player metrics tracking and structured packet trace dumps.
//!
//! Counters are atomic so the relay can call `record_packet_sent` /
//! `record_packet_received` on every packet without taking a lock — the
//! only locked path is registering / unregistering a player or reading
//! the optional packet trace buffer.

use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use uuid::Uuid;

/// Per-player metrics. Counters are atomic so the relay can update them
/// without acquiring a lock; only the `Instant` fields require an
/// `RwLock` wrapper at the registry level.
#[derive(Debug)]
pub struct PlayerMetrics {
    pub packets_sent: AtomicU64,
    pub packets_received: AtomicU64,
    pub bytes_sent: AtomicU64,
    pub bytes_received: AtomicU64,
    pub connected_at: Instant,
    /// Last activity timestamp as `Instant` — stored as monotonic
    /// microseconds since `connected_at` so it fits in an `AtomicU64`.
    pub last_activity_micros: AtomicU64,
    pub latency_ms: AtomicU64,
    pub latency_set: std::sync::atomic::AtomicBool,
}

impl PlayerMetrics {
    fn new() -> Self {
        Self {
            packets_sent: AtomicU64::new(0),
            packets_received: AtomicU64::new(0),
            bytes_sent: AtomicU64::new(0),
            bytes_received: AtomicU64::new(0),
            connected_at: Instant::now(),
            last_activity_micros: AtomicU64::new(0),
            latency_ms: AtomicU64::new(0),
            latency_set: std::sync::atomic::AtomicBool::new(false),
        }
    }

    /// Reconstruct the last-activity `Instant` from the stored offset.
    pub fn last_activity(&self) -> Instant {
        self.connected_at
            + Duration::from_micros(self.last_activity_micros.load(Ordering::Relaxed))
    }

    /// Snapshot the metrics into a plain `PlayerMetricsSnapshot` for
    /// callers that want a stable, owned view (HTTP API, command
    /// handlers, etc.).
    pub fn snapshot(&self) -> PlayerMetricsSnapshot {
        PlayerMetricsSnapshot {
            packets_sent: self.packets_sent.load(Ordering::Relaxed),
            packets_received: self.packets_received.load(Ordering::Relaxed),
            bytes_sent: self.bytes_sent.load(Ordering::Relaxed),
            bytes_received: self.bytes_received.load(Ordering::Relaxed),
            connected_at: self.connected_at,
            last_activity: self.last_activity(),
            latency_ms: if self.latency_set.load(Ordering::Relaxed) {
                Some(self.latency_ms.load(Ordering::Relaxed))
            } else {
                None
            },
        }
    }
}

/// Owned, lock-free snapshot of a player's metrics.
#[derive(Debug, Clone)]
pub struct PlayerMetricsSnapshot {
    pub packets_sent: u64,
    pub packets_received: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub connected_at: Instant,
    pub last_activity: Instant,
    pub latency_ms: Option<u64>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PacketTraceEntry {
    pub timestamp: u64,
    pub direction: String,
    pub packet_id: i32,
    pub packet_name: String,
    pub size_bytes: usize,
    pub protocol_version: u32,
}

#[derive(Debug, Clone)]
pub struct PacketTrace {
    pub entries: Vec<PacketTraceEntry>,
    pub max_entries: usize,
}

impl PacketTrace {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: Vec::with_capacity(max_entries),
            max_entries,
        }
    }

    pub fn add(&mut self, entry: PacketTraceEntry) {
        if self.entries.len() >= self.max_entries {
            self.entries.remove(0);
        }
        self.entries.push(entry);
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

/// Registry of per-player metrics and packet traces.
///
/// `metrics` is a `DashMap<Uuid, Arc<PlayerMetrics>>` — the relay clones
/// the `Arc` once at session start and then increments atomic counters
/// directly, never touching the map again on the hot path.
#[derive(Clone)]
pub struct PlayerMetricsRegistry {
    metrics: Arc<DashMap<Uuid, Arc<PlayerMetrics>>>,
    traces: Arc<RwLock<std::collections::HashMap<Uuid, PacketTrace>>>,
    trace_enabled: Arc<std::sync::atomic::AtomicBool>,
    max_trace_entries: usize,
}

impl PlayerMetricsRegistry {
    pub fn new(max_trace_entries: usize) -> Self {
        Self {
            metrics: Arc::new(DashMap::new()),
            traces: Arc::new(RwLock::new(std::collections::HashMap::new())),
            trace_enabled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            max_trace_entries,
        }
    }

    /// Register a player; returns the `Arc<PlayerMetrics>` for the
    /// relay to cache and increment from the hot path.
    pub fn register_player(&self, uuid: Uuid) -> Arc<PlayerMetrics> {
        let entry = Arc::new(PlayerMetrics::new());
        self.metrics.insert(uuid, entry.clone());
        entry
    }

    pub async fn unregister_player(&self, uuid: Uuid) {
        self.metrics.remove(&uuid);
        let mut traces = self.traces.write().await;
        traces.remove(&uuid);
    }

    /// Lookup the metrics handle for a player. Returns `None` if the
    /// player isn't registered.
    pub fn get(&self, uuid: &Uuid) -> Option<Arc<PlayerMetrics>> {
        self.metrics.get(uuid).map(|e| e.value().clone())
    }

    /// Fire-and-forget: record an inbound packet by `Arc<PlayerMetrics>`
    /// handle. Lock-free.
    #[inline]
    pub fn record_sent(metrics: &PlayerMetrics, size_bytes: usize, now_micros: u64) {
        metrics.packets_sent.fetch_add(1, Ordering::Relaxed);
        metrics
            .bytes_sent
            .fetch_add(size_bytes as u64, Ordering::Relaxed);
        metrics
            .last_activity_micros
            .store(now_micros, Ordering::Relaxed);
    }

    /// Fire-and-forget: record an outbound packet. Lock-free.
    #[inline]
    pub fn record_received(metrics: &PlayerMetrics, size_bytes: usize, now_micros: u64) {
        metrics.packets_received.fetch_add(1, Ordering::Relaxed);
        metrics
            .bytes_received
            .fetch_add(size_bytes as u64, Ordering::Relaxed);
        metrics
            .last_activity_micros
            .store(now_micros, Ordering::Relaxed);
    }

    pub fn update_latency(&self, uuid: Uuid, latency_ms: u64) {
        if let Some(m) = self.metrics.get(&uuid) {
            m.latency_ms.store(latency_ms, Ordering::Relaxed);
            m.latency_set.store(true, Ordering::Relaxed);
        }
    }

    pub async fn add_trace_entry(&self, uuid: Uuid, entry: PacketTraceEntry) {
        if !self.trace_enabled.load(Ordering::Relaxed) {
            return;
        }
        let mut traces = self.traces.write().await;
        let trace = traces
            .entry(uuid)
            .or_insert_with(|| PacketTrace::new(self.max_trace_entries));
        trace.add(entry);
    }

    pub fn set_trace_enabled(&self, enabled: bool) {
        self.trace_enabled.store(enabled, Ordering::Relaxed);
    }

    pub fn get_metrics(&self, uuid: Uuid) -> Option<PlayerMetricsSnapshot> {
        self.metrics.get(&uuid).map(|m| m.value().snapshot())
    }

    pub async fn get_trace(&self, uuid: Uuid) -> Option<Vec<PacketTraceEntry>> {
        let traces = self.traces.read().await;
        traces.get(&uuid).map(|t| t.entries.clone())
    }

    pub fn get_all_metrics(&self) -> std::collections::HashMap<Uuid, PlayerMetricsSnapshot> {
        self.metrics
            .iter()
            .map(|e| (*e.key(), e.value().snapshot()))
            .collect()
    }

    pub async fn clear_trace(&self, uuid: Uuid) {
        let mut traces = self.traces.write().await;
        if let Some(trace) = traces.get_mut(&uuid) {
            trace.clear();
        }
    }

    pub async fn evict_inactive(&self, timeout: Duration) {
        let now = Instant::now();
        // Collect uuids to remove first — DashMap allows removal during
        // iteration but it's cleaner to drop the iterator before the
        // tokio await on the traces map.
        let stale: Vec<Uuid> = self
            .metrics
            .iter()
            .filter(|e| now.duration_since(e.value().last_activity()) >= timeout)
            .map(|e| *e.key())
            .collect();
        for uuid in &stale {
            self.metrics.remove(uuid);
        }
        if !stale.is_empty() {
            let mut traces = self.traces.write().await;
            for uuid in &stale {
                traces.remove(uuid);
            }
        }
    }
}
