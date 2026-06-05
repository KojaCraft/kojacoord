use bytes::{Bytes, BytesMut};
use kojacoord_protocol::{codec::Encode, types::VarInt, ProtocolVersion};
use uuid::Uuid;

use crate::{
    modloader,
    packet_ids::{cb_chat_id, cb_play, cb_plugin_message_id, nearest, sb_plugin_message_id},
    plugin_decoder,
};

pub fn build_system_message_packet(text: &str, proto: u32) -> Bytes {
    let json = serde_json::json!({ "text": text, "color": "yellow" }).to_string();
    let pid = cb_chat_id(proto);
    let mut payload = BytesMut::new();
    VarInt(pid as i32).encode(&mut payload).unwrap();

    match nearest(proto) {
        ProtocolVersion::V1_6_4 | ProtocolVersion::V1_7_10 => {
            use kojacoord_protocol::versions::v1_7_10::play::ClientboundChatMessage;
            ClientboundChatMessage {
                json_message: json,
                position: 1,
            }
            .encode(&mut payload)
            .unwrap();
        },
        ProtocolVersion::V1_8 | ProtocolVersion::V1_12_2 => {
            use kojacoord_protocol::versions::v1_12_2::play::ClientboundChatMessage;
            ClientboundChatMessage {
                json_message: json,
                position: 1,
            }
            .encode(&mut payload)
            .unwrap();
        },
        ProtocolVersion::V1_16_5 => {
            use kojacoord_protocol::versions::v1_16_5::play::ClientboundChatMessage;
            ClientboundChatMessage {
                json_message: json,
                position: 1,
                sender: Uuid::nil(),
            }
            .encode(&mut payload)
            .unwrap();
        },
        _ => {
            use kojacoord_protocol::versions::v1_20_4::play::ClientboundSystemChat;
            ClientboundSystemChat {
                content: json,
                overlay: false,
            }
            .encode(&mut payload)
            .unwrap();
        },
    }

    payload.freeze()
}

pub fn build_plugin_message_packet(channel: &str, data: &[u8], proto: u32) -> Bytes {
    let pid = cb_plugin_message_id(proto);
    let body = plugin_decoder::encode_plugin_message(channel, data, proto).unwrap_or_else(|_| {
        let mut b = BytesMut::new();
        channel.to_owned().encode(&mut b).unwrap();
        b.extend_from_slice(data);
        b.freeze()
    });
    let mut payload = BytesMut::with_capacity(1 + body.len());
    VarInt(pid as i32).encode(&mut payload).unwrap();
    payload.extend_from_slice(&body);
    payload.freeze()
}

pub fn build_serverbound_plugin_message_packet(channel: &str, data: &[u8], proto: u32) -> Bytes {
    let pid = sb_plugin_message_id(proto);
    let body = plugin_decoder::encode_plugin_message(channel, data, proto).unwrap_or_else(|_| {
        let mut b = BytesMut::new();
        channel.to_owned().encode(&mut b).unwrap();
        b.extend_from_slice(data);
        b.freeze()
    });
    let mut payload = BytesMut::with_capacity(1 + body.len());
    VarInt(pid as i32).encode(&mut payload).unwrap();
    payload.extend_from_slice(&body);
    payload.freeze()
}

pub fn build_brand_packet(kind: modloader::ModloaderKind, proto: u32) -> Bytes {
    let brand_str: &str = match kind {
        modloader::ModloaderKind::Fml1 | modloader::ModloaderKind::Fml2 => "fml,bukkit",
        modloader::ModloaderKind::Fml3 => "forge",
        modloader::ModloaderKind::NeoForge => "neoforge",
        modloader::ModloaderKind::Fabric => "fabric",
        modloader::ModloaderKind::Unknown | modloader::ModloaderKind::Vanilla => "Kojacoord",
    };

    let pid = cb_plugin_message_id(proto);
    let mut payload = BytesMut::new();
    VarInt(pid as i32).encode(&mut payload).unwrap();

    if proto <= 47 {
        "MC|Brand".to_owned().encode(&mut payload).unwrap();
    } else {
        "minecraft:brand".to_owned().encode(&mut payload).unwrap();
    }
    brand_str.to_owned().encode(&mut payload).unwrap();

    payload.freeze()
}

pub fn build_disconnect_packet(json_reason: &str, proto: u32) -> Bytes {
    let pkt_id = cb_play(proto, "ClientboundDisconnect");
    let mut payload = BytesMut::new();
    VarInt(pkt_id as i32).encode(&mut payload).unwrap();

    match nearest(proto) {
        ProtocolVersion::V1_6_4 => {
            use kojacoord_protocol::versions::v1_6_4::play::ClientboundDisconnect;
            ClientboundDisconnect {
                reason: json_reason.to_string(),
            }
            .encode(&mut payload)
            .unwrap();
        },
        ProtocolVersion::V1_7_10 => {
            use kojacoord_protocol::versions::v1_7_10::play::ClientboundDisconnect;
            ClientboundDisconnect {
                reason: json_reason.to_string(),
            }
            .encode(&mut payload)
            .unwrap();
        },
        ProtocolVersion::V1_8 | ProtocolVersion::V1_12_2 => {
            use kojacoord_protocol::versions::v1_12_2::play::ClientboundDisconnect;
            ClientboundDisconnect {
                reason: json_reason.to_string(),
            }
            .encode(&mut payload)
            .unwrap();
        },
        ProtocolVersion::V1_16_5 => {
            use kojacoord_protocol::versions::v1_16_5::play::ClientboundDisconnect;
            ClientboundDisconnect {
                reason: json_reason.to_string(),
            }
            .encode(&mut payload)
            .unwrap();
        },
        ProtocolVersion::V1_19_4 => {
            use kojacoord_protocol::versions::v1_19_4::play::ClientboundDisconnect;
            ClientboundDisconnect {
                reason: json_reason.to_string(),
            }
            .encode(&mut payload)
            .unwrap();
        },
        ProtocolVersion::V1_20_4 => {
            use kojacoord_protocol::versions::v1_20_4::play::ClientboundDisconnect;
            ClientboundDisconnect {
                reason: json_reason.to_string(),
            }
            .encode(&mut payload)
            .unwrap();
        },
        ProtocolVersion::V1_21 => {
            use kojacoord_protocol::versions::v1_21::play::ClientboundDisconnect;
            ClientboundDisconnect {
                reason: json_reason.to_string(),
            }
            .encode(&mut payload)
            .unwrap();
        },
        _ => {
            let reason_bytes = json_reason.as_bytes();
            VarInt(reason_bytes.len() as i32)
                .encode(&mut payload)
                .unwrap();
            payload.extend_from_slice(reason_bytes);
        },
    }

    payload.freeze()
}
