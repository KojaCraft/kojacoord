//! Limbo packets for the v1_20_x canonical bucket (1.20 – 1.20.6).

use bytes::{BufMut, BytesMut};
use kojacoord_protocol::codec::Encode;
use kojacoord_protocol::types::VarInt;
use kojacoord_protocol::versions::v1_20_x::play as p;
use uuid::Uuid;

use super::{encode, EncodedPacket, LimboPackets, PlayerPos, SoundParams};

pub struct V1_20;

impl LimboPackets for V1_20 {
    fn join_game(&self, proto: u32, world_name: &str) -> Option<EncodedPacket> {
        encode(
            proto,
            p::ClientboundLogin {
                entity_id: 0,
                is_hardcore: false,
                dimension_names: vec![world_name.to_owned()],
                max_players: VarInt(20),
                view_distance: VarInt(8),
                simulation_distance: VarInt(8),
                reduced_debug_info: false,
                enable_respawn_screen: true,
                // `do_limited_crafting` was added in 1.20.2 (proto 764)
                // per BungeeCord `protocol/Login.java`:
                //   `if ( protocolVersion >= MINECRAFT_1_20_2 ) {
                //        limitedCrafting = buf.readBoolean();`
                // The Configuration-phase split also landed at 764, so
                // the entire post-1.20.2 Login(Play) compact form starts
                // here. 1.20-1.20.1 (763) do NOT carry it.
                do_limited_crafting: if proto >= 764 { Some(false) } else { None },
                // 1.20.2 / 1.20.4 (proto 764 / 765) expect an Identifier
                // here (`minecraft:overworld`); 1.20.5+ (proto 766) flipped
                // to a VarInt registry index. See DimensionTypeRef.
                dimension_type: if proto >= 766 {
                    p::DimensionTypeRef::Registry(VarInt(0))
                } else {
                    p::DimensionTypeRef::Identifier("minecraft:overworld".to_owned())
                },
                dimension_name: world_name.to_owned(),
                hashed_seed: 0,
                game_mode: 3,
                previous_game_mode: -1,
                is_debug: false,
                is_flat: true,
                death_location: None,
                portal_cooldown: VarInt(0),
                // `secure_profile` was added in proto 766 (1.20.5). For
                // 1.20-1.20.4 it must be absent. Per BungeeCord Login.java.
                secure_profile: if proto >= 766 { Some(false) } else { None },
            },
        )
    }

    fn respawn(&self, proto: u32, world_name: &str) -> Option<EncodedPacket> {
        encode(
            proto,
            p::ClientboundRespawn {
                dimension_type: VarInt(0),
                dimension_name: world_name.to_owned(),
                hashed_seed: 0,
                game_mode: 0,
                previous_game_mode: -1,
                is_debug: false,
                is_flat: true,
                data_kept: 0,
                death_location: None,
                portal_cooldown: VarInt(0),
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
        encode(
            proto,
            p::ClientboundSystemChat {
                content: json_message.to_owned(),
                overlay: false,
            },
        )
    }

    fn note_sound(&self, proto: u32, pos: SoundParams) -> Option<EncodedPacket> {
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
            p::ClientboundBossBar {
                uuid,
                action: p::BossBarAction::Add {
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
            p::ClientboundBossBar {
                uuid,
                action: p::BossBarAction::Remove,
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
            p::ClientboundPluginMessage {
                channel: "minecraft:brand".to_owned(),
                data: data.to_vec(),
            },
        )
    }
}
