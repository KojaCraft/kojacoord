use bytes::{Buf, BufMut, Bytes, BytesMut};
use kojacoord_protocol::codec::{Decode, Encode};
use kojacoord_protocol::types::VarInt;
use kojacoord_protocol::ProtocolVersion;

use super::{build_payload, nearest, split_id};
use crate::converter::ConversionResult;

const V18_C2S_CHAT: u8 = 0x01;
const V18_C2S_USE_ENTITY: u8 = 0x02;
const V18_C2S_PLAYER_POS_LOOK: u8 = 0x06;
const V18_C2S_DIGGING: u8 = 0x07;
const V18_C2S_BLOCK_PLACE: u8 = 0x08;
const V18_C2S_HELD_ITEM: u8 = 0x09;
const V18_C2S_ANIMATION: u8 = 0x0A;
const V18_C2S_TAB_COMPLETE: u8 = 0x14;
const V18_C2S_SETTINGS: u8 = 0x15;
const V18_C2S_WINDOW_CLICK: u8 = 0x0F;
const V18_C2S_ENTITY_ACTION: u8 = 0x0B;
#[allow(dead_code)]
const V18_C2S_CONFIRM_TRANSACTION: u8 = 0x0F;

pub fn convert_c2s(payload: Bytes, server_proto: u32) -> ConversionResult {
    let Some((id, body)) = split_id(payload.clone()) else {
        return ConversionResult::Passthrough;
    };

    let ver = nearest(server_proto);

    match id {
        V18_C2S_CHAT => c2s_chat(body, ver),
        V18_C2S_PLAYER_POS_LOOK => c2s_player_pos_look(body, ver),
        V18_C2S_HELD_ITEM => c2s_held_item(body, ver),
        V18_C2S_ANIMATION => c2s_animation(body, ver),
        V18_C2S_USE_ENTITY => c2s_use_entity(body, ver),
        V18_C2S_BLOCK_PLACE => c2s_block_place(body, ver),
        V18_C2S_DIGGING => c2s_digging(body, ver),
        V18_C2S_TAB_COMPLETE => c2s_tab_complete(body, ver),
        V18_C2S_SETTINGS => c2s_settings(body, ver),
        V18_C2S_WINDOW_CLICK => c2s_window_click(body, ver),
        V18_C2S_ENTITY_ACTION => c2s_entity_action(body, ver),
        _ => ConversionResult::Passthrough,
    }
}

fn c2s_chat(mut body: Bytes, ver: ProtocolVersion) -> ConversionResult {
    let Ok(msg) = String::decode(&mut body) else {
        return ConversionResult::Passthrough;
    };

    match ver {
        ProtocolVersion::V1_8 | ProtocolVersion::V1_12_2 => {
            let mut out = BytesMut::new();
            msg.encode(&mut out).unwrap();
            ConversionResult::Converted(vec![build_payload(0x02, &out)])
        },
        _ => {
            use std::time::{SystemTime, UNIX_EPOCH};
            let ts = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as i64;

            let mut out = BytesMut::new();
            msg.encode(&mut out).unwrap();
            out.put_i64(ts);
            out.put_i64(0);
            VarInt(0).encode(&mut out).unwrap();
            out.put_u8(0);
            ConversionResult::Converted(vec![build_payload(0x05, &out)])
        },
    }
}

fn c2s_player_pos_look(body: Bytes, _ver: ProtocolVersion) -> ConversionResult {
    ConversionResult::Converted(vec![build_payload(0x0D, &body)])
}

fn c2s_held_item(mut body: Bytes, _ver: ProtocolVersion) -> ConversionResult {
    if body.remaining() < 2 {
        return ConversionResult::Passthrough;
    }
    let slot = body.get_i16() as i32;
    let mut out = BytesMut::new();
    VarInt(slot).encode(&mut out).unwrap();
    ConversionResult::Converted(vec![build_payload(0x21, &out)])
}

fn c2s_animation(_body: Bytes, _ver: ProtocolVersion) -> ConversionResult {
    let mut out = BytesMut::new();
    VarInt(0).encode(&mut out).unwrap();
    ConversionResult::Converted(vec![build_payload(0x27, &out)])
}

