//! 1.7.10 (protocol 5) ↔ 1.8.x (protocol 47) translation, server→client and
//! client→server.
//!
//! References used while writing this module (no live web access — drawn from
//! the canonical wire-format spec on minecraft.wiki / wiki.vg mirror and the
//! PrismarineJS minecraft-data 1.7 / 1.8 proto.yml files):
//!
//! Key wire-format differences from 1.7.10 → 1.8:
//! * Position encoding: 1.7 uses separate `i32 x, u8 y, i32 z`; 1.8 introduced
//!   the packed-long `Position` (26+12+26 bits).
//! * Chat (S2C): 1.8 added a trailing `position: byte` field (0 = chat,
//!   1 = system, 2 = above hotbar).
//! * Player Position And Look (S2C 0x08): 1.7 has 4 doubles (x, stance, y, z)
//!   plus yaw/pitch/on_ground; 1.8 drops `stance` (3 doubles) and replaces
//!   `on_ground` with a `flags` byte.
//! * Use Entity (C2S 0x02): 1.8 added a varint `type` (0=interact, 1=attack,
//!   2=interact_at + x/y/z floats); 1.7 was `i32 target, i8 mouse`.
//! * Player Digging / Block Placement: 1.7 uses split coords + u8 Y; 1.8 uses
//!   the packed Position.
//! * Update Sign (S2C/C2S): 1.7 sends raw text lines, 1.8 sends JSON chat
//!   components and uses Position.
//! * Window Click: a Transaction ID byte layout changed slightly (no actual
//!   wire-bytes diff — the action enum extended).
//! * Statistics (S2C): 1.7 array of strings; 1.8 array of `(string, varint)`.
//! * Tab Complete (S2C): a different shape (1.8 wraps in count of strings).
//! * Spawn Position (S2C 0x05): coord triple → packed long Position.
//!
//! Internal proxy convention: many "v1_8" packets stored by the proxy after a
//! `modern_to_v1_8` pass still use 1.7-style separate-int coordinates (see
//! `modern_to_v1_8::s2c_block_change` for the canonical example). Because the
//! 1.12.2 → 1.7 path goes `modern_to_v1_8 → v1_8_to_v1_7`, this converter must
//! treat that internal convention as authoritative for the S2C direction.

use bytes::{Buf, BufMut, Bytes, BytesMut};
use kojacoord_protocol::codec::{Decode, Encode};
use kojacoord_protocol::types::VarInt;

use super::{build_payload, split_id};
use crate::converter::ConversionResult;

// ── 1.7 (protocol 5) S2C packet IDs ─────────────────────────────────────────
const V17_S2C_KEEP_ALIVE: u8 = 0x00;
const V17_S2C_JOIN_GAME: u8 = 0x01;
const V17_S2C_CHAT: u8 = 0x02;
const V17_S2C_EQUIPMENT: u8 = 0x04;
const V17_S2C_SPAWN_POSITION: u8 = 0x05;
const V17_S2C_PLAYER_POS_LOOK: u8 = 0x08;
const V17_S2C_ENTITY: u8 = 0x0E;
const V17_S2C_ENTITY_DESTROY: u8 = 0x13;
const V17_S2C_ENTITY_REL_MOVE: u8 = 0x15;
const V17_S2C_ENTITY_TELEPORT: u8 = 0x18;
const V17_S2C_ENTITY_METADATA: u8 = 0x1C;
const V17_S2C_EXPERIENCE: u8 = 0x1F;
const V17_S2C_BLOCK_CHANGE: u8 = 0x23;
const V17_S2C_CHUNK_BULK: u8 = 0x26;
const V17_S2C_UPDATE_SIGN: u8 = 0x33;
const V17_S2C_STATISTICS: u8 = 0x37;
const V17_S2C_PLAYER_LIST_ITEM: u8 = 0x38;
const V17_S2C_TAB_COMPLETE: u8 = 0x3A;
const V17_S2C_SCOREBOARD_OBJ: u8 = 0x3B;
const V17_S2C_SCOREBOARD_SCORE: u8 = 0x3C;
const V17_S2C_SCOREBOARD_TEAM: u8 = 0x3E;
// 1.7 packets without 1.8 equivalents that need dropping:
const V17_S2C_SET_SLOT: u8 = 0x2F;
const V17_S2C_WINDOW_ITEMS: u8 = 0x30;

