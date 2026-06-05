use bytes::{Buf, BufMut, Bytes, BytesMut};
use kojacoord_protocol::codec::{Decode, Encode};
use kojacoord_protocol::types::VarInt;
use kojacoord_protocol::ProtocolVersion;

use crate::converter::ConversionResult;

use super::{build_payload, nearest, split_id};

pub fn convert_s2c(payload: Bytes, server_proto: u32) -> ConversionResult {
    let ver = nearest(server_proto);
    let Some((id, body)) = split_id(payload.clone()) else {
        return ConversionResult::Passthrough;
    };

    match ver {
        ProtocolVersion::V1_12_2 => dispatch_1_12(id, body),
        ProtocolVersion::V1_16_5 => dispatch_1_16(id, body),
        ProtocolVersion::V1_19_4 | ProtocolVersion::V1_20_4 | ProtocolVersion::V1_21 => {
            dispatch_modern(id, body, server_proto)
        },
        _ => dispatch_1_12(id, body),
    }
}

fn dispatch_1_12(id: u8, body: Bytes) -> ConversionResult {
    match id {
        0x23 => s2c_join_game(body),
        0x2F => s2c_player_pos_look(body),
        0x0F => s2c_chat(body),
        0x41 => s2c_set_health(body),
        0x33 => s2c_respawn(body),
        0x1F => s2c_keep_alive_long(body),
        0x44 => s2c_time_update(body),
        0x3E => s2c_tab_list(body),
        0x4A => s2c_entity_velocity(body),
        0x26 => ConversionResult::Drop,
        0x18 => s2c_entity_teleport(body),
        0x15 => s2c_entity_rel_move(body),
        0x0E => s2c_entity(body),
        0x13 => s2c_entity_destroy(body),
        0x19 => s2c_block_change(body),
        0x09 => s2c_held_item_change(body),
        0x3D => s2c_sound_effect(body),

        0x30 => s2c_window_items(body, ProtocolVersion::V1_12_2),
        0x1C => s2c_entity_metadata(body),
        0x03 => s2c_entity_equipment(body, ProtocolVersion::V1_12_2),
        0x1D => s2c_experience(body),
        0x3B => s2c_scoreboard_obj(body),
        0x3C => s2c_scoreboard_score(body),
        _ => ConversionResult::Passthrough,
    }
}

fn dispatch_1_16(id: u8, body: Bytes) -> ConversionResult {
    match id {
        0x24 => s2c_join_game(body),
        0x34 => s2c_player_pos_look(body),
        0x0E => s2c_chat(body),
        0x49 => s2c_set_health(body),
        0x3A => s2c_respawn(body),
        0x1F => s2c_keep_alive_long(body),
        0x4E => s2c_time_update(body),
        0x48 => s2c_tab_list(body),
        0x46 => s2c_entity_velocity(body),
        0x20 => ConversionResult::Drop,
        0x18 => s2c_entity_teleport(body),
        0x15 => s2c_entity_rel_move(body),

        0x13 => s2c_entity_destroy(body),
        0x19 => s2c_block_change(body),
        0x2F => s2c_set_slot(body, ProtocolVersion::V1_16_5),
        0x30 => s2c_window_items(body, ProtocolVersion::V1_16_5),
        0x1C => s2c_entity_metadata(body),
        0x03 => s2c_entity_equipment(body, ProtocolVersion::V1_16_5),
        0x1D => s2c_experience(body),
        0x3B => s2c_scoreboard_obj(body),
        0x3C => s2c_scoreboard_score(body),
        0x47 => s2c_held_item_change(body),
        _ => ConversionResult::Passthrough,
    }
}

