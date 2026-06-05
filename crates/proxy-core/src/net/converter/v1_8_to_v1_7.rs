use bytes::{Buf, BufMut, Bytes, BytesMut};
use kojacoord_protocol::codec::{Decode, Encode};
use kojacoord_protocol::types::VarInt;

use super::{build_payload, split_id};
use crate::converter::ConversionResult;

const V18_S2C_LOGIN: u8 = 0x01;
const V18_S2C_PLAYER_POS_LOOK: u8 = 0x08;
const V18_S2C_CHAT: u8 = 0x02;
const V18_S2C_SCOREBOARD_TEAM: u8 = 0x3E;
const V18_S2C_ENTITY_TELEPORT: u8 = 0x18;
const V18_S2C_ENTITY_REL_MOVE: u8 = 0x15;
const V18_S2C_ENTITY: u8 = 0x0E;
const V18_S2C_ENTITY_DESTROY: u8 = 0x13;
const V18_S2C_BLOCK_CHANGE: u8 = 0x19;
const V18_S2C_SET_SLOT: u8 = 0x2F;
const V18_S2C_WINDOW_ITEMS: u8 = 0x30;
const V18_S2C_ENTITY_METADATA: u8 = 0x1C;
const V18_S2C_ENTITY_EQUIPMENT: u8 = 0x03;
const V18_S2C_EXPERIENCE: u8 = 0x1D;
const V18_S2C_SCOREBOARD_OBJ: u8 = 0x3B;
const V18_S2C_SCOREBOARD_SCORE: u8 = 0x3C;

const V17_S2C_LOGIN: u8 = 0x01;
const V17_S2C_PLAYER_POS_LOOK: u8 = 0x08;
const V17_S2C_CHAT: u8 = 0x02;
const V17_S2C_SCOREBOARD_TEAM: u8 = 0x3E;
const V17_S2C_ENTITY_TELEPORT: u8 = 0x18;
const V17_S2C_ENTITY_REL_MOVE: u8 = 0x15;
const V17_S2C_ENTITY: u8 = 0x0E;
const V17_S2C_ENTITY_DESTROY: u8 = 0x13;
const V17_S2C_BLOCK_CHANGE: u8 = 0x19;
const V17_S2C_SET_SLOT: u8 = 0x2F;
const V17_S2C_WINDOW_ITEMS: u8 = 0x30;
const V17_S2C_ENTITY_METADATA: u8 = 0x1C;
const V17_S2C_ENTITY_EQUIPMENT: u8 = 0x03;
const V17_S2C_EXPERIENCE: u8 = 0x1D;
const V17_S2C_SCOREBOARD_OBJ: u8 = 0x3B;
const V17_S2C_SCOREBOARD_SCORE: u8 = 0x3C;

const V18_C2S_PLAYER_POS_LOOK: u8 = 0x06;
const V17_C2S_PLAYER_POS_LOOK: u8 = 0x06;

const STANCE_OFFSET: f64 = 1.62;

pub fn convert_s2c(payload: Bytes) -> ConversionResult {
    let Some((id, body)) = split_id(payload.clone()) else {
        return ConversionResult::Passthrough;
    };

    match id {
        V18_S2C_LOGIN => s2c_login(body),
        V18_S2C_PLAYER_POS_LOOK => s2c_player_pos_look(body),
        V18_S2C_CHAT => s2c_chat(body),
        V18_S2C_SCOREBOARD_TEAM => s2c_scoreboard_team(body),
        V18_S2C_ENTITY_TELEPORT => s2c_entity_teleport(body),
        V18_S2C_ENTITY_REL_MOVE => s2c_entity_rel_move(body),
        V18_S2C_ENTITY => s2c_entity(body),
        V18_S2C_ENTITY_DESTROY => s2c_entity_destroy(body),
        V18_S2C_BLOCK_CHANGE => s2c_block_change(body),
        V18_S2C_SET_SLOT => s2c_set_slot(body),
        V18_S2C_WINDOW_ITEMS => s2c_window_items(body),
        V18_S2C_ENTITY_METADATA => s2c_entity_metadata(body),
        V18_S2C_ENTITY_EQUIPMENT => s2c_entity_equipment(body),
        V18_S2C_EXPERIENCE => s2c_experience(body),
        V18_S2C_SCOREBOARD_OBJ => s2c_scoreboard_obj(body),
        V18_S2C_SCOREBOARD_SCORE => s2c_scoreboard_score(body),
        _ => ConversionResult::Passthrough,
    }
}

fn s2c_login(body: Bytes) -> ConversionResult {
    if body.len() < 1 {
        return ConversionResult::Passthrough;
    }
    let trimmed = body.slice(..body.len().saturating_sub(1));
    ConversionResult::Converted(vec![build_payload(V17_S2C_LOGIN, &trimmed)])
}