// ── 1.8 (protocol 47) S2C packet IDs ────────────────────────────────────────
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
const V18_S2C_TAB_COMPLETE: u8 = 0x3A;
const V18_S2C_SCOREBOARD_TEAM: u8 = 0x3E;
const V18_S2C_SCOREBOARD_OBJ: u8 = 0x3B;
const V18_S2C_SCOREBOARD_SCORE: u8 = 0x3C;
const V18_S2C_SET_SLOT: u8 = 0x2F;
const V18_S2C_WINDOW_ITEMS: u8 = 0x30;
const V18_S2C_EQUIPMENT: u8 = 0x04;

// ── 1.7 (protocol 5) C2S packet IDs ─────────────────────────────────────────
const V17_C2S_CHAT: u8 = 0x01;
const V17_C2S_USE_ENTITY: u8 = 0x02;
const V17_C2S_PLAYER_POS_LOOK: u8 = 0x06;
const V17_C2S_PLAYER_DIGGING: u8 = 0x07;
const V17_C2S_PLAYER_BLOCK_PLACE: u8 = 0x08;
const V17_C2S_UPDATE_SIGN: u8 = 0x12;

// ── 1.8 (protocol 47) C2S packet IDs ────────────────────────────────────────
#[allow(dead_code)]
const V18_C2S_CHAT: u8 = 0x01;
const V18_C2S_USE_ENTITY: u8 = 0x02;
const V18_C2S_PLAYER_POS_LOOK: u8 = 0x06;
const V18_C2S_PLAYER_DIGGING: u8 = 0x07;
const V18_C2S_PLAYER_BLOCK_PLACE: u8 = 0x08;
const V18_C2S_UPDATE_SIGN: u8 = 0x12;

// ──────────────────────────────────────────────────────────────────────────
// S2C: 1.7 server → 1.8 client
// ──────────────────────────────────────────────────────────────────────────

pub fn convert_s2c(payload: Bytes) -> ConversionResult {
    let Some((id, body)) = split_id(payload.clone()) else {
        return ConversionResult::Passthrough;
    };

    match id {
        V17_S2C_KEEP_ALIVE => ConversionResult::Passthrough,
        V17_S2C_JOIN_GAME => s2c_join_game(body),
        V17_S2C_CHAT => s2c_chat(body),
        V17_S2C_SPAWN_POSITION => s2c_spawn_position(body),
        V17_S2C_PLAYER_POS_LOOK => s2c_player_pos_look(body),
        V17_S2C_ENTITY => s2c_entity(body),
        V17_S2C_ENTITY_DESTROY => s2c_entity_destroy(body),
        V17_S2C_ENTITY_REL_MOVE => {
            ConversionResult::Converted(vec![build_payload(V18_S2C_ENTITY_REL_MOVE, &body)])
        },
        V17_S2C_ENTITY_TELEPORT => {
            ConversionResult::Converted(vec![build_payload(V18_S2C_ENTITY_TELEPORT, &body)])
        },
        V17_S2C_ENTITY_METADATA => {
            ConversionResult::Converted(vec![build_payload(V18_S2C_ENTITY_METADATA, &body)])
        },
        V17_S2C_EXPERIENCE => s2c_experience(body),
        V17_S2C_BLOCK_CHANGE => {
            // The in-repo "v1_8" BlockChange retains 1.7-style coords. Pass through.
            ConversionResult::Converted(vec![build_payload(V18_S2C_BLOCK_CHANGE, &body)])
        },
        V17_S2C_CHUNK_BULK => {
            tracing::debug!(target: "converter", "v1_7→v1_8: chunk bulk packet dropped (incompatible chunk format)");
            ConversionResult::Drop
        },
        V17_S2C_UPDATE_SIGN => s2c_update_sign(body),
        V17_S2C_STATISTICS => {
            // 1.7 and 1.8 statistics share the `varint count; (string, varint)[]`
            // shape (since 1.7.6); forward unchanged.
            ConversionResult::Converted(vec![build_payload(V18_S2C_STATISTICS, &body)])
        },
        V17_S2C_PLAYER_LIST_ITEM => {
            tracing::debug!(target: "converter", "v1_7→v1_8: player list item dropped (action-based format added in 1.8)");
            ConversionResult::Drop
        },
        V17_S2C_TAB_COMPLETE => s2c_tab_complete(body),
        V17_S2C_SCOREBOARD_OBJ => {
            ConversionResult::Converted(vec![build_payload(V18_S2C_SCOREBOARD_OBJ, &body)])
        },
        V17_S2C_SCOREBOARD_SCORE => {
            ConversionResult::Converted(vec![build_payload(V18_S2C_SCOREBOARD_SCORE, &body)])
        },
        V17_S2C_SCOREBOARD_TEAM => s2c_scoreboard_team(body),
        V17_S2C_SET_SLOT => {
            ConversionResult::Converted(vec![build_payload(V18_S2C_SET_SLOT, &body)])
        },
        V17_S2C_WINDOW_ITEMS => {
            ConversionResult::Converted(vec![build_payload(V18_S2C_WINDOW_ITEMS, &body)])
        },
        V17_S2C_EQUIPMENT => {
            ConversionResult::Converted(vec![build_payload(V18_S2C_EQUIPMENT, &body)])
        },
        _ => ConversionResult::Passthrough,
    }
}

