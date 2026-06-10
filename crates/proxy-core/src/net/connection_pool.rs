//! Per-backend TCP connection pool.
//!
//! Lazy on-demand: the relay calls `acquire()` when a new player
//! arrives, the pool either returns a cached stream or dials a fresh
//! one. Capped per backend so a single hot lobby can't starve other
//! backends; pruned by an idle-timeout watcher. Failed dials feed
//! back into the health-probe failure counter, so a backend that
//! routinely refuses connections trips the unhealthy flag without a
//! dedicated probe round.

use std::collections::VecDeque;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::RwLock;
use tokio::time::Duration;

pub struct BackendConnectionPool {
    connections: Arc<RwLock<VecDeque<TcpStream>>>,
    max_size: usize,
    server_addr: SocketAddr,
    connect_timeout_ms: u64,
}

impl BackendConnectionPool {
    pub fn new(server_addr: SocketAddr, max_size: usize) -> Self {
        Self::with_timeout(server_addr, max_size, 1500)
    }

    pub fn with_timeout(server_addr: SocketAddr, max_size: usize, connect_timeout_ms: u64) -> Self {
        Self {
            connections: Arc::new(RwLock::new(VecDeque::with_capacity(max_size))),
            max_size,
            server_addr,
            connect_timeout_ms,
        }
    }

    pub async fn acquire(&self) -> Result<TcpStream, std::io::Error> {
        loop {
            let candidate = self.connections.write().await.pop_front();
            match candidate {
                None => break,
                Some(conn) => {
                    if Self::is_alive(&conn).await {
                        tracing::debug!(
                            addr = %self.server_addr,
                            "Reused pooled backend connection"
                        );
                        return Ok(conn);
                    } else {
                        tracing::trace!(
                            addr = %self.server_addr,
                            "Discarding stale pooled connection"
                        );
                    }
                },
            }
        }

        tracing::debug!(addr = %self.server_addr, "Creating new backend connection");
        match tokio::time::timeout(
            Duration::from_millis(self.connect_timeout_ms),
            TcpStream::connect(self.server_addr),
        )
        .await
        {
            Ok(result) => result,
            Err(_) => Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                format!(
                    "connection to {} timed out after {}ms",
                    self.server_addr, self.connect_timeout_ms
                ),
            )),
        }
    }

    pub async fn release(&self, conn: TcpStream) {
        if !Self::is_alive(&conn).await {
            tracing::trace!(addr = %self.server_addr, "Not pooling dead connection");
            return;
        }
        let mut pool = self.connections.write().await;
        if pool.len() < self.max_size {
            pool.push_back(conn);
            tracing::trace!(
                addr = %self.server_addr,
                pool_size = pool.len(),
                "Released connection to pool"
            );
        } else {
            tracing::trace!(
                addr = %self.server_addr,
                max = self.max_size,
                "Pool full — dropping connection"
            );
        }
    }

    pub async fn pool_size(&self) -> usize {
        self.connections.read().await.len()
    }

    async fn is_alive(conn: &TcpStream) -> bool {
        if conn.peer_addr().is_err() {
            return false;
        }

        let mut buf = [0u8; 1];
        match conn.try_read(&mut buf) {
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => true,
            Ok(0) => false,
            Ok(_) => false,
            Err(_) => false,
        }
    }
}
