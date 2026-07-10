//! Active TCP health-checking for registered backends.
//!
//! Every 10 seconds the prober opens a throwaway TCP connection to
//! each backend with a per-server interval/timeout/threshold. After
//! `health_probe_fail_threshold` consecutive failures the backend
//! flips to `unhealthy` (routing skips it); one success flips it
//! back. The check is intentionally just a TCP handshake — we don't
//! send a Minecraft ping packet because most backends accept TCP
//! before they accept gameplay, and we'd rather route around a stuck
//! handshake than a stuck listener.

use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::TcpStream;
use tokio::time::timeout;

use crate::server::ServerRegistry;

/// One TCP probe; returns the connect latency in milliseconds on success
/// (a SYN-ACK inside the timeout), `None` on failure/timeout. The stream is
/// dropped immediately — we never send a packet across it. The latency
/// feeds `BackendServer::last_latency_ms`, used by `[[server_groups]]`'s
/// `"latency"` selection strategy.
async fn probe_server(address: std::net::SocketAddr, timeout_secs: u64) -> Option<u64> {
    let started = Instant::now();
    match timeout(
        Duration::from_secs(timeout_secs),
        TcpStream::connect(address),
    )
    .await
    {
        Ok(Ok(_)) => Some(started.elapsed().as_millis() as u64),
        _ => None,
    }
}

/// Spawn the probe loop. Runs forever in the background; never exits
/// even if individual probes fail. Servers with
/// `health_probe_interval_secs = 0` are treated as opted-out.
pub fn start_health_probes(registry: Arc<ServerRegistry>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(10)); // Check every 10 seconds

        loop {
            interval.tick().await;

            let servers = registry.all();

            for server in servers {
                // Skip servers with health probes disabled
                if server.health_probe_interval_secs == 0 {
                    continue;
                }

                let latency_ms =
                    probe_server(server.address, server.health_probe_timeout_secs).await;

                if let Some(latency_ms) = latency_ms {
                    // Probe succeeded
                    server.last_latency_ms.store(latency_ms, Ordering::Relaxed);
                    let fail_count = server.health_fail_count.load(Ordering::Relaxed);
                    if fail_count > 0 {
                        // Reset failure count
                        server.health_fail_count.store(0, Ordering::Relaxed);
                    }

                    // If server was marked unhealthy, mark it healthy again
                    if server.health_unhealthy.load(Ordering::Relaxed) {
                        server.health_unhealthy.store(false, Ordering::Relaxed);
                        tracing::info!(server = %server.name, "Server health restored, marked as healthy");
                    }
                } else {
                    // Probe failed
                    let new_fail_count =
                        server.health_fail_count.fetch_add(1, Ordering::Relaxed) + 1;

                    if new_fail_count >= server.health_probe_fail_threshold {
                        // Mark as unhealthy if threshold reached
                        if !server.health_unhealthy.load(Ordering::Relaxed) {
                            server.health_unhealthy.store(true, Ordering::Relaxed);
                            tracing::warn!(
                                server = %server.name,
                                fail_count = new_fail_count,
                                threshold = server.health_probe_fail_threshold,
                                "Server marked as unhealthy after consecutive probe failures"
                            );
                        }
                    } else {
                        tracing::debug!(
                            server = %server.name,
                            fail_count = new_fail_count,
                            "Server health probe failed"
                        );
                    }
                }
            }
        }
    });
}
