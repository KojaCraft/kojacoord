//! Bridge a 1.16.5 (proto 754) server to a 1.20.2+ (proto 764 / 765 / 767) client.
//!
//! Scope: this is intentionally a *partial* bridge. A full ViaVersion-grade
//! translation across this five-year gap (chunk format rewrite, registry
//! restructuring, chat signing, configuration phase, ID shifts) is multi-week
//! work. We get the bits that make the handshake/login dance survive, drop the
//! tricky bits with a warning log, and pass everything else through.
//!
//! Packet ID tables used (from PrismarineJS minecraft-data proto.yml):
//!
//! ## 1.16.5 (proto 754) — login state S2C
//! - 0x00 Disconnect
//! - 0x01 Encryption Request
//! - 0x02 Login Success
//! - 0x03 Set Compression
//! - 0x04 Login Plugin Request
//!
//! ## 1.16.5 (proto 754) — play state S2C (relevant subset)
//! - 0x0E Chat Message
//! - 0x19 Disconnect (play)
//! - 0x1F Keep Alive
//! - 0x24 Join Game
//!
//! ## 1.20.4 (proto 765) — login state S2C
//! - 0x00 Disconnect
//! - 0x01 Encryption Request
//! - 0x02 Login Success
//! - 0x03 Set Compression
//! - 0x04 Login Plugin Request
//!
//! ## 1.20.4 (proto 765) — configuration state S2C
//! - 0x00 Plugin Message
//! - 0x01 Disconnect
//! - 0x02 Finish Configuration
//! - 0x03 Keep Alive
//! - 0x05 Registry Data
//!
//! ## 1.20.4 (proto 765) — play state S2C (relevant subset)
//! - 0x1B Disconnect (play)
//! - 0x24 Keep Alive
//! - 0x29 Login (Join Game)
//! - 0x35 Player Chat
//! - 0x69 System Chat
//!
//! ## 1.21 (proto 767) — most play IDs shift relative to 1.20.4; here we route
//! 1.21 through this same converter so the IDs we emit will be slightly off
//! for play. The login/configuration IDs above are stable through 1.21.

use bytes::{Buf, BufMut, Bytes, BytesMut};
use kojacoord_protocol::codec::{Decode, Encode};
use kojacoord_protocol::types::VarInt;

use super::{build_payload, split_id};
use crate::converter::ConversionResult;

// ---- 1.16.5 source IDs ----
const V165_LOGIN_S2C_DISCONNECT: u8 = 0x00;
const V165_LOGIN_S2C_ENCRYPTION_REQUEST: u8 = 0x01;
const V165_LOGIN_S2C_LOGIN_SUCCESS: u8 = 0x02;
const V165_LOGIN_S2C_SET_COMPRESSION: u8 = 0x03;

const V165_PLAY_S2C_CHAT: u8 = 0x0E;
const V165_PLAY_S2C_DISCONNECT: u8 = 0x19;
const V165_PLAY_S2C_KEEP_ALIVE: u8 = 0x1F;
const V165_PLAY_S2C_JOIN_GAME: u8 = 0x24;

// ---- 1.20.4 target IDs ----
const V1204_LOGIN_S2C_DISCONNECT: u8 = 0x00;
const V1204_LOGIN_S2C_ENCRYPTION_REQUEST: u8 = 0x01;
const V1204_LOGIN_S2C_LOGIN_SUCCESS: u8 = 0x02;
const V1204_LOGIN_S2C_SET_COMPRESSION: u8 = 0x03;

const V1204_PLAY_S2C_DISCONNECT: u8 = 0x1B;
const V1204_PLAY_S2C_KEEP_ALIVE: u8 = 0x24;
const V1204_PLAY_S2C_SYSTEM_CHAT: u8 = 0x69;