fn s2c_join_game(body: Bytes) -> ConversionResult {
    // 1.7: i32 eid; u8 gamemode; i8 dimension; u8 difficulty; u8 maxPlayers; string levelType
    // 1.8: same + bool reduced_debug_info trailing byte.
    if body.is_empty() {
        return ConversionResult::Passthrough;
    }
    let mut out = BytesMut::with_capacity(body.len() + 1);
    out.extend_from_slice(&body);
    out.put_u8(0);
    ConversionResult::Converted(vec![build_payload(V18_S2C_JOIN_GAME, &out)])
}

fn s2c_chat(body: Bytes) -> ConversionResult {
    // 1.7: string json. 1.8: string json + u8 position.
    let mut out = BytesMut::with_capacity(body.len() + 1);
    out.extend_from_slice(&body);
    out.put_u8(0); // chat box
    ConversionResult::Converted(vec![build_payload(V18_S2C_CHAT, &out)])
}

fn s2c_spawn_position(mut body: Bytes) -> ConversionResult {
    // 1.7: i32 x; i32 y; i32 z. 1.8: packed Position (i64).
    if body.remaining() < 12 {
        return ConversionResult::Passthrough;
    }
    let x = body.get_i32() as i64;
    let y = body.get_i32() as i64;
    let z = body.get_i32() as i64;
    let packed = ((x & 0x3FF_FFFF) << 38) | ((z & 0x3FF_FFFF) << 12) | (y & 0xFFF);
    let mut out = BytesMut::with_capacity(8);
    out.put_i64(packed);
    ConversionResult::Converted(vec![build_payload(V18_S2C_SPAWN_POSITION, &out)])
}

fn s2c_player_pos_look(mut body: Bytes) -> ConversionResult {
    // 1.7 wire: f64 x; f64 stance; f64 y; f64 z; f32 yaw; f32 pitch; u8 onGround.
    // 1.8 wire: f64 x; f64 y; f64 z; f32 yaw; f32 pitch; u8 flags.
    if body.remaining() < 41 {
        return ConversionResult::Passthrough;
    }
    let x = body.get_f64();
    let _stance = body.get_f64();
    let y = body.get_f64();
    let z = body.get_f64();
    let yaw = body.get_f32();
    let pitch = body.get_f32();
    let _on_ground = body.get_u8();

    let mut out = BytesMut::with_capacity(33);
    out.put_f64(x);
    out.put_f64(y);
    out.put_f64(z);
    out.put_f32(yaw);
    out.put_f32(pitch);
    out.put_u8(0);
    ConversionResult::Converted(vec![build_payload(V18_S2C_PLAYER_POS_LOOK, &out)])
}

fn s2c_entity(body: Bytes) -> ConversionResult {
    // 1.7 Entity (0x0E): single varint entity_id. 1.8 same.
    ConversionResult::Converted(vec![build_payload(V18_S2C_ENTITY, &body)])
}

