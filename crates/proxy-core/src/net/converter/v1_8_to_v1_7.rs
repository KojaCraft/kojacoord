//! 1.8.x (protocol 47) ↔ 1.7.10 (protocol 5) translation.
//!
//! IMPORTANT: this module is on the 1.12.2 → 1.7 server-to-client pipeline:
//! `modern_to_v1_8::convert_s2c → v1_8_to_v1_7::convert_s2c`. Many "v1_8"
//! packets produced by `modern_to_v1_8` already carry 1.7-style separate-int
//! coordinates rather than real 1.8 packed-long Positions (see
//! `modern_to_v1_8::s2c_block_change`). Those are forwarded as-is. Packets
//! that genuinely differ in wire format between real 1.7 and 1.8 — chat
//! (added position byte), PlayerPositionAndLook (added stance), JoinGame
//! (added reduced_debug_info), Experience (i16 vs varint), entity destroy
//! (varint count and ids) — are translated here.

use bytes::{Buf, BufMut, Bytes, BytesMut};
use kojacoord_protocol::codec::{Decode, Encode};
use kojacoord_protocol::types::VarInt;

use super::{build_payload, split_id};
use crate::converter::ConversionResult;

// ── 1.8 S2C packet IDs ──────────────────────────────────────────────────────
const V18_S2C_JOIN_GAME: u8 = 0x01;
const V18_S2C_CHAT: u8 = 0x02;
const V18_S2C_SPAWN_POSITION: u8 = 0x05;
const V18_S2C_PLAYER_POS_LOOK: u8 = 0x08;
const V18_S2C_ENTITY: u8 = 0x0E;
const V18_S2C_ENTITY_DESTROY: u8 = 0x13;
const V18_S2C_ENTITY_REL_MOVE: u8 = 0x15;
const V18_S2C_ENTITY_TELEPORT: u8 = 0x18;
const V18_S2C_ENTITY_METADATA: u8 = 0x1C;
const V18_S2C_EXPERIENCE: u8 = 0x1F;
const V18_S2C_BLOCK_CHANGE: u8 = 0x23;
const V18_S2C_UPDATE_SIGN: u8 = 0x33;
const V18_S2C_STATISTICS: u8 = 0x37;
const V18_S2C_PLAYER_LIST_ITEM: u8 = 0x38;
const V18_S2C_TAB_COMPLETE: u8 = 0x3A;
const V18_S2C_SCOREBOARD_TEAM: u8 = 0x3E;
const V18_S2C_SET_SLOT: u8 = 0x2F;
const V18_S2C_WINDOW_ITEMS: u8 = 0x30;
const V18_S2C_EQUIPMENT: u8 = 0x04;

// ── 1.7 S2C packet IDs ──────────────────────────────────────────────────────
const V17_S2C_JOIN_GAME: u8 = 0x01;
const V17_S2C_CHAT: u8 = 0x02;
const V17_S2C_SPAWN_POSITION: u8 = 0x05;
const V17_S2C_PLAYER_POS_LOOK: u8 = 0x08;
const V17_S2C_ENTITY: u8 = 0x0E;
const V17_S2C_ENTITY_DESTROY: u8 = 0x13;
const V17_S2C_ENTITY_REL_MOVE: u8 = 0x15;
const V17_S2C_ENTITY_TELEPORT: u8 = 0x18;
const V17_S2C_ENTITY_METADATA: u8 = 0x1C;
const V17_S2C_EXPERIENCE: u8 = 0x1D;
const V17_S2C_BLOCK_CHANGE: u8 = 0x23;
const V17_S2C_UPDATE_SIGN: u8 = 0x33;
const V17_S2C_STATISTICS: u8 = 0x37;
const V17_S2C_TAB_COMPLETE: u8 = 0x3A;
const V17_S2C_SCOREBOARD_TEAM: u8 = 0x3E;
const V17_S2C_SET_SLOT: u8 = 0x2F;
const V17_S2C_WINDOW_ITEMS: u8 = 0x30;
const V17_S2C_EQUIPMENT: u8 = 0x04;

