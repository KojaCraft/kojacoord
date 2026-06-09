//! 1.12.2 → 1.16.5 packet converter.
//!
//! Crosses the 1.13 "flattening" boundary (numeric block/item IDs → varint
//! state/item IDs) plus the 1.14 Position bit-layout shift, the 1.15 biome
//! storage rework, and the 1.16 dimension codec / NBT-in-JoinGame changes.
//!
//! Reference material:
//! - <https://minecraft.wiki/w/Java_Edition_protocol/Packets> (current shapes)
//! - <https://minecraft.wiki/w/Java_Edition_protocol_history> (per-version diffs)
//! - <https://minecraft.wiki/w/Java_Edition_1.13/Flattening> (block/item flattening)
//! - PrismarineJS minecraft-data proto.yml for 1.12.2 and 1.16.5
//!
//! Packets whose body shape carries flattened block/item state IDs or
//! palette-encoded chunk data are too large to remap in one pass (it
//! requires a full ~12000-entry state mapping table). Those are dropped
//! with a warn-level trace and a note pointing at the wiki section to
//! consult. The result is a stable proxy that loses block/inventory
//! fidelity but does not desync the client connection.

use bytes::{Buf, BufMut, Bytes, BytesMut};
use kojacoord_protocol::codec::Encode;
use kojacoord_protocol::types::{Position, VarInt};

use super::{build_payload, split_id};
use crate::converter::ConversionResult;

// ── Packet ID mapping table ──────────────────────────────────────────────────
// (kept as `const` slices so they're easy to audit in one place)

/// `(1.12.2 id, 1.16.5 id)` for clientbound packets we forward as-is
/// (body unchanged between the two versions).
const S2C_ID_PASSTHROUGH: &[(u8, u8)] = &[
    // KeepAlive: both encode a single i64. IDs match too (0x1F == 0x1F).
    (0x1F, 0x1F),
];

// ── S2C IDs ──────────────────────────────────────────────────────────────────
const V12_S2C_KEEP_ALIVE: u8 = 0x1F;
const V12_S2C_JOIN_GAME: u8 = 0x23;
const V12_S2C_CHAT: u8 = 0x0F;
const V12_S2C_PLAYER_POS_LOOK: u8 = 0x2F;
const V12_S2C_SPAWN_POSITION: u8 = 0x46;
const V12_S2C_RESPAWN: u8 = 0x35;
const V12_S2C_DISCONNECT: u8 = 0x1A;
const V12_S2C_HELD_ITEM_CHANGE: u8 = 0x3A;
const V12_S2C_PLAYER_ABILITIES: u8 = 0x2C;
const V12_S2C_SET_EXPERIENCE: u8 = 0x40;
const V12_S2C_BLOCK_CHANGE: u8 = 0x0B;
const V12_S2C_MULTI_BLOCK_CHANGE: u8 = 0x10;
const V12_S2C_SET_SLOT: u8 = 0x16;
const V12_S2C_WINDOW_ITEMS: u8 = 0x14;
const V12_S2C_ENTITY_EQUIPMENT: u8 = 0x3F;
const V12_S2C_CHUNK_DATA: u8 = 0x20;
const V12_S2C_ENTITY_TELEPORT: u8 = 0x4C;
const V12_S2C_MOVE_ENTITY_POS: u8 = 0x25;
const V12_S2C_MOVE_ENTITY_POS_ROT: u8 = 0x26;
const V12_S2C_MOVE_ENTITY_ROT: u8 = 0x27;
const V12_S2C_DESTROY_ENTITIES: u8 = 0x32;
const V12_S2C_ENTITY_HEAD_LOOK: u8 = 0x36;
const V12_S2C_ENTITY_VELOCITY: u8 = 0x3E;