fn s2c_entity_destroy(mut body: Bytes) -> ConversionResult {
    // 1.7: i8 count; i32[] ids. 1.8: varint count; varint[] ids.
    if body.remaining() < 1 {
        return ConversionResult::Passthrough;
    }
    let count = body.get_i8() as i32;
    if count < 0 || (count as usize) * 4 > body.remaining() {
        return ConversionResult::Passthrough;
    }
    let mut out = BytesMut::new();
    VarInt(count).encode(&mut out).unwrap();
    for _ in 0..count {
        let id = body.get_i32();
        VarInt(id).encode(&mut out).unwrap();
    }
    ConversionResult::Converted(vec![build_payload(V18_S2C_ENTITY_DESTROY, &out)])
}

fn s2c_experience(mut body: Bytes) -> ConversionResult {
    // 1.7: f32 bar; i16 level; i16 total. 1.8: f32 bar; varint level; varint total.
    if body.remaining() < 8 {
        return ConversionResult::Passthrough;
    }
    let bar = body.get_f32();
    let level = body.get_i16() as i32;
    let total = body.get_i16() as i32;
    let mut out = BytesMut::new();
    out.put_f32(bar);
    VarInt(level).encode(&mut out).unwrap();
    VarInt(total).encode(&mut out).unwrap();
    ConversionResult::Converted(vec![build_payload(V18_S2C_EXPERIENCE, &out)])
}

fn s2c_update_sign(mut body: Bytes) -> ConversionResult {
    // 1.7: i32 x; i16 y; i32 z; 4 strings. 1.8: Position + 4 chat-JSON strings.
    if body.remaining() < 10 {
        return ConversionResult::Passthrough;
    }
    let x = body.get_i32() as i64;
    let y = body.get_i16() as i64;
    let z = body.get_i32() as i64;
    let packed = ((x & 0x3FF_FFFF) << 38) | ((z & 0x3FF_FFFF) << 12) | (y & 0xFFF);

    let mut out = BytesMut::new();
    out.put_i64(packed);
    for _ in 0..4 {
        let Ok(line) = String::decode(&mut body) else {
            return ConversionResult::Passthrough;
        };
        let json = format!("{{\"text\":{}}}", json_escape(&line));
        json.encode(&mut out).unwrap();
    }
    ConversionResult::Converted(vec![build_payload(V18_S2C_UPDATE_SIGN, &out)])
}

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn s2c_tab_complete(mut body: Bytes) -> ConversionResult {
    // 1.7: single string (newline-separated). 1.8: varint count; string[].
    let Ok(joined) = String::decode(&mut body) else {
        return ConversionResult::Passthrough;
    };
    let parts: Vec<&str> = if joined.is_empty() {
        Vec::new()
    } else {
        joined.split('\0').collect()
    };
    let mut out = BytesMut::new();
    VarInt(parts.len() as i32).encode(&mut out).unwrap();
    for p in parts {
        p.to_string().encode(&mut out).unwrap();
    }
    ConversionResult::Converted(vec![build_payload(V18_S2C_TAB_COMPLETE, &out)])
}

fn s2c_scoreboard_team(mut body: Bytes) -> ConversionResult {
    // 1.8 added a "name tag visibility" string after action 0/2.
    let Ok(team_name) = String::decode(&mut body) else {
        return ConversionResult::Passthrough;
    };
    if body.is_empty() {
        return ConversionResult::Passthrough;
    }
    let action = body[0];
    let mut out = BytesMut::new();
    team_name.encode(&mut out).unwrap();
    out.extend_from_slice(&body);
    if action == 0 || action == 2 {
        "always".to_owned().encode(&mut out).unwrap();
    }
    ConversionResult::Converted(vec![build_payload(V18_S2C_SCOREBOARD_TEAM, &out)])
}

// ──────────────────────────────────────────────────────────────────────────
// C2S: 1.7 client → 1.8 server
// ──────────────────────────────────────────────────────────────────────────