// ── C2S packet IDs (identical between 1.7 and 1.8 for these) ────────────────
const V18_C2S_CHAT: u8 = 0x01;
const V18_C2S_USE_ENTITY: u8 = 0x02;
const V18_C2S_PLAYER_POS_LOOK: u8 = 0x06;
const V18_C2S_PLAYER_DIGGING: u8 = 0x07;
const V18_C2S_PLAYER_BLOCK_PLACE: u8 = 0x08;
const V18_C2S_UPDATE_SIGN: u8 = 0x12;

const V17_C2S_USE_ENTITY: u8 = 0x02;
const V17_C2S_PLAYER_POS_LOOK: u8 = 0x06;
const V17_C2S_PLAYER_DIGGING: u8 = 0x07;
const V17_C2S_PLAYER_BLOCK_PLACE: u8 = 0x08;
const V17_C2S_UPDATE_SIGN: u8 = 0x12;

const STANCE_OFFSET: f64 = 1.62;

// ──────────────────────────────────────────────────────────────────────────
// Helpers
// ──────────────────────────────────────────────────────────────────────────

fn unpack_position(packed: i64) -> (i32, i32, i32) {
    let mut x = (packed >> 38) & 0x3FF_FFFF;
    let mut y = packed & 0xFFF;
    let mut z = (packed >> 12) & 0x3FF_FFFF;
    if x >= 0x200_0000 {
        x -= 0x400_0000;
    }
    if z >= 0x200_0000 {
        z -= 0x400_0000;
    }
    if y >= 0x800 {
        y -= 0x1000;
    }
    (x as i32, y as i32, z as i32)
}

// ──────────────────────────────────────────────────────────────────────────
// S2C: 1.8 server (or proxy-emitted "v1_8") → 1.7 client
// ──────────────────────────────────────────────────────────────────────────

pub fn convert_s2c(payload: Bytes) -> ConversionResult {
    let Some((id, body)) = split_id(payload.clone()) else {
        return ConversionResult::Passthrough;
    };

    match id {
        V18_S2C_JOIN_GAME => s2c_join_game(body),
        V18_S2C_CHAT => s2c_chat(body),
        V18_S2C_SPAWN_POSITION => s2c_spawn_position(body),
        V18_S2C_PLAYER_POS_LOOK => s2c_player_pos_look(body),
        V18_S2C_ENTITY => ConversionResult::Converted(vec![build_payload(V17_S2C_ENTITY, &body)]),
        V18_S2C_ENTITY_DESTROY => s2c_entity_destroy(body),
        V18_S2C_ENTITY_REL_MOVE => {
            ConversionResult::Converted(vec![build_payload(V17_S2C_ENTITY_REL_MOVE, &body)])
        },
        V18_S2C_ENTITY_TELEPORT => {
            ConversionResult::Converted(vec![build_payload(V17_S2C_ENTITY_TELEPORT, &body)])
        },
        V18_S2C_ENTITY_METADATA => {
            ConversionResult::Converted(vec![build_payload(V17_S2C_ENTITY_METADATA, &body)])
        },
        V18_S2C_EXPERIENCE => s2c_experience(body),
        V18_S2C_BLOCK_CHANGE => {
            // modern_to_v1_8 emits 1.7-style separate-int coords; passthrough id.
            ConversionResult::Converted(vec![build_payload(V17_S2C_BLOCK_CHANGE, &body)])
        },
        V18_S2C_UPDATE_SIGN => s2c_update_sign(body),
        V18_S2C_STATISTICS => {
            // 1.8 and 1.7 carry the same `varint count; (string, varint)[]` shape after
            // the 1.7.6 update; safe to pass through.
            ConversionResult::Converted(vec![build_payload(V17_S2C_STATISTICS, &body)])
        },
        V18_S2C_PLAYER_LIST_ITEM => {
            // 1.8 player list uses an action-based, UUID-keyed wire format that
            // has no 1.7 equivalent. Drop rather than emit garbage.
            tracing::debug!(target: "converter", "v1_8→v1_7: player list item dropped (incompatible action-based format)");
            ConversionResult::Drop
        },
        V18_S2C_TAB_COMPLETE => s2c_tab_complete(body),
        V18_S2C_SCOREBOARD_TEAM => s2c_scoreboard_team(body),
        V18_S2C_SET_SLOT => {
            ConversionResult::Converted(vec![build_payload(V17_S2C_SET_SLOT, &body)])
        },
        V18_S2C_WINDOW_ITEMS => {
            ConversionResult::Converted(vec![build_payload(V17_S2C_WINDOW_ITEMS, &body)])
        },
        V18_S2C_EQUIPMENT => {
            ConversionResult::Converted(vec![build_payload(V17_S2C_EQUIPMENT, &body)])
        },
        _ => ConversionResult::Passthrough,
    }
}