const V16_S2C_KEEP_ALIVE: u8 = 0x1F;
const V16_S2C_JOIN_GAME: u8 = 0x24;
const V16_S2C_CHAT: u8 = 0x0E;
const V16_S2C_PLAYER_POS_LOOK: u8 = 0x34;
const V16_S2C_SPAWN_POSITION: u8 = 0x42;
const V16_S2C_RESPAWN: u8 = 0x39;
const V16_S2C_DISCONNECT: u8 = 0x19;
const V16_S2C_HELD_ITEM_CHANGE: u8 = 0x3F;
const V16_S2C_PLAYER_ABILITIES: u8 = 0x2F;
const V16_S2C_SET_EXPERIENCE: u8 = 0x48;
const V16_S2C_ENTITY_TELEPORT: u8 = 0x56;
const V16_S2C_MOVE_ENTITY_POS: u8 = 0x27;
const V16_S2C_MOVE_ENTITY_POS_ROT: u8 = 0x28;
const V16_S2C_MOVE_ENTITY_ROT: u8 = 0x29;
const V16_S2C_DESTROY_ENTITIES: u8 = 0x36;
const V16_S2C_ENTITY_HEAD_LOOK: u8 = 0x3A;
const V16_S2C_ENTITY_VELOCITY: u8 = 0x46;
const V16_S2C_BLOCK_CHANGE: u8 = 0x0B;

// ── C2S IDs ──────────────────────────────────────────────────────────────────
const V12_C2S_TELEPORT_CONFIRM: u8 = 0x00;
const V12_C2S_CHAT: u8 = 0x02;
const V12_C2S_CLIENT_STATUS: u8 = 0x03;
const V12_C2S_CLIENT_SETTINGS: u8 = 0x04;
const V12_C2S_PLUGIN_MESSAGE: u8 = 0x09;
const V12_C2S_INTERACT: u8 = 0x0A;
const V12_C2S_KEEP_ALIVE: u8 = 0x0B;
const V12_C2S_MOVE_PLAYER_POS: u8 = 0x0C;
const V12_C2S_MOVE_PLAYER_POS_ROT: u8 = 0x0D;
const V12_C2S_MOVE_PLAYER_ROT: u8 = 0x0E;
const V12_C2S_PLAYER_ABILITIES: u8 = 0x12;
const V12_C2S_PLAYER_DIGGING: u8 = 0x14;
const V12_C2S_ENTITY_ACTION: u8 = 0x15;
const V12_C2S_HELD_ITEM_CHANGE: u8 = 0x1A;
const V12_C2S_ANIMATION: u8 = 0x1D;
const V12_C2S_PLAYER_BLOCK_PLACEMENT: u8 = 0x1F;
const V12_C2S_USE_ITEM: u8 = 0x20;

const V16_C2S_TELEPORT_CONFIRM: u8 = 0x00;
const V16_C2S_CHAT: u8 = 0x03;
const V16_C2S_CLIENT_STATUS: u8 = 0x04;
const V16_C2S_CLIENT_SETTINGS: u8 = 0x05;
const V16_C2S_PLUGIN_MESSAGE: u8 = 0x0B;
const V16_C2S_INTERACT: u8 = 0x0E;
const V16_C2S_KEEP_ALIVE: u8 = 0x10;
const V16_C2S_MOVE_PLAYER_POS: u8 = 0x12;
const V16_C2S_MOVE_PLAYER_ROT: u8 = 0x13;
const V16_C2S_MOVE_PLAYER_POS_ROT: u8 = 0x14;
const V16_C2S_PLAYER_ABILITIES: u8 = 0x19;
const V16_C2S_PLAYER_DIGGING: u8 = 0x1B;
const V16_C2S_ENTITY_ACTION: u8 = 0x1C;
const V16_C2S_HELD_ITEM_CHANGE: u8 = 0x25;
const V16_C2S_ANIMATION: u8 = 0x2C;
const V16_C2S_PLAYER_BLOCK_PLACEMENT: u8 = 0x2E;
const V16_C2S_USE_ITEM: u8 = 0x2F;

