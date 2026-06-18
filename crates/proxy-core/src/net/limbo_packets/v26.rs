//! Limbo packets for the v26 canonical bucket (26.1 – 26.2, proto 775/776).
//!
//! 26.1 reuses the 1.21.x play wire shapes; only the play packet ids shifted
//! (ViaVersion `ClientboundPackets26_1`). Every shape-identical builder
//! delegates to [`V1_21`], which already resolves ids through the registry.
//! The four ids that the V1_21 bucket hardcodes per-proto are overridden here
//! with the 26.1 ordinals.

use bytes::{BufMut, BytesMut};
use kojacoord_protocol::codec::Encode;
use kojacoord_protocol::types::VarInt;
use uuid::Uuid;

use super::v1_21::V1_21;
use super::{EncodedPacket, LimboPackets, PlayerPos, SoundParams};

pub struct V26;

impl LimboPackets for V26 {
    fn join_game(&self, proto: u32, world_name: &str) -> Option<EncodedPacket> {
        V1_21.join_game(proto, world_name)
    }

    fn respawn(&self, proto: u32, world_name: &str) -> Option<EncodedPacket> {
        V1_21.respawn(proto, world_name)
    }

    fn player_abilities(&self, proto: u32) -> Option<EncodedPacket> {
        V1_21.player_abilities(proto)
    }

    fn held_item_change(&self, proto: u32) -> Option<EncodedPacket> {
        V1_21.held_item_change(proto)
    }

    fn player_position(
        &self,
        proto: u32,
        pos: PlayerPos,
        teleport_id: i32,
    ) -> Option<EncodedPacket> {
        V1_21.player_position(proto, pos, teleport_id)
    }

    fn chat(&self, proto: u32, json_message: &str) -> Option<EncodedPacket> {
        V1_21.chat(proto, json_message)
    }

    fn note_sound(&self, proto: u32, pos: SoundParams) -> Option<EncodedPacket> {
        V1_21.note_sound(proto, pos)
    }

    fn bossbar_add(&self, proto: u32, uuid: Uuid, title: &str) -> Option<EncodedPacket> {
        V1_21.bossbar_add(proto, uuid, title)
    }

    fn bossbar_remove(&self, proto: u32, uuid: Uuid) -> Option<EncodedPacket> {
        V1_21.bossbar_remove(proto, uuid)
    }

    fn keepalive(&self, proto: u32, id: i64) -> Option<EncodedPacket> {
        V1_21.keepalive(proto, id)
    }

    fn brand(&self, proto: u32, brand: &str) -> Option<EncodedPacket> {
        V1_21.brand(proto, brand)
    }

    fn chunk_data(&self, proto: u32) -> Option<EncodedPacket> {
        V1_21.chunk_data(proto)
    }

    fn set_center_chunk(&self, _proto: u32) -> Option<EncodedPacket> {
        let mut body = BytesMut::new();
        VarInt(0).encode(&mut body).ok()?;
        VarInt(0).encode(&mut body).ok()?;
        Some(EncodedPacket { id: 0x5e, body })
    }

    fn chunk_batch_start(&self, _proto: u32) -> Option<EncodedPacket> {
        Some(EncodedPacket {
            id: 0x0c,
            body: BytesMut::new(),
        })
    }

    fn chunk_batch_finished(&self, _proto: u32, batch_size: i32) -> Option<EncodedPacket> {
        let mut body = BytesMut::new();
        VarInt(batch_size).encode(&mut body).ok()?;
        Some(EncodedPacket { id: 0x0b, body })
    }

    fn start_wait_chunks_event(&self, _proto: u32) -> Option<EncodedPacket> {
        let mut body = BytesMut::new();
        body.put_u8(13);
        body.put_f32(0.0);
        Some(EncodedPacket { id: 0x26, body })
    }
}
