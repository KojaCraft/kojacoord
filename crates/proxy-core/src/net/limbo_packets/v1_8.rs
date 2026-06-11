//! Limbo packets for the v1_8_x canonical bucket (1.8).

use bytes::{BufMut, BytesMut};
use kojacoord_protocol::codec::Encode;
use kojacoord_protocol::types::VarInt;
use kojacoord_protocol::versions::v1_8_x::play as p;
use uuid::Uuid;

use super::{encode, EncodedPacket, LimboPackets, PlayerPos, SoundParams};

pub struct V1_8;

impl LimboPackets for V1_8 {
    fn join_game(&self, proto: u32, _world_name: &str) -> Option<EncodedPacket> {
        encode(
            proto,
            p::ClientboundJoinGame {
                entity_id: 0,
                game_mode: 0x03,
                dimension: 0,
                difficulty: 0,
                max_players: 20,
                level_type: "flat".to_string(),
                reduced_debug_info: false,
            },
        )
    }

    fn respawn(&self, proto: u32, _world_name: &str) -> Option<EncodedPacket> {
        encode(
            proto,
            p::ClientboundRespawn {
                dimension: 0,
                difficulty: 0,
                game_mode: 0,
                level_type: "flat".to_string(),
            },
        )
    }

    fn player_abilities(&self, proto: u32) -> Option<EncodedPacket> {
        encode(
            proto,
            p::ClientboundPlayerAbilities {
                flags: 0x06,
                flying_speed: 0.0,
                field_of_view_modifier: 0.0,
            },
        )
    }

    fn held_item_change(&self, proto: u32) -> Option<EncodedPacket> {
        encode(proto, p::ClientboundSetHeldItem { slot: 0 })
    }

    fn player_position(
        &self,
        proto: u32,
        pos: PlayerPos,
        _teleport_id: i32,
    ) -> Option<EncodedPacket> {
        encode(
            proto,
            p::ClientboundPlayerPosition {
                x: pos.x,
                y: pos.y,
                z: pos.z,
                yaw: pos.yaw,
                pitch: pos.pitch,
                flags: 0,
            },
        )
    }

    fn chat(&self, proto: u32, json_message: &str) -> Option<EncodedPacket> {
        encode(
            proto,
            p::ClientboundChatMessage {
                json_message: json_message.to_owned(),
                position: 1,
            },
        )
    }

    fn note_sound(&self, proto: u32, pos: SoundParams) -> Option<EncodedPacket> {
        encode(
            proto,
            p::ClientboundSound {
                sound_name: "records.cat".to_owned(),
                x: pos.x as i32,
                y: pos.y as i32,
                z: pos.z as i32,
                volume: pos.volume,
                pitch: (pos.pitch * 63.0) as u8,
            },
        )
    }

    fn bossbar_add(&self, _proto: u32, _uuid: Uuid, _title: &str) -> Option<EncodedPacket> {
        // 1.8 has no bossbar — that came in 1.9.
        None
    }

    fn bossbar_remove(&self, _proto: u32, _uuid: Uuid) -> Option<EncodedPacket> {
        None
    }

    fn keepalive(&self, proto: u32, id: i64) -> Option<EncodedPacket> {
        encode(
            proto,
            p::ClientboundKeepAlive {
                keep_alive_id: VarInt(id as i32),
            },
        )
    }

    fn brand(&self, proto: u32, brand: &str) -> Option<EncodedPacket> {
        let mut data = BytesMut::new();
        VarInt(brand.len() as i32).encode(&mut data).ok()?;
        data.put_slice(brand.as_bytes());
        encode(
            proto,
            p::ClientboundPluginMessage {
                channel: "MC|Brand".to_owned(),
                data: data.to_vec(),
            },
        )
    }
}
