//! Per-canonical-version packet builders for limbo.
//!
//! Each `LimboPackets` impl knows how to build one canonical-version
//! family's wire-shape for every packet limbo emits (JoinGame, Respawn,
//! Position, KeepAlive, Chat, Sound, BossBar, etc.) — returning an
//! [`EncodedPacket`] = `(packet_id, body)`. The `LimboHandler` keeps a
//! `&'static dyn LimboPackets` pointer chosen at construction time;
//! every `send_*` method becomes a one-liner that asks the impl for
//! the encoded bytes and writes them.
//!
//! Adding a new canonical version is one new module file plus one
//! entry in [`for_version`] — no edits to `limbo.rs`.
//!
//! `None` returned from a builder means "this version doesn't speak
//! that packet" (e.g. pre-netty has no BossBar). The handler skips it.

use bytes::BytesMut;
use kojacoord_protocol::CanonicalVersion;
use uuid::Uuid;

// Canonical buckets — own struct construction logic.
pub mod v1_12;
pub mod v1_16;
pub mod v1_19;
pub mod v1_20;
pub mod v1_21;
pub mod v1_6;
pub mod v1_7;
pub mod v1_8;

// Minor-version aliases — each re-exports its canonical bucket so
// downstream code can name the version directly. (1.9.x/1.10.x/1.11.x →
// v1_12; 1.13.x/1.14.x/1.15.x → v1_16; 1.17.x/1.18.x → v1_19.)
pub mod v1_10;
pub mod v1_11;
pub mod v1_13;
pub mod v1_14;
pub mod v1_15;
pub mod v1_17;
pub mod v1_18;
pub mod v1_9;

/// A wire-encoded limbo packet — packet id followed by the body.
/// The handler will prepend the VarInt(id) and frame the result.
pub struct EncodedPacket {
    pub id: u8,
    pub body: BytesMut,
}

/// Position emitted by `send_player_position`.
#[derive(Debug, Clone, Copy)]
pub struct PlayerPos {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub yaw: f32,
    pub pitch: f32,
}

/// Sound parameters. The exact id-vs-name mapping varies per version;
/// the impl picks whichever its struct expects.
#[derive(Debug, Clone, Copy)]
pub struct SoundParams {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub volume: f32,
    pub pitch: f32,
}

/// Every method takes `proto` (the negotiated wire protocol number)
/// because each canonical bucket spans several protocol numbers and the
/// returned id is looked up through the central registry, which keys on
/// the exact proto. Returns `None` if this version doesn't support that
/// packet (e.g. SystemChat on 1.17/1.18, BossBar on pre-1.9).
pub trait LimboPackets: Send + Sync {
    /// Build the initial JoinGame / Login packet (limbo flat world).
    fn join_game(&self, proto: u32, world_name: &str) -> Option<EncodedPacket>;

    /// Build a Respawn packet — used both when leaving limbo and when
    /// transitioning between worlds.
    fn respawn(&self, proto: u32, world_name: &str) -> Option<EncodedPacket>;

    /// Build a PlayerAbilities packet (flight, etc.).
    fn player_abilities(&self, proto: u32) -> Option<EncodedPacket>;

    /// Build a HeldItemChange / SetCarriedItem (slot 0).
    fn held_item_change(&self, proto: u32) -> Option<EncodedPacket>;

    /// Build a PlayerPosition packet anchoring the client to limbo.
    fn player_position(
        &self,
        proto: u32,
        pos: PlayerPos,
        teleport_id: i32,
    ) -> Option<EncodedPacket>;

    /// Build a chat / system-chat message.
    fn chat(&self, proto: u32, json_message: &str) -> Option<EncodedPacket>;

    /// Build a note-block sound effect at the limbo location.
    fn note_sound(&self, proto: u32, pos: SoundParams) -> Option<EncodedPacket>;

    /// Build a BossBar Add packet (or None for versions without bossbars).
    fn bossbar_add(&self, proto: u32, uuid: Uuid, title: &str) -> Option<EncodedPacket>;

    /// Build a BossBar Remove packet for the given uuid.
    fn bossbar_remove(&self, proto: u32, uuid: Uuid) -> Option<EncodedPacket>;

    /// Build a KeepAlive packet for the given id.
    fn keepalive(&self, proto: u32, id: i64) -> Option<EncodedPacket>;

    /// Build a clientbound PluginMessage containing the server brand.
    fn brand(&self, proto: u32, brand: &str) -> Option<EncodedPacket>;

    /// 1.6.x-only essentials. Returning `None` by default makes the
    /// other canonical buckets no-op these — modern clients don't
    /// need a SpawnPosition broadcast to render their HUD; they take
    /// it from the JoinGame coordinate fields directly.
    ///
    /// `spawn_position`: tells pre-netty clients where the compass
    /// should point. Without it the compass UI stays blank.
    fn spawn_position(&self, _proto: u32, _pos: PlayerPos) -> Option<EncodedPacket> {
        None
    }

    /// `time_update`: pre-netty world stays at midnight (black sky)
    /// without a TimeUpdate. Modern clients use a different packet
    /// shape per epoch — limbo doesn't need to send it on those.
    fn time_update(&self, _proto: u32) -> Option<EncodedPacket> {
        None
    }

    /// `update_health`: pre-netty clients render the respawn screen
    /// (and reject input) until they see UpdateHealth with `health > 0`.
    /// Modern clients seed their HUD from JoinGame.
    fn update_health(&self, _proto: u32) -> Option<EncodedPacket> {
        None
    }
}

/// Static dispatch: pick the [`LimboPackets`] implementation that
/// matches `canonical` once at construction time, then call its
/// methods on the hot path.
pub fn for_version(canonical: CanonicalVersion) -> &'static dyn LimboPackets {
    match canonical {
        CanonicalVersion::V1_6_4 => &v1_6::V1_6,
        CanonicalVersion::V1_7_10 => &v1_7::V1_7,
        CanonicalVersion::V1_8 => &v1_8::V1_8,
        CanonicalVersion::V1_12_2 => &v1_12::V1_12,
        CanonicalVersion::V1_16_5 => &v1_16::V1_16,
        CanonicalVersion::V1_19_4 => &v1_19::V1_19,
        CanonicalVersion::V1_20_4 => &v1_20::V1_20,
        CanonicalVersion::V1_21 => &v1_21::V1_21,
    }
}

/// Helper used by every impl: encode a typed packet into an
/// [`EncodedPacket`] using `PacketId::packet_id(proto)` for the id and
/// `Encode::encode` for the body.
pub(crate) fn encode<T: kojacoord_protocol::codec::Encode + kojacoord_protocol::codec::PacketId>(
    proto: u32,
    pkt: T,
) -> Option<EncodedPacket> {
    let id = T::packet_id(proto);
    if id == 0xFF {
        return None;
    }
    let mut body = BytesMut::new();
    pkt.encode(&mut body).ok()?;
    Some(EncodedPacket { id, body })
}