fn dispatch_modern(id: u8, body: Bytes, server_proto: u32) -> ConversionResult {
    let ver = nearest(server_proto);
    match id {
        0x28 => s2c_join_game(body),
        0x3E => s2c_player_pos_look(body),
        0x36 => s2c_system_chat(body),
        0x1A => s2c_set_health(body),
        0x43 => s2c_respawn(body),
        0x23 => s2c_keep_alive_long(body),
        0x5A => s2c_time_update(body),
        0x65 => s2c_tab_list(body),
        0x56 => s2c_entity_velocity(body),
        0x25 => ConversionResult::Drop,
        0x18 => s2c_entity_teleport(body),
        0x15 => s2c_entity_rel_move(body),
        0x0E => s2c_entity(body),
        0x13 => s2c_entity_destroy(body),
        0x19 => s2c_block_change(body),
        0x2F => s2c_set_slot(body, ver),
        0x30 => s2c_window_items(body, ver),
        0x1C => s2c_entity_metadata(body),
        0x03 => s2c_entity_equipment(body, ver),
        0x1D => s2c_experience(body),
        0x3B => s2c_scoreboard_obj(body),
        0x3C => s2c_scoreboard_score(body),
        0x48 => s2c_held_item_change(body),
        _ => ConversionResult::Passthrough,
    }
}

fn s2c_join_game(body: Bytes) -> ConversionResult {
    let mut r = super::safe::Reader::new(body);
    let Some(entity_id) = r.i32() else {
        return ConversionResult::Passthrough;
    };
    let Some(gm) = r.u8() else {
        return ConversionResult::Passthrough;
    };
    let gamemode = gm & 0x03;

    let dimension: i8 = 0;

    let difficulty: u8 = 2;
    let max_players: u8 = 100;
    let level_type = "default".to_owned();

    let mut out = BytesMut::new();
    out.put_i32(entity_id);
    out.put_u8(gamemode);
    out.put_i8(dimension);
    out.put_u8(difficulty);
    out.put_u8(max_players);
    level_type.encode(&mut out).unwrap();
    out.put_u8(0);
    ConversionResult::Converted(vec![build_payload(0x01, &out)])
}

fn s2c_player_pos_look(body: Bytes) -> ConversionResult {
    let mut r = super::safe::Reader::new(body);
    let Some(x) = r.f64() else {
        return ConversionResult::Passthrough;
    };
    let Some(y) = r.f64() else {
        return ConversionResult::Passthrough;
    };
    let Some(z) = r.f64() else {
        return ConversionResult::Passthrough;
    };
    let Some(yaw) = r.f32() else {
        return ConversionResult::Passthrough;
    };
    let Some(pitch) = r.f32() else {
        return ConversionResult::Passthrough;
    };
    let flags = r.u8().unwrap_or(0);

    let mut out = BytesMut::new();
    out.put_f64(x);
    out.put_f64(y);
    out.put_f64(z);
    out.put_f32(yaw);
    out.put_f32(pitch);
    out.put_u8(flags);
    ConversionResult::Converted(vec![build_payload(0x08, &out)])
}

fn s2c_chat(mut body: Bytes) -> ConversionResult {
    let Ok(json) = String::decode(&mut body) else {
        return ConversionResult::Passthrough;
    };
    let position = if body.remaining() >= 1 {
        body.get_u8()
    } else {
        0
    };
    let mut out = BytesMut::new();
    json.encode(&mut out).unwrap();
    out.put_u8(position);
    ConversionResult::Converted(vec![build_payload(0x02, &out)])
}

fn s2c_system_chat(mut body: Bytes) -> ConversionResult {
    let Ok(content) = String::decode(&mut body) else {
        return ConversionResult::Passthrough;
    };
    let _overlay = if body.remaining() >= 1 {
        body.get_u8()
    } else {
        0
    };
    let mut out = BytesMut::new();
    content.encode(&mut out).unwrap();
    out.put_u8(1);
    ConversionResult::Converted(vec![build_payload(0x02, &out)])
}

fn s2c_set_health(body: Bytes) -> ConversionResult {
    ConversionResult::Converted(vec![build_payload(0x06, &body)])
}

fn s2c_respawn(_body: Bytes) -> ConversionResult {
    let dimension: i8 = 0;
    let mut out = BytesMut::new();
    out.put_i8(dimension);
    out.put_u8(2);
    out.put_u8(0);
    "default".to_owned().encode(&mut out).unwrap();
    ConversionResult::Converted(vec![build_payload(0x07, &out)])
}

