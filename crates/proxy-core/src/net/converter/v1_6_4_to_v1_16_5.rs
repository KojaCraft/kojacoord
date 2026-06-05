use bytes::{Buf, BufMut, Bytes, BytesMut};
use kojacoord_protocol::codec::Encode;
use kojacoord_protocol::types::VarInt;

use super::{build_payload, split_id};
use crate::converter::ConversionResult;

const V164_S2C_KEEP_ALIVE: u8 = 0x00;
const V164_S2C_CHAT: u8 = 0x03;
const V164_S2C_PLAYER_POS_LOOK: u8 = 0x13;
const V164_S2C_SPAWN_PLAYER: u8 = 0x14;
const V164_S2C_ENTITY_TELEPORT: u8 = 0x18;
const V164_S2C_ENTITY_REL_MOVE: u8 = 0x15;
const V164_S2C_ENTITY: u8 = 0x1E;
const V164_S2C_BLOCK_CHANGE: u8 = 0x35;
const V164_S2C_SET_SLOT: u8 = 0x67;
const V164_S2C_WINDOW_ITEMS: u8 = 0x68;
const V164_S2C_ENTITY_EQUIPMENT: u8 = 0x1C;
const V164_S2C_EXPERIENCE: u8 = 0x2B;
const V164_S2C_HELD_ITEM_CHANGE: u8 = 0x09;
const V164_S2C_PLAYER_ABILITIES: u8 = 0x43;
const V164_S2C_DISCONNECT: u8 = 0xFF;

const V165_S2C_KEEP_ALIVE: u8 = 0x1F;
const V165_S2C_CHAT: u8 = 0x0E;
const V165_S2C_PLAYER_POS_LOOK: u8 = 0x32;
const V165_S2C_SPAWN_PLAYER: u8 = 0x05;
const V165_S2C_ENTITY_TELEPORT: u8 = 0x56;
const V165_S2C_ENTITY_REL_MOVE: u8 = 0x28;
const V165_S2C_ENTITY: u8 = 0x00;
const V165_S2C_BLOCK_CHANGE: u8 = 0x0B;
const V165_S2C_SET_SLOT: u8 = 0x16;
const V165_S2C_WINDOW_ITEMS: u8 = 0x14;
const V165_S2C_ENTITY_EQUIPMENT: u8 = 0x46;
const V165_S2C_EXPERIENCE: u8 = 0x3D;
const V165_S2C_HELD_ITEM_CHANGE: u8 = 0x48;
const V165_S2C_PLAYER_ABILITIES: u8 = 0x2B;
const V165_S2C_DISCONNECT: u8 = 0x1A;

pub fn convert_s2c(payload: Bytes) -> ConversionResult {
    let Some((id, body)) = split_id(payload.clone()) else {
        return ConversionResult::Passthrough;
    };

    match id {
        V164_S2C_KEEP_ALIVE => s2c_keep_alive(body),
        V164_S2C_CHAT => s2c_chat(body),
        V164_S2C_PLAYER_POS_LOOK => s2c_player_pos_look(body),
        V164_S2C_SPAWN_PLAYER => s2c_spawn_player(body),
        V164_S2C_ENTITY_TELEPORT => s2c_entity_teleport(body),
        V164_S2C_ENTITY_REL_MOVE => s2c_entity_rel_move(body),
        V164_S2C_ENTITY => s2c_entity(body),
        V164_S2C_BLOCK_CHANGE => s2c_block_change(body),
        V164_S2C_SET_SLOT => s2c_set_slot(body),
        V164_S2C_WINDOW_ITEMS => s2c_window_items(body),
        V164_S2C_ENTITY_EQUIPMENT => s2c_entity_equipment(body),
        V164_S2C_EXPERIENCE => s2c_experience(body),
        V164_S2C_HELD_ITEM_CHANGE => s2c_held_item_change(body),
        V164_S2C_PLAYER_ABILITIES => s2c_player_abilities(body),
        V164_S2C_DISCONNECT => s2c_disconnect(body),
        _ => ConversionResult::Passthrough,
    }
}

