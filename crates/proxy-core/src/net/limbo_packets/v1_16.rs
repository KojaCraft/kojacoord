//! Limbo packets for the v1_16_x canonical bucket (1.13 – 1.16.5).

use bytes::{BufMut, BytesMut};
use kojacoord_protocol::codec::Encode;
use kojacoord_protocol::types::VarInt;
use kojacoord_protocol::versions::v1_16_x::play as p;
use uuid::Uuid;

use super::{encode, EncodedPacket, LimboPackets, PlayerPos, SoundParams};

pub struct V1_16;

impl LimboPackets for V1_16 {
    fn join_game(&self, proto: u32, world_name: &str) -> Option<EncodedPacket> {
        encode(
            proto,
            p::ClientboundJoinGame {
                entity_id: 0,
                is_hardcore: false,
                game_mode: 3,
                previous_game_mode: -1,
                world_names: vec![world_name.to_owned()],
                dimension: "minecraft:overworld".to_owned(),
                world_name: world_name.to_owned(),
                hashed_seed: 0,
                max_players: VarInt(20),
                view_distance: VarInt(8),
                reduced_debug_info: false,
                enable_respawn_screen: true,
                is_debug: false,
                is_flat: true,
            },
        )
    }

    fn respawn(&self, proto: u32, world_name: &str) -> Option<EncodedPacket> {
        encode(
            proto,
            p::ClientboundRespawn {
                dimension: "minecraft:overworld".to_owned(),
                world_name: world_name.to_owned(),
                hashed_seed: 0,
                game_mode: 0,
                previous_game_mode: -1,
                is_debug: false,
                is_flat: true,
                copy_metadata: false,
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
        teleport_id: i32,
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
                teleport_id: VarInt(teleport_id),
            },
        )
    }

    fn chat(&self, proto: u32, json_message: &str) -> Option<EncodedPacket> {
        encode(
            proto,
            p::ClientboundChatMessage {
                json_message: json_message.to_owned(),
                position: 1,
                sender: Uuid::nil(),
            },
        )
    }

    fn note_sound(&self, proto: u32, pos: SoundParams) -> Option<EncodedPacket> {
        encode(
            proto,
            p::ClientboundNamedSoundEffect {
                sound_name: "minecraft:records.cat".to_owned(),
                sound_category: VarInt(2),
                effect_position_x: (pos.x * 8.0) as i32,
                effect_position_y: (pos.y * 8.0) as i32,
                effect_position_z: (pos.z * 8.0) as i32,
                volume: pos.volume,
                pitch: pos.pitch,
            },
        )
    }

    fn bossbar_add(&self, proto: u32, uuid: Uuid, title: &str) -> Option<EncodedPacket> {
        // The v1_16_x module doesn't re-export BossBar after the prune;
        // we reuse the 1.12 typed struct which has the same wire shape
        // on 1.16 (the registry resolves the id correctly per proto).
        encode(
            proto,
            kojacoord_protocol::versions::v1_12_x::play::ClientboundBossBar {
                uuid,
                action: kojacoord_protocol::versions::v1_12_x::play::BossBarAction::Add {
                    title: title.to_owned(),
                    health: 1.0,
                    color: VarInt(1),
                    division: VarInt(0),
                    flags: 0,
                },
            },
        )
    }

    fn bossbar_remove(&self, proto: u32, uuid: Uuid) -> Option<EncodedPacket> {
        encode(
            proto,
            kojacoord_protocol::versions::v1_12_x::play::ClientboundBossBar {
                uuid,
                action: kojacoord_protocol::versions::v1_12_x::play::BossBarAction::Remove,
            },
        )
    }

    fn keepalive(&self, proto: u32, id: i64) -> Option<EncodedPacket> {
        encode(proto, p::ClientboundKeepAlive { keep_alive_id: id })
    }

    fn brand(&self, proto: u32, brand: &str) -> Option<EncodedPacket> {
        let mut data = BytesMut::new();
        VarInt(brand.len() as i32).encode(&mut data).ok()?;
        data.put_slice(brand.as_bytes());
        encode(
            proto,
            kojacoord_protocol::versions::v1_20_x::play::ClientboundPluginMessage {
                channel: "minecraft:brand".to_owned(),
                data: data.to_vec(),
            },
        )
    }
}
