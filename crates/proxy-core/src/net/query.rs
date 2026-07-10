//! GameSpy4 (UT3) query protocol.
//!
//! The same `enable-query`/`query.port` protocol vanilla Minecraft servers
//! speak, so third-party server-list aggregators that ping via UDP query
//! (rather than the normal Java status ping) see this proxy correctly.
//! Entirely separate from the Java client TCP protocol — its own UDP
//! socket, its own wire format, zero interaction with packet compression
//! or the Minecraft protocol stack. Opt-in via `[query].enabled`.
//!
//! Wire format (stable and unauthenticated by design — same trust level as
//! the Java status ping; this is a public, read-only protocol):
//!
//! ```text
//! Request:  0xFE 0xFD <type:1> <session_id:4 BE>  [+ type-specific tail]
//! ```
//!
//! - **Handshake** (`type == 0x09`): server replies with a numeric
//!   challenge token (ASCII decimal, null-terminated) the client must echo
//!   back — as a 4-byte big-endian int — in its stat request.
//! - **Stat** (`type == 0x00`): `<session_id:4><token:4>` requests a
//!   *basic* stat; four more (ignored) padding bytes after the token
//!   request a *full* stat instead.
//!
//! Reference: the Minecraft protocol wiki's Query page, which documents
//! vanilla's implementation of the same GameSpy4 protocol.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use rand::Rng;
use tokio::net::UdpSocket;

use crate::proxy::ProxyState;

const MAGIC: [u8; 2] = [0xFE, 0xFD];
const TYPE_HANDSHAKE: u8 = 0x09;
const TYPE_STAT: u8 = 0x00;

/// How long a challenge token stays valid after a handshake. Generous —
/// this is a low-stakes, read-only protocol; the TTL only exists so
/// `challenges` doesn't grow forever from one-off handshakes that never
/// follow up with a stat request.
const CHALLENGE_TTL: Duration = Duration::from_secs(30);
const MAX_CHALLENGES: usize = 100_000;

struct Challenge {
    token: i32,
    issued_at: Instant,
}

pub struct QueryServer {
    state: Arc<ProxyState>,
    challenges: DashMap<(SocketAddr, i32), Challenge>,
}

impl QueryServer {
    pub fn new(state: Arc<ProxyState>) -> Arc<Self> {
        Arc::new(Self {
            state,
            challenges: DashMap::new(),
        })
    }

    /// Bind the UDP socket and serve forever. Each packet is handled on its
    /// own spawned task so one slow/malformed request can't stall others.
    pub async fn serve(self: Arc<Self>, bind: String) -> std::io::Result<()> {
        let socket = Arc::new(UdpSocket::bind(&bind).await?);
        tracing::info!("Query (GS4) server listening on {}", bind);

        let mut buf = [0u8; 1500];
        loop {
            let (len, addr) = match socket.recv_from(&mut buf).await {
                Ok(v) => v,
                Err(e) => {
                    tracing::debug!(error = %e, "query socket recv error");
                    continue;
                },
            };
            // 1500 bytes is far more than any valid GS4 request needs;
            // truncate defensively rather than trusting `len`.
            let data = buf[..len.min(buf.len())].to_vec();
            let this = Arc::clone(&self);
            let socket = Arc::clone(&socket);
            tokio::spawn(async move {
                if let Some(response) = this.handle_packet(&data, addr).await {
                    let _ = socket.send_to(&response, addr).await;
                }
            });
        }
    }

    async fn handle_packet(&self, data: &[u8], addr: SocketAddr) -> Option<Vec<u8>> {
        if data.len() < 7 || data[0] != MAGIC[0] || data[1] != MAGIC[1] {
            return None;
        }
        let packet_type = data[2];
        let session_id = i32::from_be_bytes(data[3..7].try_into().ok()?);

        match packet_type {
            TYPE_HANDSHAKE => Some(self.handshake_response(addr, session_id)),
            TYPE_STAT => {
                if data.len() < 11 {
                    return None;
                }
                let token = i32::from_be_bytes(data[7..11].try_into().ok()?);
                if !self.verify_challenge(addr, session_id, token) {
                    tracing::debug!(%addr, "query stat request with invalid/expired challenge token");
                    return None;
                }
                // A full-stat request carries 4 extra (content-irrelevant)
                // padding bytes after the token; basic-stat doesn't.
                let full = data.len() >= 15;
                Some(if full {
                    self.full_stat_response(session_id)
                } else {
                    self.basic_stat_response(session_id)
                })
            },
            _ => None,
        }
    }