fn s2c_join_game(body: Bytes) -> ConversionResult {
    // 1.8 has a trailing `reduced_debug_info` byte we strip.
    if body.is_empty() {
        return ConversionResult::Passthrough;
    }
    let trimmed = body.slice(..body.len().saturating_sub(1));
    ConversionResult::Converted(vec![build_payload(V17_S2C_JOIN_GAME, &trimmed)])
}

fn s2c_chat(mut body: Bytes) -> ConversionResult {
    // 1.8: string json + u8 position. 1.7: string json. Drop above-hotbar (2).
    let Ok(json) = String::decode(&mut body) else {
        return ConversionResult::Passthrough;
    };
    let position = if body.has_remaining() {
        body.get_u8()
    } else {
        0
    };
    if position == 2 {
        return ConversionResult::Drop;
    }
    let mut out = BytesMut::new();
    json.encode(&mut out).unwrap();
    ConversionResult::Converted(vec![build_payload(V17_S2C_CHAT, &out)])
}

fn s2c_spawn_position(mut body: Bytes) -> ConversionResult {
    // 1.8: packed Position. 1.7: i32 x, y, z.
    if body.remaining() < 8 {
        return ConversionResult::Passthrough;
    }
    let packed = body.get_i64();
    let (x, y, z) = unpack_position(packed);
    let mut out = BytesMut::with_capacity(12);
    out.put_i32(x);
    out.put_i32(y);
    out.put_i32(z);
    ConversionResult::Converted(vec![build_payload(V17_S2C_SPAWN_POSITION, &out)])
}

fn s2c_player_pos_look(mut body: Bytes) -> ConversionResult {
    // 1.8 wire: x, y, z, yaw, pitch, flags (33 bytes).
    // 1.7 wire: x, stance, y, z, yaw, pitch, on_ground (41 bytes).
    if body.remaining() < 33 {
        return ConversionResult::Passthrough;
    }
    let x = body.get_f64();
    let y = body.get_f64();
    let z = body.get_f64();
    let yaw = body.get_f32();
    let pitch = body.get_f32();
    let _flags = body.get_u8();

    let mut out = BytesMut::with_capacity(41);
    out.put_f64(x);
    out.put_f64(y + STANCE_OFFSET);
    out.put_f64(y);
    out.put_f64(z);
    out.put_f32(yaw);
    out.put_f32(pitch);
    out.put_u8(0);
    ConversionResult::Converted(vec![build_payload(V17_S2C_PLAYER_POS_LOOK, &out)])
}

fn s2c_entity_destroy(mut body: Bytes) -> ConversionResult {
    // 1.8: varint count; varint[] ids. 1.7: i8 count; i32[] ids.
    let count = match VarInt::decode(&mut body) {
        Ok(v) => v.0,
        Err(_) => return ConversionResult::Passthrough,
    };
    if !(0..=127).contains(&count) {
        return ConversionResult::Passthrough;
    }
    let mut out = BytesMut::new();
    out.put_i8(count as i8);
    for _ in 0..count {
        let id = match VarInt::decode(&mut body) {
            Ok(v) => v.0,
            Err(_) => return ConversionResult::Passthrough,
        };
        out.put_i32(id);
    }
    ConversionResult::Converted(vec![build_payload(V17_S2C_ENTITY_DESTROY, &out)])
}

fn s2c_experience(mut body: Bytes) -> ConversionResult {
    // 1.8: f32, varint, varint. 1.7: f32, i16, i16.
    if body.remaining() < 4 {
        return ConversionResult::Passthrough;
    }
    let bar = body.get_f32();
    let level = match VarInt::decode(&mut body) {
        Ok(v) => v.0,
        Err(_) => return ConversionResult::Passthrough,
    };
    let total = match VarInt::decode(&mut body) {
        Ok(v) => v.0,
        Err(_) => return ConversionResult::Passthrough,
    };
    let mut out = BytesMut::new();
    out.put_f32(bar);
    out.put_i16(level.clamp(i16::MIN as i32, i16::MAX as i32) as i16);
    out.put_i16(total.clamp(i16::MIN as i32, i16::MAX as i32) as i16);
    ConversionResult::Converted(vec![build_payload(V17_S2C_EXPERIENCE, &out)])
}