fn s2c_keep_alive(mut body: Bytes) -> ConversionResult {
    if body.remaining() < 4 {
        return ConversionResult::Passthrough;
    }
    let id = body.get_i32();
    let mut out = BytesMut::with_capacity(8);
    VarInt(id).encode(&mut out).unwrap();
    VarInt(0).encode(&mut out).unwrap();
    ConversionResult::Converted(vec![build_payload(V165_S2C_KEEP_ALIVE, &out)])
}

fn s2c_chat(mut body: Bytes) -> ConversionResult {
    if body.remaining() < 2 {
        return ConversionResult::Passthrough;
    }

    let str_len = body.get_u16() as usize;
    if body.remaining() < str_len * 2 {
        return ConversionResult::Passthrough;
    }

    let mut utf16_bytes = vec![0u8; str_len * 2];
    body.copy_to_slice(&mut utf16_bytes);

    let utf8_string = String::from_utf16_lossy(
        &utf16_bytes
            .chunks(2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .collect::<Vec<_>>(),
    );

    let mut out = BytesMut::new();
    VarInt(utf8_string.len() as i32).encode(&mut out).unwrap();
    out.extend_from_slice(utf8_string.as_bytes());
    out.put_u8(0);

    let nil_uuid = uuid::Uuid::nil();
    let (hi, lo) = nil_uuid.as_u64_pair();
    out.put_i64(hi as i64);
    out.put_i64(lo as i64);

    ConversionResult::Converted(vec![build_payload(V165_S2C_CHAT, &out)])
}

fn s2c_player_pos_look(mut body: Bytes) -> ConversionResult {
    if body.remaining() < 33 {
        return ConversionResult::Passthrough;
    }
    let x = body.get_i32() as f64;
    let y = body.get_i32() as f64;
    let _stance = body.get_i32() as f64;
    let z = body.get_i32() as f64;
    let yaw = body.get_f32();
    let pitch = body.get_f32();
    let on_ground = body.get_u8() != 0;

    let mut out = BytesMut::with_capacity(33);
    out.put_f64(x);
    out.put_f64(y);
    out.put_f64(z);
    out.put_f32(yaw);
    out.put_f32(pitch);
    out.put_u8(if on_ground { 1 } else { 0 });
    ConversionResult::Converted(vec![build_payload(V165_S2C_PLAYER_POS_LOOK, &out)])
}

fn s2c_spawn_player(mut body: Bytes) -> ConversionResult {
    if body.remaining() < 4 {
        return ConversionResult::Passthrough;
    }

    let entity_id = body.get_i32();

    if body.remaining() < 2 {
        return ConversionResult::Passthrough;
    }
    let username_len = body.get_u16() as usize;
    if body.remaining() < username_len * 2 {
        return ConversionResult::Passthrough;
    }
    let mut utf16_bytes = vec![0u8; username_len * 2];
    body.copy_to_slice(&mut utf16_bytes);
    let _username = String::from_utf16_lossy(
        &utf16_bytes
            .chunks(2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .collect::<Vec<_>>(),
    );

    if body.remaining() < 4 + 4 + 4 + 1 + 1 + 2 {
        return ConversionResult::Passthrough;
    }

    let x = body.get_i32() as f64;
    let y = body.get_i32() as f64;
    let z = body.get_i32() as f64;
    let yaw = body.get_i8();
    let pitch = body.get_i8();
    let current_item = body.get_i16();

    let player_uuid = uuid::Uuid::new_v4();

    let mut out = BytesMut::new();
    VarInt(entity_id).encode(&mut out).unwrap();

    let (hi, lo) = player_uuid.as_u64_pair();
    out.put_i64(hi as i64);
    out.put_i64(lo as i64);

    out.put_f64(x);
    out.put_f64(y);
    out.put_f64(z);
    out.put_u8(yaw as u8);
    out.put_u8(pitch as u8);
    VarInt(current_item as i32).encode(&mut out).unwrap();

    out.put_u8(0xFF);

    ConversionResult::Converted(vec![build_payload(V165_S2C_SPAWN_PLAYER, &out)])
}

fn s2c_entity_teleport(mut body: Bytes) -> ConversionResult {
    if body.remaining() < 18 {
        return ConversionResult::Passthrough;
    }
    let entity_id = body.get_i32();
    let x = body.get_i32() as f64;
    let y = body.get_i32() as f64;
    let z = body.get_i32() as f64;
    let yaw = body.get_i8();
    let pitch = body.get_i8();

    let mut out = BytesMut::with_capacity(18);
    VarInt(entity_id).encode(&mut out).unwrap();
    out.put_f64(x);
    out.put_f64(y);
    out.put_f64(z);
    out.put_u8(yaw as u8);
    out.put_u8(pitch as u8);
    out.put_u8(0);
    ConversionResult::Converted(vec![build_payload(V165_S2C_ENTITY_TELEPORT, &out)])
}

fn s2c_entity_rel_move(mut body: Bytes) -> ConversionResult {
    if body.remaining() < 7 {
        return ConversionResult::Passthrough;
    }
    let entity_id = body.get_i32();
    let dx = body.get_i8();
    let dy = body.get_i8();
    let dz = body.get_i8();

    let mut out = BytesMut::with_capacity(8);
    VarInt(entity_id).encode(&mut out).unwrap();
    out.put_i16(dx as i16);
    out.put_i16(dy as i16);
    out.put_i16(dz as i16);
    out.put_u8(0);
    ConversionResult::Converted(vec![build_payload(V165_S2C_ENTITY_REL_MOVE, &out)])
}

fn s2c_entity(mut body: Bytes) -> ConversionResult {
    if body.remaining() < 4 + 1 + 4 + 4 + 4 + 1 + 1 + 1 + 2 + 2 + 2 {
        return ConversionResult::Passthrough;
    }

    let entity_id = body.get_i32();
    let entity_type = body.get_i8() as i32;
    let x = body.get_i32() as f64;
    let y = body.get_i32() as f64;
    let z = body.get_i32() as f64;
    let yaw = body.get_i8();
    let pitch = body.get_i8();
    let head_pitch = body.get_i8();
    let velocity_x = body.get_i16();
    let velocity_y = body.get_i16();
    let velocity_z = body.get_i16();

    let mut out = BytesMut::new();
    VarInt(entity_id).encode(&mut out).unwrap();
    VarInt(entity_type).encode(&mut out).unwrap();
    out.put_f64(x);
    out.put_f64(y);
    out.put_f64(z);
    out.put_u8(yaw as u8);
    out.put_u8(pitch as u8);
    out.put_u8(head_pitch as u8);
    out.put_i16(velocity_x);
    out.put_i16(velocity_y);
    out.put_i16(velocity_z);

    out.put_u8(0xFF);

    ConversionResult::Converted(vec![build_payload(V165_S2C_ENTITY, &out)])
}

fn s2c_block_change(mut body: Bytes) -> ConversionResult {
    if body.remaining() < 11 {
        return ConversionResult::Passthrough;
    }
    let x = body.get_i32();
    let y = body.get_i8();
    let z = body.get_i32();
    let block_id = body.get_i32();
    let metadata = body.get_i8();

    let mut out = BytesMut::with_capacity(11);
    VarInt(x).encode(&mut out).unwrap();
    out.put_u8(y as u8);
    VarInt(z).encode(&mut out).unwrap();
    VarInt(block_id).encode(&mut out).unwrap();
    out.put_u8(metadata as u8);
    ConversionResult::Converted(vec![build_payload(V165_S2C_BLOCK_CHANGE, &out)])
}

fn s2c_set_slot(mut body: Bytes) -> ConversionResult {
    if body.remaining() < 1 + 2 {
        return ConversionResult::Passthrough;
    }

    let window_id = body.get_i8();
    let slot = body.get_i16();

    if body.remaining() < 2 + 2 + 1 {
        return ConversionResult::Passthrough;
    }

    let item_id = body.get_i16();
    let _damage = body.get_i16();
    let count = body.get_i8();

    let mut out = BytesMut::new();
    out.put_i8(window_id);
    out.put_i16(slot);

    if item_id == -1 {
        out.put_u8(0);
    } else {
        VarInt(item_id as i32).encode(&mut out).unwrap();
        out.put_i8(count);

        out.put_u8(0x00);
    }

    ConversionResult::Converted(vec![build_payload(V165_S2C_SET_SLOT, &out)])
}

fn s2c_window_items(mut body: Bytes) -> ConversionResult {
    if body.remaining() < 1 + 2 {
        return ConversionResult::Passthrough;
    }

    let window_id = body.get_i8();
    let count = body.get_i16();

    let mut out = BytesMut::new();
    out.put_i8(window_id);
    VarInt(0).encode(&mut out).unwrap();

    for _ in 0..count {
        if body.remaining() < 2 + 2 + 1 {
            return ConversionResult::Passthrough;
        }

        let item_id = body.get_i16();
        let _damage = body.get_i16();
        let slot_count = body.get_i8();

        if item_id == -1 {
            out.put_u8(0);
        } else {
            VarInt(item_id as i32).encode(&mut out).unwrap();
            out.put_i8(slot_count);

            out.put_u8(0x00);
        }
    }

    ConversionResult::Converted(vec![build_payload(V165_S2C_WINDOW_ITEMS, &out)])
}

fn s2c_entity_equipment(mut body: Bytes) -> ConversionResult {
    if body.remaining() < 6 {
        return ConversionResult::Passthrough;
    }
    let entity_id = body.get_i32();
    let slot = body.get_i16();
    let item = body.get_i16();

    let mut out = BytesMut::with_capacity(8);
    VarInt(entity_id).encode(&mut out).unwrap();
    VarInt(slot as i32).encode(&mut out).unwrap();
    VarInt(item as i32).encode(&mut out).unwrap();
    out.put_u8(0);
    ConversionResult::Converted(vec![build_payload(V165_S2C_ENTITY_EQUIPMENT, &out)])
}

fn s2c_experience(mut body: Bytes) -> ConversionResult {
    if body.remaining() < 4 {
        return ConversionResult::Passthrough;
    }
    let experience_bar = body.get_f32();
    let level = body.get_i16();
    let total_experience = body.get_i16();

    let mut out = BytesMut::with_capacity(6);
    out.put_f32(experience_bar);
    VarInt(level as i32).encode(&mut out).unwrap();
    VarInt(total_experience as i32).encode(&mut out).unwrap();
    ConversionResult::Converted(vec![build_payload(V165_S2C_EXPERIENCE, &out)])
}

fn s2c_held_item_change(mut body: Bytes) -> ConversionResult {
    if body.remaining() < 1 {
        return ConversionResult::Passthrough;
    }
    let slot = body.get_i8();

    let mut out = BytesMut::with_capacity(2);
    VarInt(slot as i32).encode(&mut out).unwrap();
    ConversionResult::Converted(vec![build_payload(V165_S2C_HELD_ITEM_CHANGE, &out)])
}

fn s2c_player_abilities(mut body: Bytes) -> ConversionResult {
    if body.remaining() < 12 {
        return ConversionResult::Passthrough;
    }
    let flags = body.get_u8();
    let flying_speed = body.get_f32();
    let walking_speed = body.get_f32();

    let mut out = BytesMut::with_capacity(9);
    out.put_u8(flags);
    out.put_f32(flying_speed);
    out.put_f32(walking_speed);
    ConversionResult::Converted(vec![build_payload(V165_S2C_PLAYER_ABILITIES, &out)])
}

fn s2c_disconnect(mut body: Bytes) -> ConversionResult {
    if body.remaining() < 2 {
        return ConversionResult::Passthrough;
    }

    let str_len = body.get_u16() as usize;
    if body.remaining() < str_len * 2 {
        return ConversionResult::Passthrough;
    }

    let mut utf16_bytes = vec![0u8; str_len * 2];
    body.copy_to_slice(&mut utf16_bytes);

    let plain_text = String::from_utf16_lossy(
        &utf16_bytes
            .chunks(2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .collect::<Vec<_>>(),
    );

    let json_message = format!(r#"{{"text":"{}"}}"#, plain_text.replace('"', r#"\""#));

    let mut out = BytesMut::new();
    VarInt(json_message.len() as i32).encode(&mut out).unwrap();
    out.extend_from_slice(json_message.as_bytes());

    ConversionResult::Converted(vec![build_payload(V165_S2C_DISCONNECT, &out)])
}

const V164_C2S_KEEP_ALIVE: u8 = 0x00;
const V164_C2S_CHAT: u8 = 0x01;
const V164_C2S_PLAYER_POS_LOOK: u8 = 0x05;
const V164_C2S_PLAYER_DIGGING: u8 = 0x0E;
const V164_C2S_PLAYER_BLOCK_PLACEMENT: u8 = 0x0F;
const V164_C2S_HELD_ITEM_CHANGE: u8 = 0x10;
const V164_C2S_ENTITY_ACTION: u8 = 0x13;

const V165_C2S_KEEP_ALIVE: u8 = 0x0E;
const V165_C2S_CHAT: u8 = 0x03;
const V165_C2S_PLAYER_POS_LOOK: u8 = 0x11;
const V165_C2S_PLAYER_DIGGING: u8 = 0x18;
const V165_C2S_PLAYER_BLOCK_PLACEMENT: u8 = 0x1E;
const V165_C2S_HELD_ITEM_CHANGE: u8 = 0x25;
const V165_C2S_ENTITY_ACTION: u8 = 0x1C;

pub fn convert_c2s(payload: Bytes) -> ConversionResult {
    let Some((id, body)) = split_id(payload.clone()) else {
        return ConversionResult::Passthrough;
    };

    match id {
        V164_C2S_KEEP_ALIVE => c2s_keep_alive(body),
        V164_C2S_CHAT => c2s_chat(body),
        V164_C2S_PLAYER_POS_LOOK => c2s_player_pos_look(body),
        V164_C2S_PLAYER_DIGGING => c2s_player_digging(body),
        V164_C2S_PLAYER_BLOCK_PLACEMENT => c2s_player_block_placement(body),
        V164_C2S_HELD_ITEM_CHANGE => c2s_held_item_change(body),
        V164_C2S_ENTITY_ACTION => c2s_entity_action(body),
        _ => ConversionResult::Passthrough,
    }
}

fn c2s_keep_alive(mut body: Bytes) -> ConversionResult {
    if body.remaining() < 4 {
        return ConversionResult::Passthrough;
    }
    let id = body.get_i32();
    let mut out = BytesMut::with_capacity(8);
    VarInt(id).encode(&mut out).unwrap();
    ConversionResult::Converted(vec![build_payload(V165_C2S_KEEP_ALIVE, &out)])
}

fn c2s_chat(mut body: Bytes) -> ConversionResult {
    if body.remaining() < 2 {
        return ConversionResult::Passthrough;
    }

    let str_len = body.get_u16() as usize;
    if body.remaining() < str_len * 2 {
        return ConversionResult::Passthrough;
    }

    let mut utf16_bytes = vec![0u8; str_len * 2];
    body.copy_to_slice(&mut utf16_bytes);

    let utf8_string = String::from_utf16_lossy(
        &utf16_bytes
            .chunks(2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .collect::<Vec<_>>(),
    );

    let mut out = BytesMut::new();
    VarInt(utf8_string.len() as i32).encode(&mut out).unwrap();
    out.extend_from_slice(utf8_string.as_bytes());

    ConversionResult::Converted(vec![build_payload(V165_C2S_CHAT, &out)])
}

fn c2s_player_pos_look(mut body: Bytes) -> ConversionResult {
    if body.remaining() < 33 {
        return ConversionResult::Passthrough;
    }
    let x = body.get_i32() as f64;
    let y = body.get_i32() as f64;
    let _stance = body.get_i32() as f64;
    let z = body.get_i32() as f64;
    let yaw = body.get_f32();
    let pitch = body.get_f32();
    let on_ground = body.get_u8() != 0;

    let mut out = BytesMut::with_capacity(33);
    out.put_f64(x);
    out.put_f64(y);
    out.put_f64(z);
    out.put_f32(yaw);
    out.put_f32(pitch);
    out.put_u8(if on_ground { 1 } else { 0 });
    ConversionResult::Converted(vec![build_payload(V165_C2S_PLAYER_POS_LOOK, &out)])
}

fn c2s_player_digging(mut body: Bytes) -> ConversionResult {
    if body.remaining() < 11 {
        return ConversionResult::Passthrough;
    }

    let status = body.get_i8();
    let x = body.get_i32();
    let y = body.get_i8();
    let z = body.get_i32();
    let face = body.get_i8();

    let mut out = BytesMut::new();
    VarInt(status as i32).encode(&mut out).unwrap();

    out.put_i64(x as i64);
    out.put_i64(y as i64);
    out.put_i64(z as i64);

    VarInt(face as i32).encode(&mut out).unwrap();

    ConversionResult::Converted(vec![build_payload(V165_C2S_PLAYER_DIGGING, &out)])
}

fn c2s_player_block_placement(mut body: Bytes) -> ConversionResult {
    if body.remaining() < 16 {
        return ConversionResult::Passthrough;
    }

    let x = body.get_i32();
    let y = body.get_i8();
    let z = body.get_i32();
    let direction = body.get_i8();
    let held_item = body.get_i16();
    let cursor_x = body.get_i8();
    let cursor_y = body.get_i8();
    let cursor_z = body.get_i8();

    let mut out = BytesMut::new();

    out.put_i64(x as i64);
    out.put_i64(y as i64);
    out.put_i64(z as i64);

    VarInt(direction as i32).encode(&mut out).unwrap();
    VarInt(0).encode(&mut out).unwrap();
    out.put_i16(held_item);
    out.put_i8(cursor_x);
    out.put_i8(cursor_y);
    out.put_i8(cursor_z);

    ConversionResult::Converted(vec![build_payload(V165_C2S_PLAYER_BLOCK_PLACEMENT, &out)])
}

fn c2s_held_item_change(mut body: Bytes) -> ConversionResult {
    if body.remaining() < 2 {
        return ConversionResult::Passthrough;
    }
    let slot = body.get_i16();

    let mut out = BytesMut::with_capacity(2);
    VarInt(slot as i32).encode(&mut out).unwrap();
    ConversionResult::Converted(vec![build_payload(V165_C2S_HELD_ITEM_CHANGE, &out)])
}

fn c2s_entity_action(mut body: Bytes) -> ConversionResult {
    if body.remaining() < 5 {
        return ConversionResult::Passthrough;
    }

    let entity_id = body.get_i32();
    let action_id = body.get_i8();

    let mut out = BytesMut::new();
    VarInt(entity_id).encode(&mut out).unwrap();
    VarInt(action_id as i32).encode(&mut out).unwrap();
    VarInt(0).encode(&mut out).unwrap();

    ConversionResult::Converted(vec![build_payload(V165_C2S_ENTITY_ACTION, &out)])
}