fn s2c_keep_alive_long(body: Bytes) -> ConversionResult {
    let mut r = super::safe::Reader::new(body);
    let Some(id) = r.i64() else {
        return ConversionResult::Passthrough;
    };
    let mut out = BytesMut::new();
    VarInt(id as i32).encode(&mut out).unwrap();
    ConversionResult::Converted(vec![build_payload(0x00, &out)])
}

fn s2c_held_item_change(mut body: Bytes) -> ConversionResult {
    if body.remaining() < 1 {
        return ConversionResult::Passthrough;
    }
    let slot = body.get_u8();
    let mut out = BytesMut::new();
    out.put_u8(slot);
    ConversionResult::Converted(vec![build_payload(0x09, &out)])
}

fn s2c_sound_effect(mut body: Bytes) -> ConversionResult {
    let Ok(sound_name) = String::decode(&mut body) else {
        return ConversionResult::Passthrough;
    };
    if body.remaining() < 4 + 4 + 4 + 4 + 1 {
        return ConversionResult::Passthrough;
    }
    let x = body.get_i32();
    let y = body.get_i32();
    let z = body.get_i32();
    let volume = body.get_f32();
    let pitch = body.get_u8();

    let mut out = BytesMut::new();
    sound_name.encode(&mut out).unwrap();
    out.put_i32(x);
    out.put_i32(y);
    out.put_i32(z);
    out.put_f32(volume);
    out.put_u8(pitch);
    ConversionResult::Converted(vec![build_payload(0x29, &out)])
}

fn s2c_time_update(body: Bytes) -> ConversionResult {
    ConversionResult::Converted(vec![build_payload(0x03, &body)])
}

fn s2c_tab_list(body: Bytes) -> ConversionResult {
    ConversionResult::Converted(vec![build_payload(0x48, &body)])
}

fn s2c_entity_velocity(body: Bytes) -> ConversionResult {
    ConversionResult::Converted(vec![build_payload(0x12, &body)])
}

fn s2c_entity_teleport(body: Bytes) -> ConversionResult {
    let mut r = super::safe::Reader::new(body);
    let Some(entity_id) = r.varint() else {
        return ConversionResult::Passthrough;
    };
    let Some(x) = r.f64() else {
        return ConversionResult::Passthrough;
    };
    let x = x as i32;
    let Some(y) = r.f64() else {
        return ConversionResult::Passthrough;
    };
    let y = y as i32;
    let Some(z) = r.f64() else {
        return ConversionResult::Passthrough;
    };
    let z = z as i32;
    let Some(yaw) = r.u8() else {
        return ConversionResult::Passthrough;
    };
    let Some(pitch) = r.u8() else {
        return ConversionResult::Passthrough;
    };

    let mut out = BytesMut::new();
    out.put_i32(entity_id);
    out.put_i32(x);
    out.put_i32(y);
    out.put_i32(z);
    out.put_u8(yaw);
    out.put_u8(pitch);
    ConversionResult::Converted(vec![build_payload(0x18, &out)])
}

fn s2c_entity_rel_move(body: Bytes) -> ConversionResult {
    let mut r = super::safe::Reader::new(body);
    let Some(entity_id) = r.varint() else {
        return ConversionResult::Passthrough;
    };
    let Some(dx) = r.i16() else {
        return ConversionResult::Passthrough;
    };
    let dx = dx as i8;
    let Some(dy) = r.i16() else {
        return ConversionResult::Passthrough;
    };
    let dy = dy as i8;
    let Some(dz) = r.i16() else {
        return ConversionResult::Passthrough;
    };
    let dz = dz as i8;

    let mut out = BytesMut::new();
    out.put_i32(entity_id);
    out.put_i8(dx);
    out.put_i8(dy);
    out.put_i8(dz);
    ConversionResult::Converted(vec![build_payload(0x15, &out)])
}