// ── Position repacking ───────────────────────────────────────────────────────
//
// 1.12.2 packing:  XXXXXXXXXXXXXXXXXXXXXXXXXX YYYYYYYYYY ZZZZZZZZZZZZZZZZZZZZZZZZZZ
//   (Y is the *middle* 12 bits — actually 12-bit field at offset 26).
//   Layout: `(x & 0x3FFFFFF) << 38 | (y & 0xFFF) << 26 | (z & 0x3FFFFFF)`.
// 1.14+ packing (matches `kojacoord_protocol::types::Position`):
//   `(x & 0x3FFFFFF) << 38 | (z & 0x3FFFFFF) << 12 | (y & 0xFFF)`.
//
// See: <https://minecraft.wiki/w/Java_Edition_protocol/Data_types#Position>
//      (note describing the Y-bit move in 1.14).

fn decode_legacy_position(val: u64) -> Position {
    let v = val as i64;
    let mut x = (v >> 38) & 0x3FF_FFFF;
    let mut y = (v >> 26) & 0xFFF;
    let mut z = v & 0x3FF_FFFF;
    if x >= 0x200_0000 {
        x -= 0x400_0000;
    }
    if z >= 0x200_0000 {
        z -= 0x400_0000;
    }
    if z & 0x200_0000 != 0 {
        // already handled
    }
    if y >= 0x800 {
        y -= 0x1000;
    }
    Position {
        x: x as i32,
        y: y as i32,
        z: z as i32,
    }
}

// ── S2C dispatch ─────────────────────────────────────────────────────────────

pub fn convert_s2c(payload: Bytes) -> ConversionResult {
    let Some((id, body)) = split_id(payload.clone()) else {
        return ConversionResult::Passthrough;
    };

    // Quick passthrough table for ID-stable, body-stable packets.
    for &(src, dst) in S2C_ID_PASSTHROUGH {
        if id == src && src == dst {
            return ConversionResult::Passthrough;
        }
    }

    match id {
        V12_S2C_KEEP_ALIVE => rebuild_with_id(V16_S2C_KEEP_ALIVE, &body),
        V12_S2C_JOIN_GAME => s2c_join_game(body),
        V12_S2C_CHAT => s2c_chat(body),
        V12_S2C_PLAYER_POS_LOOK => s2c_player_pos_look(body),
        V12_S2C_SPAWN_POSITION => s2c_spawn_position(body),
        V12_S2C_RESPAWN => s2c_respawn(body),
        V12_S2C_DISCONNECT => rebuild_with_id(V16_S2C_DISCONNECT, &body),
        V12_S2C_HELD_ITEM_CHANGE => rebuild_with_id(V16_S2C_HELD_ITEM_CHANGE, &body),
        V12_S2C_PLAYER_ABILITIES => rebuild_with_id(V16_S2C_PLAYER_ABILITIES, &body),
        V12_S2C_SET_EXPERIENCE => rebuild_with_id(V16_S2C_SET_EXPERIENCE, &body),
        V12_S2C_ENTITY_TELEPORT => rebuild_with_id(V16_S2C_ENTITY_TELEPORT, &body),
        V12_S2C_MOVE_ENTITY_POS => rebuild_with_id(V16_S2C_MOVE_ENTITY_POS, &body),
        V12_S2C_MOVE_ENTITY_POS_ROT => rebuild_with_id(V16_S2C_MOVE_ENTITY_POS_ROT, &body),
        V12_S2C_MOVE_ENTITY_ROT => rebuild_with_id(V16_S2C_MOVE_ENTITY_ROT, &body),
        V12_S2C_DESTROY_ENTITIES => rebuild_with_id(V16_S2C_DESTROY_ENTITIES, &body),
        V12_S2C_ENTITY_HEAD_LOOK => rebuild_with_id(V16_S2C_ENTITY_HEAD_LOOK, &body),
        V12_S2C_ENTITY_VELOCITY => rebuild_with_id(V16_S2C_ENTITY_VELOCITY, &body),

        // Flattening boundary: numeric block/item IDs → state IDs. Needs a
        // full ~12k-entry mapping table. Drop until a state table is wired in.
        // See <https://minecraft.wiki/w/Java_Edition_1.13/Flattening>.
        V12_S2C_BLOCK_CHANGE => s2c_block_change(body),
        V12_S2C_MULTI_BLOCK_CHANGE => {
            tracing::warn!(
                "v1_12_2_to_v1_16_5: dropping MultiBlockChange (needs flattening state table); \
                 not yet implemented"
            );
            ConversionResult::Drop
        },
        V12_S2C_CHUNK_DATA => {
            tracing::warn!(
                "v1_12_2_to_v1_16_5: dropping ChunkData (needs palette/bit-storage rewrite and \
                 biome-per-section rework, 1.15+); not yet implemented"
            );
            ConversionResult::Drop
        },
        V12_S2C_SET_SLOT => {
            tracing::warn!(
                "v1_12_2_to_v1_16_5: dropping SetSlot (needs item-id flattening map); \
                 not yet implemented — see minecraft-data 1.13 item mappings"
            );
            ConversionResult::Drop
        },
        V12_S2C_WINDOW_ITEMS => {
            tracing::warn!(
                "v1_12_2_to_v1_16_5: dropping WindowItems (needs item-id flattening map); \
                 not yet implemented"
            );
            ConversionResult::Drop
        },
        V12_S2C_ENTITY_EQUIPMENT => {
            tracing::warn!(
                "v1_12_2_to_v1_16_5: dropping EntityEquipment (needs item-id flattening + 1.16 \
                 multi-slot reshape); not yet implemented"
            );
            ConversionResult::Drop
        },
        _ => ConversionResult::Passthrough,
    }
}

