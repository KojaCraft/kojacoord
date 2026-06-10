//! HAProxy PROXY protocol v1/v2 parser.
//!
//! When the proxy sits behind a TCP load balancer (HAProxy, AWS NLB,
//! Cloudflare Spectrum), the L4 in front strips the real client IP
//! and replaces it with its own. The PROXY protocol is a small header
//! prepended to the TCP stream that carries the original client
//! address so downstream services can log/rate-limit by the real
//! source.
//!
//! Two modes: [`read_proxy_header`] is strict — header must be there
//! or the connection is rejected — and [`read_proxy_header_optional`]
//! sniffs the first six bytes, falls back to a vanilla Minecraft
//! handshake if no PROXY signature is found, and returns the consumed
//! bytes for the caller to re-feed into the handshake parser.
use std::net::{IpAddr, SocketAddr};
use tokio::io::AsyncReadExt;

use crate::error::ConnectionError;

const V2_SIG: &[u8] = b"\x0D\x0A\x0D\x0A\x00\x0D\x0A\x51\x55\x49\x54\x0A";
const V1_SIG: &[u8] = b"PROXY ";

pub enum ProxyHeaderResult {
    /// Header parsed cleanly — the inner address is the real client.
    Found(SocketAddr),
    /// No PROXY signature; the inner bytes were consumed from the
    /// stream while sniffing and must be replayed into the Minecraft
    /// handshake parser. Dropping them on the floor desyncs the wire.
    NotFound(Vec<u8>),
}

/// Sniff at most one PROXY header. Returns `Found(real_addr)` when
/// the signature parses, or `NotFound(consumed)` if the stream starts
/// with anything else — caller must re-feed `consumed` into the
/// connection handler since those bytes are now the head of the
/// Minecraft handshake.
pub async fn read_proxy_header_optional<R: AsyncReadExt + Unpin>(
    src: &mut R,
) -> Result<ProxyHeaderResult, ConnectionError> {
    let mut header_buf = [0u8; 512];
    let mut pos = 0;

    // Read the first 6 bytes to determine v1 or v2
    match src.read_exact(&mut header_buf[0..6]).await {
        Ok(_) => {},
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
            // Connection closed before any data
            return Ok(ProxyHeaderResult::NotFound(vec![]));
        },
        Err(e) => return Err(ConnectionError::Io(e)),
    }
    pos += 6;

    if &header_buf[0..6] == V1_SIG {
        // Parse v1
        loop {
            if pos >= header_buf.len() {
                return Err(ConnectionError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "PROXY v1 header too long",
                )));
            }
            let byte = src.read_u8().await?;
            header_buf[pos] = byte;
            pos += 1;
            if byte == b'\n' {
                break;
            }
        }

        let header_str = std::str::from_utf8(&header_buf[..pos]).unwrap_or("");
        let parts: Vec<&str> = header_str.split(' ').collect();
        if parts.len() >= 6 {
            if let Ok(ip) = parts[2].parse::<IpAddr>() {
                if let Ok(port) = parts[4].parse::<u16>() {
                    tracing::debug!(real_addr = %SocketAddr::new(ip, port), "PROXY v1 header parsed");
                    return Ok(ProxyHeaderResult::Found(SocketAddr::new(ip, port)));
                }
            }
        }
        // Invalid v1 header, treat as not found
        return Ok(ProxyHeaderResult::NotFound(header_buf[..pos].to_vec()));
    } else if header_buf[0..6] == V2_SIG[0..6] {
        // Parse v2
        // Read the rest of the 12-byte signature + 4 bytes of header
        src.read_exact(&mut header_buf[6..16]).await?;
        pos += 10;

        if header_buf[0..12] != *V2_SIG {
            // Invalid signature, treat as not found
            return Ok(ProxyHeaderResult::NotFound(header_buf[..pos].to_vec()));
        }

        let fam = header_buf[13];
        // The 15th and 16th byte are the length of the address block
        let addr_len = u16::from_be_bytes([header_buf[14], header_buf[15]]) as usize;
        if pos + addr_len > header_buf.len() {
            return Err(ConnectionError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "PROXY v2 header too long",
            )));
        }

        src.read_exact(&mut header_buf[16..16 + addr_len]).await?;
        pos += addr_len;

        // IPv4
        if fam == 0x11 && addr_len >= 12 {
            let ip = std::net::Ipv4Addr::new(
                header_buf[16],
                header_buf[17],
                header_buf[18],
                header_buf[19],
            );
            let port = u16::from_be_bytes([header_buf[24], header_buf[25]]);
            tracing::debug!(real_addr = %SocketAddr::new(IpAddr::V4(ip), port), "PROXY v2 header parsed");
            return Ok(ProxyHeaderResult::Found(SocketAddr::new(
                IpAddr::V4(ip),
                port,
            )));
        }
        // IPv6
        else if fam == 0x21 && addr_len >= 36 {
            let mut ip_bytes = [0u8; 16];
            ip_bytes.copy_from_slice(&header_buf[16..32]);
            let ip = std::net::Ipv6Addr::from(ip_bytes);
            let port = u16::from_be_bytes([header_buf[48], header_buf[49]]);
            tracing::debug!(real_addr = %SocketAddr::new(IpAddr::V6(ip), port), "PROXY v2 header parsed");
            return Ok(ProxyHeaderResult::Found(SocketAddr::new(
                IpAddr::V6(ip),
                port,
            )));
        }

        // Unknown family, treat as not found
        return Ok(ProxyHeaderResult::NotFound(header_buf[..pos].to_vec()));
    }

    // No PROXY header detected - return the 6 bytes we read
    tracing::debug!("No PROXY header detected, assuming direct connection");
    Ok(ProxyHeaderResult::NotFound(header_buf[..6].to_vec()))
}

