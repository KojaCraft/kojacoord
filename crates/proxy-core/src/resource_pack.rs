//! Resource-pack injection.
//!
//! Builds the clientbound resource-pack push packet for protocols that
//! support it (1.20.3+, where the packet was promoted to its own play
//! packet id). Older clients are rejected — they used the legacy
//! `LoginResourcePack` / single-pack play packet that is not yet wired
//! through this builder.

use bytes::{BufMut, BytesMut};
use kojacoord_protocol::{Encode, ProtocolVersion, VarInt};

/// Build a resource-pack-push packet for the given protocol version.
///
/// Spec: <https://minecraft.wiki/w/Java_Edition_protocol> — "Resource Pack
/// (Push)". The packet shape on 1.20.3+ is:
///
/// ```text
///   UUID pack_id        (16 bytes)
///   String url          (varint length-prefixed)
///   String hash         (varint length-prefixed, must be 40 chars or empty)
///   Boolean forced
///   Optional<Component> prompt
/// ```
pub fn build_resource_pack_packet(
    url: &str,
    hash: &str,
    required: bool,
    prompt: Option<&str>,
    protocol_version: u32,
) -> Result<BytesMut, String> {
    let canonical = ProtocolVersion::from_id(protocol_version);

    // The dedicated `Resource Pack Push` packet ships from 1.20.3 (proto
    // 765) onwards. Pre-1.20.3 protocols used a single-pack packet whose
    // structure differs slightly; we don't synthesise that variant yet.
    if canonical.id() < 765 {
        return Err(format!(
            "resource pack push not implemented for protocol {} (<765 / 1.20.3)",
            canonical.id()
        ));
    }

    // Packet id for clientbound "Resource Pack Push" in the Play state.
    //   1.20.3 / 1.20.4 (765):                0x40
    //   1.20.5 / 1.20.6 (766):                0x46
    //   1.21 / 1.21.1   (767):                0x46
    //   1.21.2+         (768+):               0x4A
    let packet_id: i32 = match canonical.id() {
        765 => 0x40,
        766 | 767 => 0x46,
        768..=u32::MAX => 0x4A,
        _ => unreachable!(),
    };

    let mut payload = BytesMut::new();
    VarInt(packet_id)
        .encode(&mut payload)
        .map_err(|e| format!("encode packet id: {}", e))?;

    // pack_id: 16 bytes that uniquely identify this pack on the client so
    // it can dedupe re-pushes. We derive it deterministically from the URL
    // via SHA-1 (truncated to 16 bytes, with variant/version bits set to
    // produce a well-formed UUIDv5-shaped value without pulling in the
    // optional `uuid/v5` cargo feature).
    use sha1::{Digest, Sha1};
    let mut hasher = Sha1::new();
    hasher.update(url.as_bytes());
    let digest = hasher.finalize();
    let mut pack_id_bytes = [0u8; 16];
    pack_id_bytes.copy_from_slice(&digest[..16]);
    pack_id_bytes[6] = (pack_id_bytes[6] & 0x0F) | 0x50; // version 5
    pack_id_bytes[8] = (pack_id_bytes[8] & 0x3F) | 0x80; // RFC 4122 variant
    payload.extend_from_slice(&pack_id_bytes);

    // url
    let url_bytes = url.as_bytes();
    VarInt(url_bytes.len() as i32)
        .encode(&mut payload)
        .map_err(|e| format!("encode url length: {}", e))?;
    payload.extend_from_slice(url_bytes);

    // hash — must be exactly 40 chars (SHA-1 hex) or empty; the client
    // rejects anything else. We pass through whatever the caller supplied.
    let hash_bytes = hash.as_bytes();
    VarInt(hash_bytes.len() as i32)
        .encode(&mut payload)
        .map_err(|e| format!("encode hash length: {}", e))?;
    payload.extend_from_slice(hash_bytes);

    // forced
    payload.put_u8(required as u8);

    // Optional<Component> prompt
    match prompt {
        Some(p) => {
            payload.put_u8(1);
            // Send a plain-text component as JSON so it works on every
            // 1.20.3+ client without needing the NBT component encoding.
            let component = format!(r#"{{"text":{}}}"#, serde_json::to_string(p).unwrap_or_else(|_| "\"\"".into()));
            let cb = component.as_bytes();
            VarInt(cb.len() as i32)
                .encode(&mut payload)
                .map_err(|e| format!("encode prompt length: {}", e))?;
            payload.extend_from_slice(cb);
        },
        None => payload.put_u8(0),
    }

    Ok(payload)
}

/// Whether the config has enough information to send a resource pack.
pub fn should_send_resource_pack(
    config_url: &Option<String>,
    config_hash: &Option<String>,
) -> bool {
    config_url.is_some() && config_hash.is_some()
}