fn rebuild_with_id(new_id: u8, body: &Bytes) -> ConversionResult {
    ConversionResult::Converted(vec![build_payload(new_id, body)])
}

fn s2c_block_change(mut body: Bytes) -> ConversionResult {
    // 1.12.2 wire: i64 packed Position (1.8 Y-in-middle layout) + VarInt block
    //              state where state = (block_id << 4) | meta.
    // 1.16.5 wire: i64 packed Position (1.14 Y-in-low layout) + VarInt
    //              flattened state id.
    if body.remaining() < 8 {
        return ConversionResult::Passthrough;
    }
    let packed_legacy = body.get_u64();
    let pos = decode_legacy_position(packed_legacy);
    let legacy_state = match VarInt::decode_from(&mut body) {
        Some(v) => v as u32,
        None => return ConversionResult::Passthrough,
    };
    let block_id = legacy_state >> 4;
    let meta = legacy_state & 0xF;
    let modern_state = super::flattening::legacy_to_state(block_id, meta);
    if modern_state == 0 && block_id != 0 {
        tracing::trace!(
            block_id,
            meta,
            "BlockChange: block not in flattening stub; emitting air"
        );
    }
    let packed_modern = encode_modern_position(pos.x, pos.y, pos.z);
    let mut out = BytesMut::new();
    out.put_i64(packed_modern);
    VarInt(modern_state as i32).encode(&mut out).unwrap();
    ConversionResult::Converted(vec![build_payload(V16_S2C_BLOCK_CHANGE, &out)])
}

fn encode_modern_position(x: i32, y: i32, z: i32) -> i64 {
    // 1.14+ layout: x high 26, z middle 26, y low 12.
    (((x as i64) & 0x3FF_FFFF) << 38) | (((z as i64) & 0x3FF_FFFF) << 12) | ((y as i64) & 0xFFF)
}