/// v2-only address extraction. Same shape as
/// [`read_proxy_header_optional`] but doesn't return the consumed bytes
/// (the caller must already know it's done with this stream). PROXY v1
/// has variable-length headers so it's not handled here.
pub async fn peek_proxy_header<R: AsyncReadExt + Unpin>(src: &mut R) -> Option<SocketAddr> {
    let mut peek_buf = [0u8; 16];

    // Try to peek at the first bytes
    match src.read_exact(&mut peek_buf[0..6]).await {
        Ok(_) => {},
        Err(_) => return None,
    }

    if &peek_buf[0..6] == V1_SIG {
        // v1 detected - parse it
        return None; // v1 requires variable-length parsing, not supported in peek
    } else if peek_buf[0..6] == V2_SIG[0..6] {
        // Read rest of signature
        if src.read_exact(&mut peek_buf[6..16]).await.is_err() {
            return None;
        }

        if peek_buf[0..12] != *V2_SIG {
            return None;
        }

        let fam = peek_buf[13];
        let addr_len = u16::from_be_bytes([peek_buf[14], peek_buf[15]]) as usize;

        if addr_len > 36 {
            return None;
        }

        let mut addr_buf = vec![0u8; addr_len];
        if src.read_exact(&mut addr_buf).await.is_err() {
            return None;
        }

        // IPv4
        if fam == 0x11 && addr_len >= 12 {
            let ip = std::net::Ipv4Addr::new(addr_buf[0], addr_buf[1], addr_buf[2], addr_buf[3]);
            let port = u16::from_be_bytes([addr_buf[8], addr_buf[9]]);
            return Some(SocketAddr::new(IpAddr::V4(ip), port));
        }
        // IPv6
        else if fam == 0x21 && addr_len >= 36 {
            let mut ip_bytes = [0u8; 16];
            ip_bytes.copy_from_slice(&addr_buf[0..16]);
            let ip = std::net::Ipv6Addr::from(ip_bytes);
            let port = u16::from_be_bytes([addr_buf[32], addr_buf[33]]);
            return Some(SocketAddr::new(IpAddr::V6(ip), port));
        }
    }

    None
}

/// Strict PROXY header parse. Returns the real client address on
/// success, errors if the stream doesn't start with a PROXY v1 or v2
/// signature. Use this when the L4 in front of us is guaranteed to
/// send the header; the connection has already lost six bytes of
/// handshake by the time we'd realise something else is on the wire.
pub async fn read_proxy_header<R: AsyncReadExt + Unpin>(
    src: &mut R,
    current_addr: SocketAddr,
) -> Result<SocketAddr, ConnectionError> {
    // We don't want to block forever if it's a slow loris attack.
    // However, the upstream connection_throttle and timeout should protect us.

    let mut header_buf = [0u8; 512];
    let mut pos = 0;

    // Read the first 6 bytes to determine v1 or v2
    src.read_exact(&mut header_buf[0..6]).await?;
    pos += 6;

    if &header_buf[0..6] == V1_SIG {
        // Parse v1
        loop {
            if pos >= header_buf.len() {
                return Err(ConnectionError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "PROXY v1 header too long",
                )));
            }
            let byte = src.read_u8().await?;
            header_buf[pos] = byte;
            pos += 1;
            if byte == b'\n' {
                break;
            }
        }

        let header_str = std::str::from_utf8(&header_buf[..pos]).unwrap_or("");
        let parts: Vec<&str> = header_str.split(' ').collect();
        if parts.len() >= 6 {
            if let Ok(ip) = parts[2].parse::<IpAddr>() {
                if let Ok(port) = parts[4].parse::<u16>() {
                    return Ok(SocketAddr::new(ip, port));
                }
            }
        }
        return Ok(current_addr);
    } else if header_buf[0..6] == V2_SIG[0..6] {
        // Parse v2
        // Read the rest of the 12-byte signature + 4 bytes of header
        src.read_exact(&mut header_buf[6..16]).await?;
        pos += 10;

        if header_buf[0..12] != *V2_SIG {
            return Err(ConnectionError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid PROXY v2 signature",
            )));
        }

        let fam = header_buf[13];
        // The 15th and 16th byte are the length of the address block
        let addr_len = u16::from_be_bytes([header_buf[14], header_buf[15]]) as usize;
        if pos + addr_len > header_buf.len() {
            return Err(ConnectionError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "PROXY v2 header too long",
            )));
        }

        src.read_exact(&mut header_buf[16..16 + addr_len]).await?;

        // IPv4
        if fam == 0x11 && addr_len >= 12 {
            let ip = std::net::Ipv4Addr::new(
                header_buf[16],
                header_buf[17],
                header_buf[18],
                header_buf[19],
            );
            let port = u16::from_be_bytes([header_buf[24], header_buf[25]]);
            return Ok(SocketAddr::new(IpAddr::V4(ip), port));
        }
        // IPv6
        else if fam == 0x21 && addr_len >= 36 {
            let mut ip_bytes = [0u8; 16];
            ip_bytes.copy_from_slice(&header_buf[16..32]);
            let ip = std::net::Ipv6Addr::from(ip_bytes);
            let port = u16::from_be_bytes([header_buf[48], header_buf[49]]);
            return Ok(SocketAddr::new(IpAddr::V6(ip), port));
        }

        return Ok(current_addr);
    }

    // If it's not a proxy header, we've just consumed 6 bytes of a Minecraft handshake!
    // This proxy_protocol mode must only be enabled when we are strictly expecting a proxy header!
    Err(ConnectionError::Io(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        "Expected PROXY header, but signature did not match",
    )))
}