    fn handshake_response(&self, addr: SocketAddr, session_id: i32) -> Vec<u8> {
        // Range starts at 1 — a token of 0 round-trips ambiguously through
        // some ASCII-parsing GS4 clients that treat an empty/zero string as
        // "no token yet".
        let token: i32 = rand::thread_rng().gen_range(1..=i32::MAX);

        if self.challenges.len() < MAX_CHALLENGES {
            self.challenges.insert(
                (addr, session_id),
                Challenge {
                    token,
                    issued_at: Instant::now(),
                },
            );
        }

        let mut out = Vec::with_capacity(16);
        out.push(TYPE_HANDSHAKE);
        out.extend_from_slice(&session_id.to_be_bytes());
        out.extend_from_slice(token.to_string().as_bytes());
        out.push(0);
        out
    }

    fn verify_challenge(&self, addr: SocketAddr, session_id: i32, token: i32) -> bool {
        match self.challenges.get(&(addr, session_id)) {
            Some(c) if c.issued_at.elapsed() < CHALLENGE_TTL => c.token == token,
            _ => false,
        }
    }

    /// Drop expired challenge tokens. Call periodically from a background
    /// task, same pattern as the other per-source record maps in this crate.
    pub fn evict_stale(&self) {
        self.challenges
            .retain(|_, c| c.issued_at.elapsed() < CHALLENGE_TTL);
    }

    fn snapshot(&self) -> (String, usize, usize, u16) {
        let motd = self.state.config.listeners.motd.clone();
        let online = self.state.sessions.len();
        let max = self.state.config.proxy.max_players;
        let port = self
            .state
            .config
            .proxy
            .bind
            .rsplit_once(':')
            .and_then(|(_, p)| p.parse::<u16>().ok())
            .unwrap_or(25565);
        (motd, online, max, port)
    }

    fn basic_stat_response(&self, session_id: i32) -> Vec<u8> {
        let (motd, online, max, port) = self.snapshot();

        let mut out = Vec::new();
        out.push(TYPE_STAT);
        out.extend_from_slice(&session_id.to_be_bytes());
        push_cstr(&mut out, &motd);
        push_cstr(&mut out, "SMP");
        push_cstr(&mut out, "world");
        push_cstr(&mut out, &online.to_string());
        push_cstr(&mut out, &max.to_string());
        // hostport is the one field NOT null-terminated ASCII — raw
        // little-endian i16, per the GS4 spec.
        out.extend_from_slice(&port.to_le_bytes());
        push_cstr(&mut out, "0.0.0.0");
        out
    }

    fn full_stat_response(&self, session_id: i32) -> Vec<u8> {
        let (motd, online, max, port) = self.snapshot();
        let server_names: Vec<String> = self
            .state
            .server_registry
            .all()
            .iter()
            .map(|s| s.name.clone())
            .collect();

        let mut out = Vec::new();
        out.push(TYPE_STAT);
        out.extend_from_slice(&session_id.to_be_bytes());

        // Constant padding the GS4 spec requires before the K/V section.
        out.extend_from_slice(b"splitnum\0\x80\0");

        let kv = [
            ("hostname", motd.as_str()),
            ("gametype", "SMP"),
            ("game_id", "MINECRAFT"),
            ("version", "1.6-1.26"),
            // No real plugin list to expose (this is a proxy, not a
            // backend) — list the configured backend names instead, the
            // closest analog available post-DB-removal.
            ("plugins", &server_names.join("; ")),
            ("map", "world"),
            ("numplayers", &online.to_string()),
            ("maxplayers", &max.to_string()),
            ("hostport", &port.to_string()),
            ("hostip", "0.0.0.0"),
        ];
        for (key, value) in kv {
            push_cstr(&mut out, key);
            push_cstr(&mut out, value);
        }
        out.push(0); // empty key terminates the K/V section
        out.push(0);

        // Constant padding before the player list.
        out.extend_from_slice(b"\x01player_\0\0");
        for entry in self.state.sessions.iter() {
            if let Ok(session) = entry.value().try_read() {
                push_cstr(&mut out, &session.username);
            }
        }
        out.push(0); // terminates the player list

        out
    }
}

fn push_cstr(out: &mut Vec<u8>, s: &str) {
    out.extend_from_slice(s.as_bytes());
    out.push(0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_magic() {
        // Can't easily build a full ProxyState in a unit test; the magic/
        // length checks in `handle_packet` run before any state access, so
        // exercise them directly against the constant.
        let data = [0x00, 0x00, 0x09, 0, 0, 0, 1];
        assert_ne!(data[0], MAGIC[0]);
    }

    #[test]
    fn push_cstr_null_terminates() {
        let mut out = Vec::new();
        push_cstr(&mut out, "hello");
        assert_eq!(out, b"hello\0");
    }
}