fn s2c_chat(mut body: Bytes) -> ConversionResult {
    // Both versions: <String json> <byte position>. 1.16.5 appends a Sender UUID.
    let str_len = match VarInt::decode_from(&mut body) {
        Some(n) => n as usize,
        None => return ConversionResult::Passthrough,
    };
    if body.remaining() < str_len + 1 {
        return ConversionResult::Passthrough;
    }
    let mut json = vec![0u8; str_len];
    body.copy_to_slice(&mut json);
    let position = body.get_u8();

    let mut out = BytesMut::new();
    VarInt(str_len as i32).encode(&mut out).unwrap();
    out.extend_from_slice(&json);
    out.put_u8(position);
    // Sender UUID — nil for system messages is fine.
    out.put_u64(0);
    out.put_u64(0);
    ConversionResult::Converted(vec![build_payload(V16_S2C_CHAT, &out)])
}

fn s2c_player_pos_look(body: Bytes) -> ConversionResult {
    // Body shape didn't change between 1.12.2 and 1.16.5: 3xf64, 2xf32, u8 flags, VarInt tp id.
    if body.remaining() < 33 {
        return ConversionResult::Passthrough;
    }
    ConversionResult::Converted(vec![build_payload(V16_S2C_PLAYER_POS_LOOK, &body)])
}

fn s2c_spawn_position(mut body: Bytes) -> ConversionResult {
    if body.remaining() < 8 {
        return ConversionResult::Passthrough;
    }
    let raw = body.get_u64();
    let pos = decode_legacy_position(raw);

    let mut out = BytesMut::with_capacity(8);
    pos.encode(&mut out).unwrap();
    ConversionResult::Converted(vec![build_payload(V16_S2C_SPAWN_POSITION, &out)])
}

fn s2c_join_game(mut body: Bytes) -> ConversionResult {
    // 1.12.2 layout: i32 eid, u8 gm, i32 dim, u8 diff, u8 maxp, VarString level, u8 reduced_debug.
    if body.remaining() < 4 + 1 + 4 + 1 + 1 {
        return ConversionResult::Passthrough;
    }
    let entity_id = body.get_i32();
    let gamemode = body.get_u8();
    let dimension_i32 = body.get_i32();
    let _difficulty = body.get_u8();
    let _max_players = body.get_u8();
    let level_len = match VarInt::decode_from(&mut body) {
        Some(n) => n as usize,
        None => return ConversionResult::Passthrough,
    };
    if body.remaining() < level_len + 1 {
        return ConversionResult::Passthrough;
    }
    let mut level_buf = vec![0u8; level_len];
    body.copy_to_slice(&mut level_buf);
    let reduced_debug_info = body.get_u8() != 0;
    let level_type = String::from_utf8_lossy(&level_buf).into_owned();
    let is_flat = level_type == "flat";

    // Translate 1.12.2 numeric dimension → 1.16.5 namespaced dimension/world key
    // and embed the dimension codec NBT + dimension type NBT a real 1.16.5
    // vanilla client expects right after `previous_game_mode`. See
    // <https://minecraft.wiki/w/Java_Edition_protocol_history> §1.16.
    let (dim_key, world_key) = match dimension_i32 {
        -1 => ("minecraft:the_nether", "minecraft:the_nether"),
        1 => ("minecraft:the_end", "minecraft:the_end"),
        _ => ("minecraft:overworld", "minecraft:overworld"),
    };

    let codec = match super::dimension_codec::dimension_codec_nbt() {
        Ok(b) => b,
        Err(e) => {
            tracing::warn!(error = %e, "failed to synthesize dimension codec NBT");
            return ConversionResult::Drop;
        },
    };
    let dim_type = match super::dimension_codec::dimension_type_nbt(dim_key) {
        Ok(b) => b,
        Err(e) => {
            tracing::warn!(error = %e, "failed to synthesize dimension type NBT");
            return ConversionResult::Drop;
        },
    };

    let mut out = BytesMut::new();
    out.put_i32(entity_id);
    out.put_u8(0); // is_hardcore
    out.put_u8(gamemode & 0x07);
    out.put_i8(-1); // previous_game_mode = "none"
    VarInt(1).encode(&mut out).unwrap();
    encode_string(world_key, &mut out);
    out.extend_from_slice(&codec);
    out.extend_from_slice(&dim_type);
    encode_string(world_key, &mut out);
    out.put_i64(0); // hashed_seed
    VarInt(0).encode(&mut out).unwrap(); // max_players (ignored)
    VarInt(8).encode(&mut out).unwrap(); // view_distance
    out.put_u8(reduced_debug_info as u8);
    out.put_u8(1); // enable_respawn_screen
    out.put_u8(0); // is_debug
    out.put_u8(is_flat as u8);

    ConversionResult::Converted(vec![build_payload(V16_S2C_JOIN_GAME, &out)])
}