pub fn convert_c2s(payload: Bytes) -> ConversionResult {
    let Some((id, body)) = split_id(payload.clone()) else {
        return ConversionResult::Passthrough;
    };

    match id {
        V17_C2S_CHAT => ConversionResult::Passthrough,
        V17_C2S_USE_ENTITY => c2s_use_entity(body),
        V17_C2S_PLAYER_POS_LOOK => c2s_player_pos_look(body),
        V17_C2S_PLAYER_DIGGING => c2s_player_digging(body),
        V17_C2S_PLAYER_BLOCK_PLACE => c2s_player_block_place(body),
        V17_C2S_UPDATE_SIGN => c2s_update_sign(body),
        _ => ConversionResult::Passthrough,
    }
}

fn c2s_use_entity(mut body: Bytes) -> ConversionResult {
    // 1.7: i32 target; i8 mouse (0=right click, 1=left click). 1.8: varint target; varint type.
    if body.remaining() < 5 {
        return ConversionResult::Passthrough;
    }
    let target = body.get_i32();
    let mouse = body.get_i8();
    let typ = if mouse == 1 { 1 } else { 0 };

    let mut out = BytesMut::new();
    VarInt(target).encode(&mut out).unwrap();
    VarInt(typ).encode(&mut out).unwrap();
    ConversionResult::Converted(vec![build_payload(V18_C2S_USE_ENTITY, &out)])
}

fn c2s_player_pos_look(mut body: Bytes) -> ConversionResult {
    // 1.7 C2S 0x06: f64 x; f64 feet_y; f64 stance; f64 z; f32 yaw; f32 pitch; bool on_ground.
    // 1.8 C2S 0x06: f64 x; f64 feet_y; f64 z; f32 yaw; f32 pitch; bool on_ground.
    if body.remaining() < 41 {
        return ConversionResult::Passthrough;
    }
    let x = body.get_f64();
    let y = body.get_f64();
    let _stance = body.get_f64();
    let z = body.get_f64();
    let yaw = body.get_f32();
    let pitch = body.get_f32();
    let on_ground = body.get_u8();

    let mut out = BytesMut::with_capacity(33);
    out.put_f64(x);
    out.put_f64(y);
    out.put_f64(z);
    out.put_f32(yaw);
    out.put_f32(pitch);
    out.put_u8(on_ground);
    ConversionResult::Converted(vec![build_payload(V18_C2S_PLAYER_POS_LOOK, &out)])
}

fn c2s_player_digging(mut body: Bytes) -> ConversionResult {
    // 1.7: i8 status; i32 x; u8 y; i32 z; i8 face.
    // 1.8: i8 status; Position; i8 face.
    if body.remaining() < 11 {
        return ConversionResult::Passthrough;
    }
    let status = body.get_i8();
    let x = body.get_i32() as i64;
    let y = body.get_u8() as i64;
    let z = body.get_i32() as i64;
    let face = body.get_i8();

    let packed = ((x & 0x3FF_FFFF) << 38) | ((z & 0x3FF_FFFF) << 12) | (y & 0xFFF);
    let mut out = BytesMut::new();
    out.put_i8(status);
    out.put_i64(packed);
    out.put_i8(face);
    ConversionResult::Converted(vec![build_payload(V18_C2S_PLAYER_DIGGING, &out)])
}

fn c2s_player_block_place(mut body: Bytes) -> ConversionResult {
    // 1.7: i32 x; u8 y; i32 z; i8 dir; legacy_slot held; i8 cx; i8 cy; i8 cz.
    // 1.8: Position; i8 dir; legacy_slot held; i8 cx; i8 cy; i8 cz.
    if body.remaining() < 10 {
        return ConversionResult::Passthrough;
    }
    let x = body.get_i32() as i64;
    let y = body.get_u8() as i64;
    let z = body.get_i32() as i64;
    let dir = body.get_i8();
    let packed = ((x & 0x3FF_FFFF) << 38) | ((z & 0x3FF_FFFF) << 12) | (y & 0xFFF);

    let mut out = BytesMut::new();
    out.put_i64(packed);
    out.put_i8(dir);
    out.extend_from_slice(&body);
    ConversionResult::Converted(vec![build_payload(V18_C2S_PLAYER_BLOCK_PLACE, &out)])
}