fn s2c_update_sign(mut body: Bytes) -> ConversionResult {
    // 1.8: Position; 4 JSON-chat strings. 1.7: i32 x; i16 y; i32 z; 4 raw strings.
    if body.remaining() < 8 {
        return ConversionResult::Passthrough;
    }
    let packed = body.get_i64();
    let (x, y, z) = unpack_position(packed);

    let mut out = BytesMut::new();
    out.put_i32(x);
    out.put_i16(y as i16);
    out.put_i32(z);
    for _ in 0..4 {
        let Ok(json_line) = String::decode(&mut body) else {
            return ConversionResult::Passthrough;
        };
        let raw = strip_chat_json(&json_line);
        raw.encode(&mut out).unwrap();
    }
    ConversionResult::Converted(vec![build_payload(V17_S2C_UPDATE_SIGN, &out)])
}

/// Best-effort extraction of plain text from a chat-component JSON string.
/// Falls back to the original string if parsing fails.
fn strip_chat_json(s: &str) -> String {
    // Look for `"text":"..."` first.
    if let Some(start) = s.find("\"text\"") {
        let after = &s[start + 6..];
        if let Some(colon) = after.find(':') {
            let after = &after[colon + 1..].trim_start();
            if let Some(rest) = after.strip_prefix('"') {
                let mut out = String::new();
                let mut chars = rest.chars();
                while let Some(c) = chars.next() {
                    match c {
                        '"' => return out,
                        '\\' => {
                            if let Some(n) = chars.next() {
                                match n {
                                    'n' => out.push('\n'),
                                    'r' => out.push('\r'),
                                    't' => out.push('\t'),
                                    '"' => out.push('"'),
                                    '\\' => out.push('\\'),
                                    other => out.push(other),
                                }
                            }
                        },
                        c => out.push(c),
                    }
                }
                return out;
            }
        }
    }
    s.to_string()
}

fn s2c_tab_complete(mut body: Bytes) -> ConversionResult {
    // 1.8: varint count; string[]. 1.7: single NUL-joined string.
    let count = match VarInt::decode(&mut body) {
        Ok(v) => v.0,
        Err(_) => return ConversionResult::Passthrough,
    };
    if !(0..=1024).contains(&count) {
        return ConversionResult::Passthrough;
    }
    let mut parts = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let Ok(s) = String::decode(&mut body) else {
            return ConversionResult::Passthrough;
        };
        parts.push(s);
    }
    let joined = parts.join("\0");
    let mut out = BytesMut::new();
    joined.encode(&mut out).unwrap();
    ConversionResult::Converted(vec![build_payload(V17_S2C_TAB_COMPLETE, &out)])
}

fn s2c_scoreboard_team(mut body: Bytes) -> ConversionResult {
    // We can't easily strip the 1.8 name-tag-visibility field without parsing all
    // earlier strings; pass through as-is. 1.7 clients tolerate extra trailing bytes.
    let Ok(team_name) = String::decode(&mut body) else {
        return ConversionResult::Passthrough;
    };
    let mut out = BytesMut::new();
    team_name.encode(&mut out).unwrap();
    out.extend_from_slice(&body);
    ConversionResult::Converted(vec![build_payload(V17_S2C_SCOREBOARD_TEAM, &out)])
}

// ──────────────────────────────────────────────────────────────────────────
// C2S: 1.8 client → 1.7 server
// ──────────────────────────────────────────────────────────────────────────

pub fn convert_c2s(payload: Bytes) -> ConversionResult {
    let Some((id, body)) = split_id(payload.clone()) else {
        return ConversionResult::Passthrough;
    };

    match id {
        V18_C2S_CHAT => ConversionResult::Passthrough,
        V18_C2S_USE_ENTITY => c2s_use_entity(body),
        V18_C2S_PLAYER_POS_LOOK => c2s_player_pos_look(body),
        V18_C2S_PLAYER_DIGGING => c2s_player_digging(body),
        V18_C2S_PLAYER_BLOCK_PLACE => c2s_player_block_place(body),
        V18_C2S_UPDATE_SIGN => c2s_update_sign(body),
        _ => ConversionResult::Passthrough,
    }
}