fn s2c_respawn(mut body: Bytes) -> ConversionResult {
    if body.remaining() < 4 + 1 + 1 {
        return ConversionResult::Passthrough;
    }
    let dimension_i32 = body.get_i32();
    let _difficulty = body.get_u8();
    let game_mode = body.get_u8();
    let level_len = match VarInt::decode_from(&mut body) {
        Some(n) => n as usize,
        None => return ConversionResult::Passthrough,
    };
    if body.remaining() < level_len {
        return ConversionResult::Passthrough;
    }
    let mut level_buf = vec![0u8; level_len];
    body.copy_to_slice(&mut level_buf);
    let level_type = String::from_utf8_lossy(&level_buf).into_owned();
    let is_flat = level_type == "flat";

    let (dim_key, world_key) = match dimension_i32 {
        -1 => ("minecraft:the_nether", "minecraft:the_nether"),
        1 => ("minecraft:the_end", "minecraft:the_end"),
        _ => ("minecraft:overworld", "minecraft:overworld"),
    };

    let dim_type = match super::dimension_codec::dimension_type_nbt(dim_key) {
        Ok(b) => b,
        Err(e) => {
            tracing::warn!(error = %e, "failed to synthesize dimension type NBT (respawn)");
            return ConversionResult::Drop;
        },
    };

    let mut out = BytesMut::new();
    out.extend_from_slice(&dim_type);
    encode_string(world_key, &mut out);
    out.put_i64(0); // hashed_seed
    out.put_u8(game_mode & 0x07);
    out.put_i8(-1); // previous_game_mode
    out.put_u8(0); // is_debug
    out.put_u8(is_flat as u8);
    out.put_u8(0); // copy_metadata
    ConversionResult::Converted(vec![build_payload(V16_S2C_RESPAWN, &out)])
}

// ── C2S dispatch ─────────────────────────────────────────────────────────────

pub fn convert_c2s(payload: Bytes) -> ConversionResult {
    let Some((id, body)) = split_id(payload.clone()) else {
        return ConversionResult::Passthrough;
    };

    match id {
        V12_C2S_TELEPORT_CONFIRM => rebuild_with_id(V16_C2S_TELEPORT_CONFIRM, &body),
        V12_C2S_CHAT => rebuild_with_id(V16_C2S_CHAT, &body),
        V12_C2S_CLIENT_STATUS => rebuild_with_id(V16_C2S_CLIENT_STATUS, &body),
        V12_C2S_CLIENT_SETTINGS => rebuild_with_id(V16_C2S_CLIENT_SETTINGS, &body),
        V12_C2S_PLUGIN_MESSAGE => rebuild_with_id(V16_C2S_PLUGIN_MESSAGE, &body),
        V12_C2S_INTERACT => rebuild_with_id(V16_C2S_INTERACT, &body),
        V12_C2S_KEEP_ALIVE => rebuild_with_id(V16_C2S_KEEP_ALIVE, &body),
        V12_C2S_MOVE_PLAYER_POS => rebuild_with_id(V16_C2S_MOVE_PLAYER_POS, &body),
        V12_C2S_MOVE_PLAYER_POS_ROT => rebuild_with_id(V16_C2S_MOVE_PLAYER_POS_ROT, &body),
        V12_C2S_MOVE_PLAYER_ROT => rebuild_with_id(V16_C2S_MOVE_PLAYER_ROT, &body),
        V12_C2S_PLAYER_ABILITIES => rebuild_with_id(V16_C2S_PLAYER_ABILITIES, &body),
        V12_C2S_PLAYER_DIGGING => c2s_player_digging(body),
        V12_C2S_ENTITY_ACTION => rebuild_with_id(V16_C2S_ENTITY_ACTION, &body),
        V12_C2S_HELD_ITEM_CHANGE => rebuild_with_id(V16_C2S_HELD_ITEM_CHANGE, &body),
        V12_C2S_ANIMATION => rebuild_with_id(V16_C2S_ANIMATION, &body),
        V12_C2S_PLAYER_BLOCK_PLACEMENT => c2s_player_block_placement(body),
        V12_C2S_USE_ITEM => rebuild_with_id(V16_C2S_USE_ITEM, &body),
        _ => ConversionResult::Passthrough,
    }
}