fn s2c_player_pos_look(mut body: Bytes) -> ConversionResult {
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

fn s2c_chat(mut body: Bytes) -> ConversionResult {
    let Ok(json) = String::decode(&mut body) else {
        return ConversionResult::Passthrough;
    };
    let mut out = BytesMut::new();
    json.encode(&mut out).unwrap();
    ConversionResult::Converted(vec![build_payload(V17_S2C_CHAT, &out)])
}

fn s2c_scoreboard_team(mut body: Bytes) -> ConversionResult {
    let Ok(team_name) = String::decode(&mut body) else {
        return ConversionResult::Passthrough;
    };
    if body.is_empty() {
        return ConversionResult::Passthrough;
    }

    let rest = body.clone();

    let mut out = BytesMut::new();
    team_name.encode(&mut out).unwrap();
    out.extend_from_slice(&rest);
    ConversionResult::Converted(vec![build_payload(V17_S2C_SCOREBOARD_TEAM, &out)])
}

fn s2c_entity_teleport(body: Bytes) -> ConversionResult {
    ConversionResult::Converted(vec![build_payload(V17_S2C_ENTITY_TELEPORT, &body)])
}

fn s2c_entity_rel_move(body: Bytes) -> ConversionResult {
    ConversionResult::Converted(vec![build_payload(V17_S2C_ENTITY_REL_MOVE, &body)])
}

fn s2c_entity(mut body: Bytes) -> ConversionResult {
    if body.remaining() < 18 {
        return ConversionResult::Passthrough;
    }
    let entity_id = body.get_i32();
    let entity_type = body.get_u8();
    let x = body.get_i32();
    let y = body.get_i32();
    let z = body.get_i32();
    let yaw = body.get_u8();
    let pitch = body.get_u8();
    let _head_pitch = body.get_u8();

    let mut out = BytesMut::new();
    out.put_i32(entity_id);
    out.put_u8(entity_type);
    out.put_i32(x);
    out.put_i32(y);
    out.put_i32(z);
    out.put_u8(yaw);
    out.put_u8(pitch);
    out.put_u8(0x7F);
    ConversionResult::Converted(vec![build_payload(V17_S2C_ENTITY, &out)])
}

fn s2c_entity_destroy(mut body: Bytes) -> ConversionResult {
    if body.remaining() < 1 {
        return ConversionResult::Passthrough;
    }
    let count = body.get_u8();
    let mut out = BytesMut::new();
    out.put_u8(count);

    for _ in 0..count {
        if body.remaining() < 4 {
            return ConversionResult::Passthrough;
        }
        let entity_id = body.get_i32();
        out.put_i32(entity_id);
    }
    ConversionResult::Converted(vec![build_payload(V17_S2C_ENTITY_DESTROY, &out)])
}

fn s2c_block_change(mut body: Bytes) -> ConversionResult {
    if body.remaining() < 10 {
        return ConversionResult::Passthrough;
    }
    let x = body.get_i32();
    let y = body.get_u8();
    let z = body.get_i32();
    let block_id = match VarInt::decode(&mut body) {
        Ok(v) => v.0,
        Err(_) => return ConversionResult::Passthrough,
    };

    let mut out = BytesMut::new();
    out.put_i32(x);
    out.put_u8(y);
    out.put_i32(z);
    VarInt(block_id).encode(&mut out).unwrap();
    out.put_u8(0);
    ConversionResult::Converted(vec![build_payload(V17_S2C_BLOCK_CHANGE, &out)])
}

fn s2c_set_slot(body: Bytes) -> ConversionResult {
    ConversionResult::Converted(vec![build_payload(V17_S2C_SET_SLOT, &body)])
}

fn s2c_window_items(body: Bytes) -> ConversionResult {
    ConversionResult::Converted(vec![build_payload(V17_S2C_WINDOW_ITEMS, &body)])
}

fn s2c_entity_metadata(body: Bytes) -> ConversionResult {
    ConversionResult::Converted(vec![build_payload(V17_S2C_ENTITY_METADATA, &body)])
}

fn s2c_entity_equipment(body: Bytes) -> ConversionResult {
    ConversionResult::Converted(vec![build_payload(V17_S2C_ENTITY_EQUIPMENT, &body)])
}

fn s2c_experience(mut body: Bytes) -> ConversionResult {
    if body.remaining() < 12 {
        return ConversionResult::Passthrough;
    }
    let experience_bar = body.get_f32();
    let level = match VarInt::decode(&mut body) {
        Ok(v) => v.0 as i16,
        Err(_) => return ConversionResult::Passthrough,
    };
    let total_experience = match VarInt::decode(&mut body) {
        Ok(v) => v.0 as i16,
        Err(_) => return ConversionResult::Passthrough,
    };

    let mut out = BytesMut::new();
    out.put_f32(experience_bar);
    out.put_i16(level);
    out.put_i16(total_experience);
    ConversionResult::Converted(vec![build_payload(V17_S2C_EXPERIENCE, &out)])
}

fn s2c_scoreboard_obj(body: Bytes) -> ConversionResult {
    ConversionResult::Converted(vec![build_payload(V17_S2C_SCOREBOARD_OBJ, &body)])
}

fn s2c_scoreboard_score(body: Bytes) -> ConversionResult {
    ConversionResult::Converted(vec![build_payload(V17_S2C_SCOREBOARD_SCORE, &body)])
}

pub fn convert_c2s(payload: Bytes) -> ConversionResult {
    let Some((id, body)) = split_id(payload.clone()) else {
        return ConversionResult::Passthrough;
    };

    match id {
        V18_C2S_PLAYER_POS_LOOK => c2s_player_pos_look(body),
        _ => ConversionResult::Passthrough,
    }
}

fn c2s_player_pos_look(mut body: Bytes) -> ConversionResult {
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

    let mut out = BytesMut::with_capacity(34);
    out.put_f64(x);
    out.put_f64(y);
    out.put_f64(z);
    out.put_f32(yaw);
    out.put_f32(pitch);
    out.put_u8(on_ground);
    ConversionResult::Converted(vec![build_payload(V17_C2S_PLAYER_POS_LOOK, &out)])
}