fn c2s_use_entity(mut body: Bytes) -> ConversionResult {
    // 1.8: varint target; varint type; (if type==2: f32 x, y, z).
    // 1.7: i32 target; i8 mouse (0=interact/right, 1=attack/left).
    let target = match VarInt::decode(&mut body) {
        Ok(v) => v.0,
        Err(_) => return ConversionResult::Passthrough,
    };
    let typ = match VarInt::decode(&mut body) {
        Ok(v) => v.0,
        Err(_) => return ConversionResult::Passthrough,
    };
    let mouse: i8 = match typ {
        1 => 1, // attack
        _ => 0, // interact / interact_at both map to "right click"
    };
    let mut out = BytesMut::new();
    out.put_i32(target);
    out.put_i8(mouse);
    ConversionResult::Converted(vec![build_payload(V17_C2S_USE_ENTITY, &out)])
}

fn c2s_player_pos_look(mut body: Bytes) -> ConversionResult {
    // 1.8: x, feet_y, z, yaw, pitch, on_ground (33 bytes).
    // 1.7: x, feet_y, stance(=feet_y+1.62), z, yaw, pitch, on_ground (41 bytes).
    if body.remaining() < 33 {
        return ConversionResult::Passthrough;
    }
    let x = body.get_f64();
    let y = body.get_f64();
    let z = body.get_f64();
    let yaw = body.get_f32();
    let pitch = body.get_f32();
    let on_ground = body.get_u8();

    let mut out = BytesMut::with_capacity(41);
    out.put_f64(x);
    out.put_f64(y);
    out.put_f64(y + STANCE_OFFSET);
    out.put_f64(z);
    out.put_f32(yaw);
    out.put_f32(pitch);
    out.put_u8(on_ground);
    ConversionResult::Converted(vec![build_payload(V17_C2S_PLAYER_POS_LOOK, &out)])
}

fn c2s_player_digging(mut body: Bytes) -> ConversionResult {
    // 1.8: i8 status; Position; i8 face. 1.7: i8 status; i32 x; u8 y; i32 z; i8 face.
    if body.remaining() < 10 {
        return ConversionResult::Passthrough;
    }
    let status = body.get_i8();
    let packed = body.get_i64();
    let face = body.get_i8();
    let (x, y, z) = unpack_position(packed);

    let mut out = BytesMut::new();
    out.put_i8(status);
    out.put_i32(x);
    out.put_u8(y as u8);
    out.put_i32(z);
    out.put_i8(face);
    ConversionResult::Converted(vec![build_payload(V17_C2S_PLAYER_DIGGING, &out)])
}

fn c2s_player_block_place(mut body: Bytes) -> ConversionResult {
    // 1.8: Position; i8 dir; slot; i8 cx; i8 cy; i8 cz.
    // 1.7: i32 x; u8 y; i32 z; i8 dir; slot; i8 cx; i8 cy; i8 cz.
    if body.remaining() < 9 {
        return ConversionResult::Passthrough;
    }
    let packed = body.get_i64();
    let dir = body.get_i8();
    let (x, y, z) = unpack_position(packed);

    let mut out = BytesMut::new();
    out.put_i32(x);
    out.put_u8(y as u8);
    out.put_i32(z);
    out.put_i8(dir);
    out.extend_from_slice(&body);
    ConversionResult::Converted(vec![build_payload(V17_C2S_PLAYER_BLOCK_PLACE, &out)])
}

fn c2s_update_sign(mut body: Bytes) -> ConversionResult {
    // 1.8: Position; 4 JSON-chat strings. 1.7: i32 x; i16 y; i32 z; 4 raw strings.
    if body.remaining() < 8 {
        return ConversionResult::Passthrough;
    }
    let packed = body.get_i64();
    let (x, y, z) = unpack_position(packed);

    let mut out = BytesMut::new();
    out.put_i32(x);
    out.put_i16(y as i16);
    out.put_i32(z);
    for _ in 0..4 {
        let Ok(json_line) = String::decode(&mut body) else {
            return ConversionResult::Passthrough;
        };
        let raw = strip_chat_json(&json_line);
        raw.encode(&mut out).unwrap();
    }
    ConversionResult::Converted(vec![build_payload(V17_C2S_UPDATE_SIGN, &out)])
}