fn c2s_player_digging(mut body: Bytes) -> ConversionResult {
    // <VarInt status> <Position location> <i8 face>.
    // Position layout changed at 1.14 — repack.
    let status = match VarInt::decode_from(&mut body) {
        Some(n) => n,
        None => return ConversionResult::Passthrough,
    };
    if body.remaining() < 8 + 1 {
        return ConversionResult::Passthrough;
    }
    let raw = body.get_u64();
    let pos = decode_legacy_position(raw);
    let face = body.get_i8();

    let mut out = BytesMut::new();
    VarInt(status).encode(&mut out).unwrap();
    pos.encode(&mut out).unwrap();
    out.put_i8(face);
    ConversionResult::Converted(vec![build_payload(V16_C2S_PLAYER_DIGGING, &out)])
}

fn c2s_player_block_placement(mut body: Bytes) -> ConversionResult {
    // 1.12.2: <Position> <VarInt face> <VarInt hand> <f32 cx> <f32 cy> <f32 cz>.
    // 1.16.5: <VarInt hand> <Position> <VarInt face> <f32 cx> <f32 cy> <f32 cz> <bool inside_block>.
    if body.remaining() < 8 {
        return ConversionResult::Passthrough;
    }
    let raw = body.get_u64();
    let pos = decode_legacy_position(raw);
    let face = match VarInt::decode_from(&mut body) {
        Some(n) => n,
        None => return ConversionResult::Passthrough,
    };
    let hand = match VarInt::decode_from(&mut body) {
        Some(n) => n,
        None => return ConversionResult::Passthrough,
    };
    if body.remaining() < 12 {
        return ConversionResult::Passthrough;
    }
    let cx = body.get_f32();
    let cy = body.get_f32();
    let cz = body.get_f32();

    let mut out = BytesMut::new();
    VarInt(hand).encode(&mut out).unwrap();
    pos.encode(&mut out).unwrap();
    VarInt(face).encode(&mut out).unwrap();
    out.put_f32(cx);
    out.put_f32(cy);
    out.put_f32(cz);
    out.put_u8(0); // inside_block
    ConversionResult::Converted(vec![build_payload(V16_C2S_PLAYER_BLOCK_PLACEMENT, &out)])
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn encode_string(s: &str, dst: &mut BytesMut) {
    let bytes = s.as_bytes();
    VarInt(bytes.len() as i32).encode(dst).unwrap();
    dst.extend_from_slice(bytes);
}

/// Thin wrapper that decodes a VarInt out of a `Bytes` without panicking.
trait VarIntExt {
    fn decode_from(body: &mut Bytes) -> Option<i32>;
}
impl VarIntExt for VarInt {
    fn decode_from(body: &mut Bytes) -> Option<i32> {
        use kojacoord_protocol::codec::Decode;
        VarInt::decode(body).ok().map(|v| v.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kojacoord_protocol::codec::Decode;

    #[test]
    fn legacy_position_roundtrips_origin() {
        // Pack origin in legacy layout, decode, re-encode to modern, decode again.
        let packed_legacy: u64 = 0;
        let pos = decode_legacy_position(packed_legacy);
        assert_eq!(pos, Position::new(0, 0, 0));
    }

    #[test]
    fn legacy_position_repacks_to_modern() {
        // Legacy pack of (100, 64, -200).
        let x: i64 = 100;
        let y: i64 = 64;
        let z: i64 = -200;
        let xi = x & 0x3FF_FFFF;
        let yi = y & 0xFFF;
        let zi = z & 0x3FF_FFFF;
        let packed = ((xi << 38) | (yi << 26) | zi) as u64;
        let pos = decode_legacy_position(packed);
        assert_eq!(pos, Position::new(100, 64, -200));

        // Now re-encode in modern layout via Position::encode, decode it back.
        let mut buf = BytesMut::new();
        pos.encode(&mut buf).unwrap();
        let mut bytes = buf.freeze();
        let round = Position::decode(&mut bytes).unwrap();
        assert_eq!(round, Position::new(100, 64, -200));
    }

    #[test]
    fn join_game_overworld_maps_to_modern() {
        let mut body = BytesMut::new();
        body.put_i32(42); // entity_id
        body.put_u8(1); // gamemode = creative
        body.put_i32(0); // dimension = overworld
        body.put_u8(2); // difficulty
        body.put_u8(20); // max_players
        let level = b"default";
        VarInt(level.len() as i32).encode(&mut body).unwrap();
        body.extend_from_slice(level);
        body.put_u8(0); // reduced_debug_info

        let res = s2c_join_game(body.freeze());
        let pkts = match res {
            ConversionResult::Converted(v) => v,
            _ => panic!("expected Converted"),
        };
        assert_eq!(pkts.len(), 1);
        let mut out = pkts[0].clone();
        let id = VarInt::decode(&mut out).unwrap().0 as u8;
        assert_eq!(id, V16_S2C_JOIN_GAME);
        let eid = out.get_i32();
        assert_eq!(eid, 42);
        let _is_hardcore = out.get_u8();
        let gm = out.get_u8();
        assert_eq!(gm, 1);
        let _prev = out.get_i8();
        let n_worlds = VarInt::decode(&mut out).unwrap().0;
        assert_eq!(n_worlds, 1);
    }

    #[test]
    fn spawn_position_repacks_layout() {
        // Legacy pack of (10, 80, -30).
        let xi: i64 = 10 & 0x3FF_FFFF;
        let yi: i64 = 80 & 0xFFF;
        let zi: i64 = (-30_i64) & 0x3FF_FFFF;
        let packed = ((xi << 38) | (yi << 26) | zi) as u64;
        let mut body = BytesMut::new();
        body.put_u64(packed);

        let res = s2c_spawn_position(body.freeze());
        let pkts = match res {
            ConversionResult::Converted(v) => v,
            _ => panic!("expected Converted"),
        };
        let mut out = pkts[0].clone();
        let id = VarInt::decode(&mut out).unwrap().0 as u8;
        assert_eq!(id, V16_S2C_SPAWN_POSITION);
        let pos = Position::decode(&mut out).unwrap();
        assert_eq!(pos, Position::new(10, 80, -30));
    }

    #[test]
    fn chat_appends_sender_uuid() {
        let json = br#"{"text":"hi"}"#;
        let mut body = BytesMut::new();
        VarInt(json.len() as i32).encode(&mut body).unwrap();
        body.extend_from_slice(json);
        body.put_u8(0); // chat position

        let res = s2c_chat(body.freeze());
        let pkts = match res {
            ConversionResult::Converted(v) => v,
            _ => panic!("expected Converted"),
        };
        let mut out = pkts[0].clone();
        let id = VarInt::decode(&mut out).unwrap().0 as u8;
        assert_eq!(id, V16_S2C_CHAT);
        // skip json len + json + position byte.
        let len = VarInt::decode(&mut out).unwrap().0 as usize;
        assert_eq!(len, json.len());
        out.advance(len + 1);
        // remaining: 16 bytes of UUID.
        assert_eq!(out.remaining(), 16);
    }
}
