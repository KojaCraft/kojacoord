//! Limbo packets for 1.6.x (pre-netty). Most are absent — 1.6 has no
//! configuration phase, no JoinGame in this shape, etc.

use kojacoord_protocol::versions::v1_6_x::play as p;
use uuid::Uuid;

use super::{encode, EncodedPacket, LimboPackets, PlayerPos, SoundParams};

pub struct V1_6;

impl LimboPackets for V1_6 {
    fn join_game(&self, _proto: u32, _world_name: &str) -> Option<EncodedPacket> {
        // 1.6.x has a completely different login dance; we never emit
        // a JoinGame at the typed layer for it.
        None
    }

    fn respawn(&self, proto: u32, _world_name: &str) -> Option<EncodedPacket> {
        encode(
            proto,
            p::ClientboundRespawn {
                dimension: 0,
                difficulty: 0,
                gamemode: 0,
                world_height: 256,
            },
        )
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

    fn held_item_change(&self, proto: u32) -> Option<EncodedPacket> {
        encode(proto, p::ClientboundHeldItemChange { slot: 0 })
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
                stance: pos.y + 1.62,
                z: pos.z,
                yaw: pos.yaw,
                pitch: pos.pitch,
                on_ground: true,
            },
        )
    }

    fn chat(&self, proto: u32, json_message: &str) -> Option<EncodedPacket> {
        // 1.6.4 chat is a plain UCS-2 string, not JSON.
        let text = crate::packet_builder::plaintext_from_chat_json(json_message);
        encode(proto, p::ClientboundChatMessage { message: text })
    }

    fn note_sound(&self, _proto: u32, _pos: SoundParams) -> Option<EncodedPacket> {
        // 1.6.x sound packet shape isn't in our typed surface; skip.
        None
    }

    fn bossbar_add(&self, _proto: u32, _uuid: Uuid, _title: &str) -> Option<EncodedPacket> {
        None // bossbars are 1.9+
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

    fn brand(&self, _proto: u32, _brand: &str) -> Option<EncodedPacket> {
        // 1.6.x has no plugin-message brand channel.
        None
    }
}