// ──────────────────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn decode_one(r: ConversionResult) -> Bytes {
        match r {
            ConversionResult::Converted(mut v) => {
                assert_eq!(v.len(), 1);
                v.remove(0)
            },
            _ => panic!("expected Converted"),
        }
    }

    fn pack_pos(x: i64, y: i64, z: i64) -> i64 {
        ((x & 0x3FF_FFFF) << 38) | ((z & 0x3FF_FFFF) << 12) | (y & 0xFFF)
    }

    #[test]
    fn position_unpack_roundtrip() {
        let cases = [
            (0, 0, 0),
            (100, 64, -100),
            (33554431, 2047, -33554432),
            (-1, -2048, -1),
        ];
        for (x, y, z) in cases {
            let packed = pack_pos(x as i64, y as i64, z as i64);
            assert_eq!(unpack_position(packed), (x, y, z));
        }
    }

    #[test]
    fn chat_strips_position_byte() {
        let mut body = BytesMut::new();
        "{\"text\":\"hi\"}".to_owned().encode(&mut body).unwrap();
        body.put_u8(1); // system
        let r = s2c_chat(body.freeze());
        let pkt = decode_one(r);
        let (id, mut rest) = split_id(pkt).unwrap();
        assert_eq!(id, V17_S2C_CHAT);
        assert_eq!(String::decode(&mut rest).unwrap(), "{\"text\":\"hi\"}");
        assert!(!rest.has_remaining());
    }

    #[test]
    fn chat_above_hotbar_dropped() {
        let mut body = BytesMut::new();
        "{\"text\":\"x\"}".to_owned().encode(&mut body).unwrap();
        body.put_u8(2);
        assert!(matches!(s2c_chat(body.freeze()), ConversionResult::Drop));
    }

    #[test]
    fn player_pos_look_adds_stance() {
        let mut body = BytesMut::new();
        body.put_f64(1.0);
        body.put_f64(70.0); // feet_y
        body.put_f64(2.0);
        body.put_f32(45.0);
        body.put_f32(10.0);
        body.put_u8(0);
        let r = s2c_player_pos_look(body.freeze());
        let pkt = decode_one(r);
        let (id, mut rest) = split_id(pkt).unwrap();
        assert_eq!(id, V17_S2C_PLAYER_POS_LOOK);
        assert_eq!(rest.get_f64(), 1.0);
        let stance = rest.get_f64();
        let y = rest.get_f64();
        assert!((stance - (y + STANCE_OFFSET)).abs() < 1e-9);
        assert_eq!(y, 70.0);
        assert_eq!(rest.get_f64(), 2.0);
    }

    #[test]
    fn entity_destroy_narrows() {
        let mut body = BytesMut::new();
        VarInt(2).encode(&mut body).unwrap();
        VarInt(7).encode(&mut body).unwrap();
        VarInt(42).encode(&mut body).unwrap();
        let r = s2c_entity_destroy(body.freeze());
        let pkt = decode_one(r);
        let (id, mut rest) = split_id(pkt).unwrap();
        assert_eq!(id, V17_S2C_ENTITY_DESTROY);
        assert_eq!(rest.get_i8(), 2);
        assert_eq!(rest.get_i32(), 7);
        assert_eq!(rest.get_i32(), 42);
    }

    #[test]
    fn c2s_digging_unpacks_position() {
        let packed = pack_pos(123, 64, -7);
        let mut body = BytesMut::new();
        body.put_i8(0);
        body.put_i64(packed);
        body.put_i8(1);
        let r = c2s_player_digging(body.freeze());
        let pkt = decode_one(r);
        let (id, mut rest) = split_id(pkt).unwrap();
        assert_eq!(id, V17_C2S_PLAYER_DIGGING);
        assert_eq!(rest.get_i8(), 0);
        assert_eq!(rest.get_i32(), 123);
        assert_eq!(rest.get_u8(), 64);
        assert_eq!(rest.get_i32(), -7);
        assert_eq!(rest.get_i8(), 1);
    }

    #[test]
    fn strip_chat_json_extracts_text() {
        assert_eq!(strip_chat_json("{\"text\":\"hello\"}"), "hello");
        assert_eq!(strip_chat_json("{\"text\":\"a\\nb\"}"), "a\nb");
        assert_eq!(strip_chat_json("plain"), "plain");
    }
}
