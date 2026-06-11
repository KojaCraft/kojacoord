//! Limbo packets for the v1_7_x canonical bucket (1.7.x).

use bytes::{BufMut, BytesMut};
use kojacoord_protocol::codec::Encode;
use kojacoord_protocol::types::VarInt;
use kojacoord_protocol::versions::v1_7_x::play as p;
use uuid::Uuid;

use super::{encode, EncodedPacket, LimboPackets, PlayerPos, SoundParams};

pub struct V1_7;

impl LimboPackets for V1_7 {
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
            },
        )
    }

    fn respawn(&self, _proto: u32, _world_name: &str) -> Option<EncodedPacket> {
        // The v1_7_x typed module no longer exposes Respawn since the
        // proxy never sent one for 1.7 in practice — limbo→backend
        // hand-off bypasses 1.7 anyway. Return None so the handler
        // skips it cleanly.
        None
    }

    fn player_abilities(&self, proto: u32) -> Option<EncodedPacket> {
        encode(
            proto,
            p::ClientboundPlayerAbilities {
                flags: 0x06,
                flying_speed: 0.0,
                walking_speed: 0.0,
            },
        )
    }

    fn held_item_change(&self, _proto: u32) -> Option<EncodedPacket> {
        // SetHeldItem isn't re-exported by v1_7_x after the prune; the
        // 1.8 typed path covers the wire shape correctly for 1.7
        // (same i8 slot), so we'd rather skip than encode wrong bytes.
        None
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
                on_ground: true,
            },
        )
    }

    fn chat(&self, proto: u32, json_message: &str) -> Option<EncodedPacket> {
        encode(
            proto,
            p::ClientboundChatMessage {
                json_message: json_message.to_owned(),
            },
        )
    }

    fn note_sound(&self, _proto: u32, _pos: SoundParams) -> Option<EncodedPacket> {
        None
    }

    fn bossbar_add(&self, _proto: u32, _uuid: Uuid, _title: &str) -> Option<EncodedPacket> {
        None
    }

    fn bossbar_remove(&self, _proto: u32, _uuid: Uuid) -> Option<EncodedPacket> {
        None
    }

    fn keepalive(&self, proto: u32, id: i64) -> Option<EncodedPacket> {
        encode(
            proto,
            p::ClientboundKeepAlive {
                keep_alive_id: id as i32,
            },
        )
    }

    fn brand(&self, proto: u32, brand: &str) -> Option<EncodedPacket> {
        // 1.7 uses the legacy `MC|Brand` channel via the v1_8_x
        // PluginMessage shape (length-prefixed string, no minecraft:
        // namespace). We reuse the 1.8 typed packet here since 1.7
        // doesn't expose one.
        let mut data = BytesMut::new();
        VarInt(brand.len() as i32).encode(&mut data).ok()?;
        data.put_slice(brand.as_bytes());
        encode(
            proto,
            kojacoord_protocol::versions::v1_8_x::play::ClientboundPluginMessage {
                channel: "MC|Brand".to_owned(),
                data: data.to_vec(),
            },
        )
    }
}
