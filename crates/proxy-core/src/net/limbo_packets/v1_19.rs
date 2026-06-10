//! Limbo packets for the v1_19_x canonical bucket (1.17 – 1.19.4).
//!
//! 1.17 and 1.18 (proto 755-758) use an NBT dimension shape that we
//! don't synthesise. Methods affected by that gate (Login, Respawn,
//! SystemChat) return `None` for those protos.

use bytes::{BufMut, BytesMut};
use kojacoord_protocol::codec::Encode;
use kojacoord_protocol::types::VarInt;
use kojacoord_protocol::versions::v1_19_x::play as p;
use uuid::Uuid;

use super::{encode, EncodedPacket, LimboPackets, PlayerPos, SoundParams};

pub struct V1_19;

impl LimboPackets for V1_19 {
    fn join_game(&self, proto: u32, world_name: &str) -> Option<EncodedPacket> {
        if proto < 759 {
            return None;
        }
        encode(
            proto,
            p::ClientboundLogin {
                entity_id: 0,
                is_hardcore: false,
                game_mode: 3,
                previous_game_mode: -1,
                dimensions: vec![world_name.to_owned()],
                registry_codec: vec![],
                dimension_type: "minecraft:overworld".to_owned(),
                dimension_name: world_name.to_owned(),
                hashed_seed: 0,
                max_players: VarInt(20),
                chunk_radius: VarInt(8),
                simulation_distance: VarInt(8),
                reduced_debug_info: false,
                enable_respawn_screen: true,
                is_debug: false,
                is_flat: true,
                death_location: None,
            },
        )
    }

    fn respawn(&self, proto: u32, world_name: &str) -> Option<EncodedPacket> {
        if proto < 759 {
            return None;
        }
        encode(
            proto,
            p::ClientboundRespawn {
                dimension_type: "minecraft:overworld".to_owned(),
                dimension_name: world_name.to_owned(),
                hashed_seed: 0,
                game_mode: 0,
                previous_game_mode: -1,
                is_debug: false,
                is_flat: true,
                data_kept: 0,
                death_location: None,
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
        encode(proto, p::ClientboundSetCarriedItem { slot: 0 })
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
        if proto < 759 {
            return None; // SystemChat is 1.19+.
        }
        encode(
            proto,
            p::ClientboundSystemChat {
                content: json_message.to_owned(),
                overlay: false,
            },
        )
    }

    fn note_sound(&self, proto: u32, pos: SoundParams) -> Option<EncodedPacket> {
        // Use the v1_21_x ClientboundSound shape — same wire format
        // across 1.19+; registry resolves the id.
        encode(
            proto,
            kojacoord_protocol::versions::v1_21_x::play::ClientboundSound {
                sound_name: "minecraft:music_disc.cat".to_owned(),
                sound_category: VarInt(2),
                sound_type: VarInt(0),
                effect_pos_x: (pos.x * 8.0) as i32,
                effect_pos_y: (pos.y * 8.0) as i32,
                effect_pos_z: (pos.z * 8.0) as i32,
                volume: pos.volume,
                pitch: pos.pitch,
                seed: 0,
            },
        )
    }

    fn bossbar_add(&self, proto: u32, uuid: Uuid, title: &str) -> Option<EncodedPacket> {
        encode(
            proto,
            kojacoord_protocol::versions::v1_20_x::play::ClientboundBossBar {
                uuid,
                action: kojacoord_protocol::versions::v1_20_x::play::BossBarAction::Add {
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
            kojacoord_protocol::versions::v1_20_x::play::ClientboundBossBar {
                uuid,
                action: kojacoord_protocol::versions::v1_20_x::play::BossBarAction::Remove,
            },
        )
    }

    fn keepalive(&self, proto: u32, id: i64) -> Option<EncodedPacket> {
        encode(proto, p::ClientboundKeepAlive { id })
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
