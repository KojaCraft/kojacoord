//! Process-wide atomic counters: connection lifetimes, packet and
//! byte volumes. Separate from `kojacoord_metrics` (which is the
//! Prometheus-shaped registry) — these are the raw `AtomicU64`s the
//! hot path increments without going through Prometheus's
//! per-label-lookup path.

use std::sync::atomic::{AtomicU64, Ordering};

pub struct ProxyMetrics {
    pub total_connections: AtomicU64,
    pub active_connections: AtomicU64,
    pub packets_relayed: AtomicU64,
    pub bytes_transferred: AtomicU64,
    pub failed_connections: AtomicU64,
}

impl ProxyMetrics {
    pub fn new() -> Self {
        Self {
            total_connections: AtomicU64::new(0),
            active_connections: AtomicU64::new(0),
            packets_relayed: AtomicU64::new(0),
            bytes_transferred: AtomicU64::new(0),
            failed_connections: AtomicU64::new(0),
        }
    }

    pub fn record_connection(&self) {
        self.total_connections.fetch_add(1, Ordering::Relaxed);
        self.active_connections.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_disconnection(&self) {
        self.active_connections.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn record_packet(&self, size: usize) {
        self.packets_relayed.fetch_add(1, Ordering::Relaxed);
        self.bytes_transferred
            .fetch_add(size as u64, Ordering::Relaxed);
    }

    /// Bump the failure counter. The caller is responsible for also
    /// calling [`Self::record_disconnection`] — every failed
    /// connection is also a disconnected one, but separating the
    /// concerns lets callers in accounting-light paths skip one or
    /// the other.
    pub fn record_failed_connection(&self) {
        self.failed_connections.fetch_add(1, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            total_connections: self.total_connections.load(Ordering::Relaxed),
            active_connections: self.active_connections.load(Ordering::Relaxed),
            packets_relayed: self.packets_relayed.load(Ordering::Relaxed),
            bytes_transferred: self.bytes_transferred.load(Ordering::Relaxed),
            failed_connections: self.failed_connections.load(Ordering::Relaxed),
        }
    }
}

impl Default for ProxyMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    pub total_connections: u64,
    pub active_connections: u64,
    pub packets_relayed: u64,
    pub bytes_transferred: u64,
    pub failed_connections: u64,
}