fn s2c_entity(body: Bytes) -> ConversionResult {
    let mut r = super::safe::Reader::new(body);
    let Some(entity_id) = r.varint() else {
        return ConversionResult::Passthrough;
    };
    let Some(_uuid) = r.take(16) else {
        return ConversionResult::Passthrough;
    };
    let Some(entity_type) = r.varint() else {
        return ConversionResult::Passthrough;
    };
    let entity_type = entity_type as u8;
    let Some(x) = r.f64() else {
        return ConversionResult::Passthrough;
    };
    let x = x as i32;
    let Some(y) = r.f64() else {
        return ConversionResult::Passthrough;
    };
    let y = y as i32;
    let Some(z) = r.f64() else {
        return ConversionResult::Passthrough;
    };
    let z = z as i32;
    let Some(yaw) = r.u8() else {
        return ConversionResult::Passthrough;
    };
    let Some(pitch) = r.u8() else {
        return ConversionResult::Passthrough;
    };
    let Some(head_yaw) = r.u8() else {
        return ConversionResult::Passthrough;
    };

    let mut out = BytesMut::new();
    out.put_i32(entity_id);
    out.put_u8(entity_type);
    out.put_i32(x);
    out.put_i32(y);
    out.put_i32(z);
    out.put_u8(yaw);
    out.put_u8(pitch);
    out.put_u8(head_yaw);
    ConversionResult::Converted(vec![build_payload(0x0E, &out)])
}

fn s2c_entity_destroy(mut body: Bytes) -> ConversionResult {
    let mut out = BytesMut::new();
    let mut count = 0u8;

    while body.remaining() > 0 {
        if let Ok(entity_id) = VarInt::decode(&mut body) {
            VarInt(entity_id.0).encode(&mut out).unwrap();
            count = count.saturating_add(1);
        } else {
            break;
        }
    }

    let mut final_out = BytesMut::new();
    final_out.put_u8(count);
    final_out.extend_from_slice(&out);
    ConversionResult::Converted(vec![build_payload(0x13, &final_out)])
}

fn s2c_block_change(body: Bytes) -> ConversionResult {
    let mut r = super::safe::Reader::new(body);
    let Some(pos) = r.i64() else {
        return ConversionResult::Passthrough;
    };
    let Some(block_state) = r.varint() else {
        return ConversionResult::Passthrough;
    };

    let x = ((pos >> 38) & 0x3FFFFFF) as i32;
    let y = ((pos >> 26) & 0xFFF) as i32;
    let z = ((pos >> 12) & 0x3FFFFFF) as i32;

    let block_id = block_state >> 4;
    let metadata = (block_state & 0xF) as u8;

    let mut out = BytesMut::new();
    out.put_i32(x);
    out.put_u8(y as u8);
    out.put_i32(z);
    VarInt(block_id).encode(&mut out).unwrap();
    out.put_u8(metadata);
    ConversionResult::Converted(vec![build_payload(0x19, &out)])
}

fn s2c_set_slot(body: Bytes, ver: ProtocolVersion) -> ConversionResult {
    if super::items::is_legacy_slot(ver) {
        return ConversionResult::Converted(vec![build_payload(0x2F, &body)]);
    }
    let mut r = super::safe::Reader::new(body);
    let Some(window) = r.i8() else {
        return ConversionResult::Passthrough;
    };
    if super::items::has_state_id(ver) {
        if r.varint().is_none() {
            return ConversionResult::Passthrough;
        }
    }
    let Some(slot_idx) = r.i16() else {
        return ConversionResult::Passthrough;
    };

    let mut out = BytesMut::new();
    out.put_i8(window);
    out.put_i16(slot_idx);
    if super::items::modern_slot_parsable(ver) {
        let Some(slot) = r.slot() else {
            return ConversionResult::Passthrough;
        };
        if super::items::modern_slot_to_legacy(&slot)
            .encode(&mut out)
            .is_err()
        {
            return ConversionResult::Passthrough;
        }
    } else {
        if kojacoord_protocol::types::slot::LegacySlot(None)
            .encode(&mut out)
            .is_err()
        {
            return ConversionResult::Passthrough;
        }
    }
    ConversionResult::Converted(vec![build_payload(0x2F, &out)])
}