fn c2s_use_entity(body: Bytes, ver: ProtocolVersion) -> ConversionResult {
    match ver {
        ProtocolVersion::V1_8 | ProtocolVersion::V1_12_2 => ConversionResult::Passthrough,
        _ => {
            let mut out = BytesMut::from(body.as_ref());
            out.put_u8(0);
            ConversionResult::Converted(vec![build_payload(0x0E, &out)])
        },
    }
}

fn c2s_block_place(mut body: Bytes, _ver: ProtocolVersion) -> ConversionResult {
    if body.remaining() < 9 {
        return ConversionResult::Passthrough;
    }
    let x = body.get_i32();
    let y = body.get_u8() as i32;
    let z = body.get_i32();
    let direction = body.get_i8() as i32;

    let packed: i64 = (((x as i64) & 0x3FF_FFFF) << 38)
        | (((z as i64) & 0x3FF_FFFF) << 12)
        | ((y as i64) & 0xFFF);

    let mut out = BytesMut::new();
    out.put_i64(packed);
    VarInt(direction).encode(&mut out).unwrap();
    VarInt(0).encode(&mut out).unwrap();
    out.put_f32(0.5);
    out.put_f32(0.5);
    out.put_f32(0.5);
    out.put_u8(0);
    ConversionResult::Converted(vec![build_payload(0x2C, &out)])
}

fn c2s_digging(mut body: Bytes, _ver: ProtocolVersion) -> ConversionResult {
    if body.remaining() < 11 {
        return ConversionResult::Passthrough;
    }
    let status = body.get_u8() as i32;
    let x = body.get_i32();
    let y = body.get_u8() as i32;
    let z = body.get_i32();
    let face = body.get_i8() as i32;

    let packed: i64 = (((x as i64) & 0x3FF_FFFF) << 38)
        | (((z as i64) & 0x3FF_FFFF) << 12)
        | ((y as i64) & 0xFFF);

    let mut out = BytesMut::new();
    VarInt(status).encode(&mut out).unwrap();
    out.put_i64(packed);
    VarInt(face).encode(&mut out).unwrap();
    ConversionResult::Converted(vec![build_payload(0x1A, &out)])
}

fn c2s_tab_complete(mut body: Bytes, ver: ProtocolVersion) -> ConversionResult {
    let Ok(text) = String::decode(&mut body) else {
        return ConversionResult::Passthrough;
    };
    match ver {
        ProtocolVersion::V1_8 => ConversionResult::Passthrough,
        _ => {
            let mut out = BytesMut::new();
            VarInt(0).encode(&mut out).unwrap();
            text.encode(&mut out).unwrap();
            ConversionResult::Converted(vec![build_payload(0x06, &out)])
        },
    }
}

fn c2s_settings(body: Bytes, ver: ProtocolVersion) -> ConversionResult {
    match ver {
        ProtocolVersion::V1_8 | ProtocolVersion::V1_12_2 => ConversionResult::Passthrough,
        _ => {
            let mut out = BytesMut::from(body.as_ref());
            VarInt(1).encode(&mut out).unwrap();
            ConversionResult::Converted(vec![build_payload(0x05, &out)])
        },
    }
}

fn c2s_window_click(body: Bytes, ver: ProtocolVersion) -> ConversionResult {
    match ver {
        ProtocolVersion::V1_8 | ProtocolVersion::V1_12_2 => ConversionResult::Passthrough,
        _ => {
            let mut out = BytesMut::from(body.as_ref());
            VarInt(0).encode(&mut out).unwrap();
            ConversionResult::Converted(vec![build_payload(0x09, &out)])
        },
    }
}

fn c2s_entity_action(body: Bytes, ver: ProtocolVersion) -> ConversionResult {
    match ver {
        ProtocolVersion::V1_8 | ProtocolVersion::V1_12_2 => ConversionResult::Passthrough,
        _ => {
            if body.remaining() < 1 {
                return ConversionResult::Passthrough;
            }
            let action = body[0] as i32;
            let mut out = BytesMut::new();
            VarInt(0).encode(&mut out).unwrap();
            VarInt(action).encode(&mut out).unwrap();
            out.extend_from_slice(&body[1..]);
            ConversionResult::Converted(vec![build_payload(0x1C, &out)])
        },
    }
}