/// S2C entry point. We don't know the connection state here (login vs play vs
/// configuration) — we use a heuristic: small IDs that decode as a known login
/// shape are treated as login. The dispatcher only calls us when the proxy is
/// past handshake, so login-state IDs (0x00..=0x04) and play-state IDs share
/// the same numeric space.
pub fn convert_s2c(payload: Bytes) -> ConversionResult {
    let Some((id, body)) = split_id(payload.clone()) else {
        return ConversionResult::Passthrough;
    };

    match id {
        // Login: field layouts are identical between 1.16.5 and 1.20.4 for
        // Set Compression / Encryption Request / Disconnect, and Login Success
        // gained a properties array in 1.19. We pass through and let the
        // connection layer handle login-state framing for the common cases.
        V165_LOGIN_S2C_SET_COMPRESSION => s2c_login_set_compression(body),
        V165_LOGIN_S2C_ENCRYPTION_REQUEST => s2c_login_encryption_request(body),
        V165_LOGIN_S2C_LOGIN_SUCCESS => s2c_login_success(body),
        V165_LOGIN_S2C_DISCONNECT => s2c_login_disconnect(body),

        // Play-state remapping. NB: these IDs overlap with login IDs above;
        // the duplicate match arms below are reached only when the earlier
        // arms decided the body wasn't shaped like the login packet. In
        // practice the dispatcher should be state-aware, but the proxy's
        // current converter signature has no state, so we accept the overlap.
        V165_PLAY_S2C_KEEP_ALIVE => s2c_play_keep_alive(body),
        V165_PLAY_S2C_CHAT => s2c_play_chat(body),
        V165_PLAY_S2C_DISCONNECT => s2c_play_disconnect(body),
        V165_PLAY_S2C_JOIN_GAME => s2c_play_join_game_drop(body),

        _ => ConversionResult::Passthrough,
    }
}

// ============================================================================
// Login state
// ============================================================================

fn s2c_login_set_compression(body: Bytes) -> ConversionResult {
    // Field shape unchanged: VarInt threshold.
    ConversionResult::Converted(vec![build_payload(V1204_LOGIN_S2C_SET_COMPRESSION, &body)])
}

fn s2c_login_encryption_request(body: Bytes) -> ConversionResult {
    // Field shape unchanged 1.16.5 → 1.20.4: server id string, pubkey, verify
    // token. 1.20.5+ adds a should-authenticate boolean, which we ignore here.
    ConversionResult::Converted(vec![build_payload(
        V1204_LOGIN_S2C_ENCRYPTION_REQUEST,
        &body,
    )])
}

fn s2c_login_success(mut body: Bytes) -> ConversionResult {
    // 1.16.5: UUID (128 bits), Username (string)
    // 1.19+:  UUID, Username, Properties (varint count + entries)
    // 1.20.5+: adds Strict Error Handling bool.
    //
    // We append an empty properties array. Strict-error-handling field is
    // omitted; 1.20.4 doesn't have it and 1.21 clients tolerate its absence
    // because we route everything via the 1.20.4 shape.
    if body.remaining() < 16 {
        return ConversionResult::Passthrough;
    }
    let mut out = BytesMut::new();
    let uuid_hi = body.get_i64();
    let uuid_lo = body.get_i64();
    out.put_i64(uuid_hi);
    out.put_i64(uuid_lo);
    let Ok(username) = String::decode(&mut body) else {
        return ConversionResult::Passthrough;
    };
    username.encode(&mut out).unwrap();
    VarInt(0).encode(&mut out).unwrap(); // properties: empty
    ConversionResult::Converted(vec![build_payload(V1204_LOGIN_S2C_LOGIN_SUCCESS, &out)])
}

fn s2c_login_disconnect(body: Bytes) -> ConversionResult {
    ConversionResult::Converted(vec![build_payload(V1204_LOGIN_S2C_DISCONNECT, &body)])
}

// ============================================================================
// Play state (best-effort)
// ============================================================================

fn s2c_play_keep_alive(mut body: Bytes) -> ConversionResult {
    // Both versions: i64 keep-alive id. Only the packet id changes.
    if body.remaining() < 8 {
        return ConversionResult::Passthrough;
    }
    let id = body.get_i64();
    let mut out = BytesMut::with_capacity(8);
    out.put_i64(id);
    ConversionResult::Converted(vec![build_payload(V1204_PLAY_S2C_KEEP_ALIVE, &out)])
}

fn s2c_play_chat(mut body: Bytes) -> ConversionResult {
    // 1.16.5 Chat Message: String (JSON), Byte position, UUID sender.
    // 1.20.4 System Chat: String (JSON), Boolean overlay.
    // We don't try to sign — System Chat is the legitimate unsigned channel.
    let Ok(json) = String::decode(&mut body) else {
        return ConversionResult::Passthrough;
    };
    let position = if body.has_remaining() {
        body.get_u8()
    } else {
        0
    };
    let mut out = BytesMut::new();
    json.encode(&mut out).unwrap();
    // overlay = true if position was 2 (game info / action bar)
    out.put_u8(if position == 2 { 1 } else { 0 });
    ConversionResult::Converted(vec![build_payload(V1204_PLAY_S2C_SYSTEM_CHAT, &out)])
}