fn c2s_update_sign(mut body: Bytes) -> ConversionResult {
    // 1.7: i32 x; i16 y; i32 z; 4 strings. 1.8: Position; 4 chat-JSON strings.
    if body.remaining() < 10 {
        return ConversionResult::Passthrough;
    }
    let x = body.get_i32() as i64;
    let y = body.get_i16() as i64;
    let z = body.get_i32() as i64;
    let packed = ((x & 0x3FF_FFFF) << 38) | ((z & 0x3FF_FFFF) << 12) | (y & 0xFFF);

    let mut out = BytesMut::new();
    out.put_i64(packed);
    for _ in 0..4 {
        let Ok(line) = String::decode(&mut body) else {
            return ConversionResult::Passthrough;
        };
        let json = format!("{{\"text\":{}}}", json_escape(&line));
        json.encode(&mut out).unwrap();
    }
    ConversionResult::Converted(vec![build_payload(V18_C2S_UPDATE_SIGN, &out)])
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
    fn position_roundtrip_via_spawn_position() {
        // 1.7 sends (10, 64, -3); 1.8 should receive packed Position with same coords.
        let mut body = BytesMut::new();
        body.put_i32(10);
        body.put_i32(64);
        body.put_i32(-3);
        let r = s2c_spawn_position(body.freeze());
        let pkt = decode_one(r);
        let (id, mut rest) = split_id(pkt).unwrap();
        assert_eq!(id, V18_S2C_SPAWN_POSITION);
        let packed = rest.get_i64();
        assert_eq!(packed, pack_pos(10, 64, -3));
    }

    #[test]
    fn chat_appends_position_byte() {
        let mut body = BytesMut::new();
        "hello".to_owned().encode(&mut body).unwrap();
        let r = s2c_chat(body.freeze());
        let pkt = decode_one(r);
        let (id, mut rest) = split_id(pkt).unwrap();
        assert_eq!(id, V18_S2C_CHAT);
        let s = String::decode(&mut rest).unwrap();
        assert_eq!(s, "hello");
        assert_eq!(rest.get_u8(), 0);
    }

    #[test]
    fn player_pos_look_drops_stance() {
        let mut body = BytesMut::new();
        body.put_f64(1.0); // x
        body.put_f64(72.62); // stance
        body.put_f64(71.0); // y
        body.put_f64(2.0); // z
        body.put_f32(90.0);
        body.put_f32(0.0);
        body.put_u8(1);
        let r = s2c_player_pos_look(body.freeze());
        let pkt = decode_one(r);
        let (id, mut rest) = split_id(pkt).unwrap();
        assert_eq!(id, V18_S2C_PLAYER_POS_LOOK);
        assert_eq!(rest.get_f64(), 1.0);
        assert_eq!(rest.get_f64(), 71.0);
        assert_eq!(rest.get_f64(), 2.0);
        assert_eq!(rest.get_f32(), 90.0);
        assert_eq!(rest.get_f32(), 0.0);
        assert_eq!(rest.get_u8(), 0);
    }

    #[test]
    fn entity_destroy_widens_count_and_ids() {
        let mut body = BytesMut::new();
        body.put_i8(2);
        body.put_i32(42);
        body.put_i32(7);
        let r = s2c_entity_destroy(body.freeze());
        let pkt = decode_one(r);
        let (id, mut rest) = split_id(pkt).unwrap();
        assert_eq!(id, V18_S2C_ENTITY_DESTROY);
        assert_eq!(VarInt::decode(&mut rest).unwrap().0, 2);
        assert_eq!(VarInt::decode(&mut rest).unwrap().0, 42);
        assert_eq!(VarInt::decode(&mut rest).unwrap().0, 7);
    }

    #[test]
    fn c2s_digging_uses_packed_position() {
        let mut body = BytesMut::new();
        body.put_i8(0); // status: started digging
        body.put_i32(100); // x
        body.put_u8(64); // y
        body.put_i32(-50); // z
        body.put_i8(1); // face
        let r = c2s_player_digging(body.freeze());
        let pkt = decode_one(r);
        let (id, mut rest) = split_id(pkt).unwrap();
        assert_eq!(id, V18_C2S_PLAYER_DIGGING);
        assert_eq!(rest.get_i8(), 0);
        assert_eq!(rest.get_i64(), pack_pos(100, 64, -50));
        assert_eq!(rest.get_i8(), 1);
    }
}
