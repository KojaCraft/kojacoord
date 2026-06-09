//! HAProxy PROXY protocol v1/v2 parser.
//!
//! When the proxy sits behind a load balancer that sends a PROXY header, this
//! module extracts the real client address before the Minecraft handshake begins.
use std::net::{IpAddr, SocketAddr};
use tokio::io::AsyncReadExt;

use crate::error::ConnectionError;

const V2_SIG: &[u8] = b"\x0D\x0A\x0D\x0A\x00\x0D\x0A\x51\x55\x49\x54\x0A";
const V1_SIG: &[u8] = b"PROXY ";

/// Reads a PROXY protocol v1 or v2 header from the stream if present, and returns
/// the original client's SocketAddr. If no PROXY header is present (and it was
/// expected), it returns an error.
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