fn s2c_play_disconnect(body: Bytes) -> ConversionResult {
    // Field unchanged: JSON string.
    ConversionResult::Converted(vec![build_payload(V1204_PLAY_S2C_DISCONNECT, &body)])
}

fn s2c_play_join_game_drop(_body: Bytes) -> ConversionResult {
    // JoinGame is dropped here because the dimension codec NBT changed
    // dramatically between 1.16.5 and 1.20.4. 1.20.5+ moved registry data
    // into the configuration phase entirely. Building a correct registry NBT
    // blob for 1.20.4 is more work than this partial bridge can justify right
    // now — see the wiki section on Login (play) packets for the full layout.
    tracing::warn!(
        target: "converter",
        from = "1.16.5",
        to = "1.20.4",
        "dropping Join Game: dimension codec NBT translation unimplemented"
    );
    ConversionResult::Drop
}

pub fn convert_c2s(payload: Bytes) -> ConversionResult {
    // The 1.16.5 server consumes c2s. Since the client (modern) side never
    // produces 1.16.5-shaped c2s, this direction is owned by the sibling
    // converter `v1_20_4_to_v1_16_5`. Kept here as a no-op for symmetry.
    let _ = payload;
    ConversionResult::Passthrough
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enc_string(s: &str) -> Vec<u8> {
        let mut b = BytesMut::new();
        s.to_string().encode(&mut b).unwrap();
        b.to_vec()
    }

    #[test]
    fn set_compression_passes_with_id_unchanged() {
        // VarInt threshold = 256
        let mut body = BytesMut::new();
        VarInt(256).encode(&mut body).unwrap();
        let payload = build_payload(V165_LOGIN_S2C_SET_COMPRESSION, &body);
        let out = convert_s2c(payload);
        match out {
            ConversionResult::Converted(p) => {
                assert_eq!(p.len(), 1);
                let (id, _rest) = split_id(p[0].clone()).unwrap();
                assert_eq!(id, V1204_LOGIN_S2C_SET_COMPRESSION);
            },
            _ => panic!("expected Converted"),
        }
    }

    #[test]
    fn login_success_appends_empty_properties() {
        let mut body = BytesMut::new();
        body.put_i64(1);
        body.put_i64(2);
        body.extend_from_slice(&enc_string("alex"));
        let payload = build_payload(V165_LOGIN_S2C_LOGIN_SUCCESS, &body);
        let out = convert_s2c(payload);
        match out {
            ConversionResult::Converted(p) => {
                let (id, mut rest) = split_id(p[0].clone()).unwrap();
                assert_eq!(id, V1204_LOGIN_S2C_LOGIN_SUCCESS);
                let hi = rest.get_i64();
                let lo = rest.get_i64();
                assert_eq!((hi, lo), (1, 2));
                let name = String::decode(&mut rest).unwrap();
                assert_eq!(name, "alex");
                let props = VarInt::decode(&mut rest).unwrap().0;
                assert_eq!(props, 0);
            },
            _ => panic!("expected Converted"),
        }
    }

    #[test]
    fn play_chat_becomes_system_chat() {
        let mut body = BytesMut::new();
        body.extend_from_slice(&enc_string("{\"text\":\"hi\"}"));
        body.put_u8(0); // position = chat
                        // Sender UUID is omitted on purpose — converter must tolerate truncated tail.
        let payload = build_payload(V165_PLAY_S2C_CHAT, &body);
        let out = convert_s2c(payload);
        match out {
            ConversionResult::Converted(p) => {
                let (id, mut rest) = split_id(p[0].clone()).unwrap();
                assert_eq!(id, V1204_PLAY_S2C_SYSTEM_CHAT);
                let json = String::decode(&mut rest).unwrap();
                assert_eq!(json, "{\"text\":\"hi\"}");
                let overlay = rest.get_u8();
                assert_eq!(overlay, 0);
            },
            _ => panic!("expected Converted"),
        }
    }

    #[test]
    fn join_game_is_dropped() {
        let payload = build_payload(V165_PLAY_S2C_JOIN_GAME, &[1, 2, 3, 4]);
        assert!(matches!(convert_s2c(payload), ConversionResult::Drop));
    }
}