fn s2c_window_items(body: Bytes, ver: ProtocolVersion) -> ConversionResult {
    if super::items::is_legacy_slot(ver) {
        return ConversionResult::Converted(vec![build_payload(0x30, &body)]);
    }
    let mut r = super::safe::Reader::new(body);
    let Some(window) = r.u8() else {
        return ConversionResult::Passthrough;
    };
    let count: i32 = if super::items::has_state_id(ver) {
        if r.varint().is_none() {
            return ConversionResult::Passthrough;
        }
        match r.varint() {
            Some(c) => c,
            None => return ConversionResult::Passthrough,
        }
    } else {
        match r.i16() {
            Some(c) => c as i32,
            None => return ConversionResult::Passthrough,
        }
    };
    if count < 0 || count > 4096 {
        return ConversionResult::Passthrough;
    }

    let mut out = BytesMut::new();
    out.put_u8(window);
    out.put_i16(count as i16);
    if super::items::modern_slot_parsable(ver) {
        for _ in 0..count {
            let Some(slot) = r.slot() else {
                return ConversionResult::Passthrough;
            };
            if super::items::modern_slot_to_legacy(&slot)
                .encode(&mut out)
                .is_err()
            {
                return ConversionResult::Passthrough;
            }
        }
    } else {
        for _ in 0..count {
            if kojacoord_protocol::types::slot::LegacySlot(None)
                .encode(&mut out)
                .is_err()
            {
                return ConversionResult::Passthrough;
            }
        }
    }
    ConversionResult::Converted(vec![build_payload(0x30, &out)])
}

fn s2c_entity_metadata(body: Bytes) -> ConversionResult {
    ConversionResult::Converted(vec![build_payload(0x1C, &body)])
}

fn s2c_entity_equipment(body: Bytes, ver: ProtocolVersion) -> ConversionResult {
    if super::items::is_legacy_slot(ver) {
        return ConversionResult::Converted(vec![build_payload(0x03, &body)]);
    }
    let mut r = super::safe::Reader::new(body);
    let Some(entity_id) = r.varint() else {
        return ConversionResult::Passthrough;
    };

    if !super::items::modern_slot_parsable(ver) {
        let mut out = BytesMut::new();
        out.put_i32(entity_id);
        out.put_i16(0);
        let _ = kojacoord_protocol::types::slot::LegacySlot(None).encode(&mut out);
        return ConversionResult::Converted(vec![build_payload(0x03, &out)]);
    }

    let mut packets = Vec::new();
    loop {
        let Some(raw_slot) = r.i8() else {
            break;
        };
        let has_more = (raw_slot as u8) & 0x80 != 0;
        let idx = (raw_slot as u8) & 0x7F;
        let Some(slot) = r.slot() else {
            break;
        };
        if let Some(v18_slot) = super::items::map_equipment_slot(idx) {
            let mut out = BytesMut::new();
            out.put_i32(entity_id);
            out.put_i16(v18_slot);
            if super::items::modern_slot_to_legacy(&slot)
                .encode(&mut out)
                .is_ok()
            {
                packets.push(build_payload(0x03, &out));
            }
        }
        if !has_more {
            break;
        }
    }
    if packets.is_empty() {
        ConversionResult::Drop
    } else {
        ConversionResult::Converted(packets)
    }
}

fn s2c_experience(body: Bytes) -> ConversionResult {
    ConversionResult::Converted(vec![build_payload(0x1D, &body)])
}

fn s2c_scoreboard_obj(body: Bytes) -> ConversionResult {
    ConversionResult::Converted(vec![build_payload(0x3B, &body)])
}

fn s2c_scoreboard_score(body: Bytes) -> ConversionResult {
    ConversionResult::Converted(vec![build_payload(0x3C, &body)])
}
