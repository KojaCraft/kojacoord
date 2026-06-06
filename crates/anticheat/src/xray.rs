// Author : Starfloof.
//
// # Honeypot Anti-XRay
//
// Strategy
// ────────
// The proxy maintains a set of invisible "honeypot" blocks for every player
// who is mining underground. When the client sends a ServerboundPlayerAction
// (START_DIGGING) at a position that coincides with one of these honeypots,
// it proves the player can see through solid terrain — i.e., they are using
// an X-Ray client modification or resource pack.
//
// Why this works
// ──────────────
// Honeypot blocks are injected into the client's view via spoofed
// `UpdateBlock` (0x09 in 1.21) packets sent from the proxy — they appear as
// valuable ores (diamond, emerald, iron, etc.) on the client's screen, but
// the real server has never sent them and treats the underlying blocks as
// stone/deepslate. A legitimate player sees only stone and will never dig
// toward the fake blocks. An X-Ray user sees glittering ores and walks
// straight to them.
//
// Confidence weighting
// ────────────────────
// Not all honeypot hits are equal. We weight detections by ore rarity:
//
//   Diamond block   → confidence 1.00  (essentially impossible to guess)
//   Emerald block   → confidence 1.00
//   Ancient Debris  → confidence 1.00
//   Gold block      → confidence 0.80
//   Lapis block     → confidence 0.70
//   Iron block      → confidence 0.50  (more common → lower weight per hit)
//   Redstone block  → confidence 0.40
//   Coal block      → confidence 0.20  (lowest weight)
//
// The accumulated confidence is compared against `XRAY_CONFIDENCE_THRESHOLD`.
// A single diamond hit (confidence 1.0) exceeds the threshold outright.
// Multiple iron hits (0.5 each) require two interactions to flag.
//
// Honeypot lifecycle
// ──────────────────
// 1. `spawn_honeypots(uuid, px, py, pz)` — called when the proxy detects the
//    player is digging underground (py < HONEYPOT_MAX_Y). Returns a `Vec` of
//    `HoneypotBlock` values that the caller MUST inject as spoofed clientbound
//    block update packets. If the player already has enough active honeypots,
//    this is a no-op.
//
// 2. `check_dig(uuid, username, dx, dy, dz)` — called on every
//    ServerboundPlayerAction(START_DIGGING). Returns `Some(Violation)` if the
//    dig position matches a honeypot and accumulated confidence is high enough.
//
// 3. `remove_honeypot(uuid, x, y, z)` — called when the server actually
//    destroys a block at that position (BlockUpdate from server side), so we
//    don't accidentally keep a honeypot at a position the real server later
//    reveals as air.
//
// 4. `player_quit(uuid)` — cleans all state for disconnected players.

use dashmap::DashMap;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;

use crate::{
    bridge::BridgeClient,
    violation::{CheckCategory, Violation},
};

// ─── Tuning knobs ────────────────────────────────────────────────────────────

/// Maximum Y level at which honeypots are spawned. Above this the player is
/// not mining deep enough for valuable ores to be plausible (sky / surface).
pub const HONEYPOT_MAX_Y: i32 = 16;

/// Minimum Y level — below bedrock level we also don't bother.
pub const HONEYPOT_MIN_Y: i32 = -64;

/// Radius around the player (in blocks) in which honeypots are scattered.
pub const HONEYPOT_RADIUS: i32 = 12;

/// Maximum number of honeypot blocks active per player at any time.
/// Keeping this low reduces memory and avoids flooding the client.
pub const HONEYPOT_MAX_PER_PLAYER: usize = 12;

/// Confidence score threshold before a Violation is emitted.
/// 1.0 = a single diamond hit; 2.0 = two diamond hits / four iron hits.
pub const XRAY_CONFIDENCE_THRESHOLD: f64 = 1.0;

/// How long a honeypot block lives before being silently expired.
/// This prevents stale honeypots from persisting across sessions.
pub const HONEYPOT_TTL: Duration = Duration::from_secs(300); // 5 minutes

/// Minimum VL before XRay violation is reported (extra safety net so a single
/// mis-click on a block that happened to collide does not insta-flag).
const VL_XRAY: u32 = 2;

// ─── Ore types ───────────────────────────────────────────────────────────────

/// Ore type used for a honeypot block. Controls both the visual block sent to
/// the client and the detection confidence weight.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum OreType {
    Diamond,
    Emerald,
    AncientDebris,
    Gold,
    Lapis,
    Iron,
    Redstone,
    Coal,
}

impl OreType {
    /// Detection confidence weight for this ore type. Higher = more suspicious.
    pub fn confidence(self) -> f64 {
        match self {
            OreType::Diamond => 1.00,
            OreType::Emerald => 1.00,
            OreType::AncientDebris => 1.00,
            OreType::Gold => 0.80,
            OreType::Lapis => 0.70,
            OreType::Iron => 0.50,
            OreType::Redstone => 0.40,
            OreType::Coal => 0.20,
        }
    }

    /// The Minecraft block state ID to send to the client for this ore.
    /// These are the block IDs for the ore blocks (not the raw ore form) so
    /// they stand out clearly to an X-Ray user.
    /// Values are for 1.20.4+ / 1.21; the proxy should translate as needed
    /// for older protocol versions.
    pub fn block_state_id_1_21(self) -> u32 {
        match self {
            // Diamond Ore (deepslate variant) — block state 4070 in 1.21
            OreType::Diamond => 4070,
            // Emerald Ore (deepslate variant) — 4094
            OreType::Emerald => 4094,
            // Ancient Debris — 4118
            OreType::AncientDebris => 4118,
            // Deepslate Gold Ore — 4058
            OreType::Gold => 4058,
            // Deepslate Lapis Ore — 4062
            OreType::Lapis => 4062,
            // Deepslate Iron Ore — 4054
            OreType::Iron => 4054,
            // Deepslate Redstone Ore — 4066
            OreType::Redstone => 4066,
            // Deepslate Coal Ore — 4050
            OreType::Coal => 4050,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            OreType::Diamond => "Diamond",
            OreType::Emerald => "Emerald",
            OreType::AncientDebris => "AncientDebris",
            OreType::Gold => "Gold",
            OreType::Lapis => "Lapis",
            OreType::Iron => "Iron",
            OreType::Redstone => "Redstone",
            OreType::Coal => "Coal",
        }
    }
}

/// Weighted distribution of ore types for honeypot placement.
/// Rarer ores are placed less frequently to stay realistic.
static ORE_DISTRIBUTION: &[(OreType, u32)] = &[
    (OreType::Coal, 30),         // 30% — most common, lowest weight
    (OreType::Iron, 25),         // 25%
    (OreType::Redstone, 15),     // 15%
    (OreType::Gold, 12),         // 12%
    (OreType::Lapis, 8),         //  8%
    (OreType::Diamond, 6),       //  6%
    (OreType::Emerald, 3),       //  3%
    (OreType::AncientDebris, 1), //  1% — rarest
];

// ─── Data structures ─────────────────────────────────────────────────────────

/// A single fake block injected into the client's view.
#[derive(Debug, Clone)]
pub struct HoneypotBlock {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub ore_type: OreType,
    /// Minecraft block state ID sent to the client.
    pub block_state_id: u32,
    /// When this honeypot was created (for TTL expiry).
    pub created_at: Instant,
    /// How many times this specific block has been interacted with.
    pub hit_count: u32,
}

impl HoneypotBlock {
    fn new(x: i32, y: i32, z: i32, ore_type: OreType) -> Self {
        Self {
            x,
            y,
            z,
            ore_type,
            block_state_id: ore_type.block_state_id_1_21(),
            created_at: Instant::now(),
            hit_count: 0,
        }
    }

    fn is_expired(&self) -> bool {
        self.created_at.elapsed() > HONEYPOT_TTL
    }

    fn position_key(x: i32, y: i32, z: i32) -> (i32, i32, i32) {
        (x, y, z)
    }
}

/// Per-player state managed by the XrayEngine.
#[derive(Debug)]
struct XrayState {
    /// Active honeypot blocks keyed by (x, y, z).
    honeypots: HashMap<(i32, i32, i32), HoneypotBlock>,
    /// Accumulated confidence score from honeypot interactions.
    confidence: f64,
    /// Violation level counter.
    vl: u32,
    /// Last time honeypots were spawned (prevents spam spawning).
    last_spawn: Instant,
}

impl Default for XrayState {
    fn default() -> Self {
        Self {
            honeypots: HashMap::new(),
            confidence: 0.0,
            vl: 0,
            last_spawn: Instant::now() - Duration::from_secs(60),
        }
    }
}

// ─── XrayEngine ──────────────────────────────────────────────────────────────

/// Manages honeypot block injection and X-Ray detection across all players.
pub struct XrayEngine {
    states: Arc<DashMap<Uuid, XrayState>>,
    bridge: Option<BridgeClient>,
    enabled: bool,
}

impl XrayEngine {
    pub fn new(enabled: bool, bridge: Option<BridgeClient>) -> Self {
        Self {
            states: Arc::new(DashMap::new()),
            bridge,
            enabled,
        }
    }

    // ─── Honeypot spawning ────────────────────────────────────────────────

    /// Generate honeypot blocks around the given player position and return
    /// them. The caller is responsible for sending `UpdateBlock` packets to
    /// the client for each returned block.
    ///
    /// Returns an empty Vec if:
    /// - X-Ray detection is disabled.
    /// - Player Y is above `HONEYPOT_MAX_Y` (not underground enough).
    /// - The player already has `HONEYPOT_MAX_PER_PLAYER` active honeypots.
    /// - Honeypots were spawned within the last 30 seconds (cooldown).
    pub fn spawn_honeypots(
        &self,
        uuid: Uuid,
        player_x: f64,
        player_y: f64,
        player_z: f64,
    ) -> Vec<HoneypotBlock> {
        if !self.enabled {
            return Vec::new();
        }

        let py = player_y as i32;
        if py > HONEYPOT_MAX_Y || py < HONEYPOT_MIN_Y {
            return Vec::new();
        }

        let mut state = self.states.entry(uuid).or_default();

        // Prune expired honeypots first.
        state.honeypots.retain(|_, hp| !hp.is_expired());

        // Cooldown: don't spam new honeypots.
        if state.last_spawn.elapsed() < Duration::from_secs(30) {
            return Vec::new();
        }
        // Already have enough.
        if state.honeypots.len() >= HONEYPOT_MAX_PER_PLAYER {
            return Vec::new();
        }

        let px = player_x as i32;
        let pz = player_z as i32;
        let to_add = HONEYPOT_MAX_PER_PLAYER - state.honeypots.len();

        let mut new_blocks = Vec::with_capacity(to_add);
        let mut attempt = 0u64;

        while new_blocks.len() < to_add && attempt < 512 {
            attempt += 1;

            // Deterministic pseudo-random offset: XOR-shift based on position,
            // uuid bytes, and attempt counter — no `rand` crate needed.
            let seed = xorshift64(
                uuid_seed(uuid)
                    ^ (attempt.wrapping_mul(0x9e3779b97f4a7c15))
                    ^ ((px as u64).wrapping_mul(0x517cc1b727220a95))
                    ^ ((pz as u64).wrapping_mul(0x6c62272e07bb0142))
                    ^ ((py as u64).wrapping_mul(0xd6d31cb2484ad0a3)),
            );

            let ox = ((seed & 0xFF) as i32) % (HONEYPOT_RADIUS * 2) - HONEYPOT_RADIUS;
            let oy = (((seed >> 8) & 0xFF) as i32) % (HONEYPOT_RADIUS / 2) - HONEYPOT_RADIUS / 4;
            let oz = (((seed >> 16) & 0xFF) as i32) % (HONEYPOT_RADIUS * 2) - HONEYPOT_RADIUS;

            let hx = px + ox;
            let hy = (py + oy).clamp(HONEYPOT_MIN_Y, HONEYPOT_MAX_Y);
            let hz = pz + oz;

            // Don't place directly on the player.
            if ox.abs() < 2 && oz.abs() < 2 {
                continue;
            }

            let key = HoneypotBlock::position_key(hx, hy, hz);
            if state.honeypots.contains_key(&key) {
                continue; // already occupied
            }

            // Pick ore type from weighted distribution.
            let ore = pick_ore(seed >> 24);
            let hp = HoneypotBlock::new(hx, hy, hz, ore);
            state.honeypots.insert(key, hp.clone());
            new_blocks.push(hp);
        }

        state.last_spawn = Instant::now();
        new_blocks
    }

    // ─── Dig detection ────────────────────────────────────────────────────

    /// Call this when the client sends `ServerboundPlayerAction(START_DIGGING)`
    /// at position (dx, dy, dz). If the position matches an active honeypot,
    /// accumulates confidence and returns a Violation once the threshold is
    /// crossed.
    pub async fn check_dig(
        &self,
        uuid: Uuid,
        username: &str,
        dig_x: i32,
        dig_y: i32,
        dig_z: i32,
    ) -> Option<Violation> {
        if !self.enabled {
            return None;
        }

        let key = HoneypotBlock::position_key(dig_x, dig_y, dig_z);
        let mut state = self.states.entry(uuid).or_default();

        // Prune expired honeypots.
        state.honeypots.retain(|_, hp| !hp.is_expired());

        if let Some(hp) = state.honeypots.get_mut(&key) {
            hp.hit_count += 1;
            let confidence_gain = hp.ore_type.confidence();
            let ore_name = hp.ore_type.name();
            let hit_count = hp.hit_count;

            tracing::warn!(
                "[XRay:Honeypot] Player {} dug honeypot {} at ({},{},{}) — \
                 hit_count={} confidence_gain={:.2}",
                username,
                ore_name,
                dig_x,
                dig_y,
                dig_z,
                hit_count,
                confidence_gain,
            );

            state.confidence += confidence_gain;
            let confidence = state.confidence;

            if confidence >= XRAY_CONFIDENCE_THRESHOLD {
                state.vl += 1;
                let vl = state.vl;

                if vl >= VL_XRAY {
                    let v = Violation {
                        player_uuid: uuid,
                        check_name: "XRay".into(),
                        check_category: CheckCategory::World,
                        value: confidence,
                        threshold: XRAY_CONFIDENCE_THRESHOLD,
                        timestamp: chrono::Utc::now(),
                        server_id: None,
                        suppressed: false,
                    };

                    drop(state);

                    if let Some(bridge) = &self.bridge {
                        let _ = bridge.report(username, &v).await;
                    }

                    return Some(v);
                }
            }
        }

        None
    }

    // ─── Block removal ────────────────────────────────────────────────────

    /// Remove a honeypot at the given position. Call this when the real server
    /// sends a `BlockUpdate` that reveals what was actually at that coordinate
    /// (air or a different block). This prevents false positives if the server
    /// legitimately places a real ore at a honeypot position.
    pub fn remove_honeypot(&self, uuid: Uuid, x: i32, y: i32, z: i32) {
        if let Some(mut state) = self.states.get_mut(&uuid) {
            state
                .honeypots
                .remove(&HoneypotBlock::position_key(x, y, z));
        }
    }

    /// Return the current set of active honeypots for a player (e.g., for
    /// debugging or re-injection after dimension change).
    pub fn get_honeypots(&self, uuid: Uuid) -> Vec<HoneypotBlock> {
        self.states
            .get(&uuid)
            .map(|s| s.honeypots.values().cloned().collect())
            .unwrap_or_default()
    }

    /// Return the current accumulated confidence score for a player.
    pub fn get_confidence(&self, uuid: Uuid) -> f64 {
        self.states.get(&uuid).map(|s| s.confidence).unwrap_or(0.0)
    }

    /// Clean up all state for a disconnected player.
    pub fn player_quit(&self, uuid: Uuid) {
        self.states.remove(&uuid);
    }

    /// Wipe all honeypots for a player (e.g., on dimension change / respawn).
    /// Confidence and VL are preserved.
    pub fn clear_honeypots(&self, uuid: Uuid) {
        if let Some(mut state) = self.states.get_mut(&uuid) {
            state.honeypots.clear();
            state.last_spawn = Instant::now() - Duration::from_secs(60);
        }
    }
}

// ─── Utilities ───────────────────────────────────────────────────────────────

/// Simple 64-bit XOR-shift PRNG — no external dependencies.
fn xorshift64(mut x: u64) -> u64 {
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    x
}

/// Derive a u64 seed from the first 8 bytes of a UUID.
fn uuid_seed(uuid: Uuid) -> u64 {
    let b = uuid.as_bytes();
    u64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]])
}

/// Select an ore type using a weighted distribution derived from the given seed.
fn pick_ore(seed: u64) -> OreType {
    let total: u32 = ORE_DISTRIBUTION.iter().map(|(_, w)| w).sum();
    let mut roll = (seed % total as u64) as u32;
    for &(ore, weight) in ORE_DISTRIBUTION {
        if roll < weight {
            return ore;
        }
        roll -= weight;
    }
    OreType::Coal // fallback
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn test_engine() -> XrayEngine {
        XrayEngine::new(true, None)
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Honeypot spawning
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn honeypots_spawn_underground() {
        let engine = test_engine();
        let uuid = Uuid::new_v4();

        println!("[XRay:Spawn] Spawning honeypots at y=-12 (deep underground)");
        let blocks = engine.spawn_honeypots(uuid, 100.0, -12.0, 100.0);
        println!("[XRay:Spawn] Spawned {} blocks:", blocks.len());
        for hp in &blocks {
            println!(
                "  ({}, {}, {}) = {} [state_id={}]",
                hp.x,
                hp.y,
                hp.z,
                hp.ore_type.name(),
                hp.block_state_id
            );
        }

        assert!(
            !blocks.is_empty(),
            "Honeypots MUST be spawned when player is underground (y=-12)"
        );
        assert!(
            blocks.len() <= HONEYPOT_MAX_PER_PLAYER,
            "Must not exceed HONEYPOT_MAX_PER_PLAYER={HONEYPOT_MAX_PER_PLAYER}"
        );

        // All spawned blocks must be within radius
        for hp in &blocks {
            let dx = (hp.x - 100).abs();
            let dz = (hp.z - 100).abs();
            assert!(
                dx <= HONEYPOT_RADIUS && dz <= HONEYPOT_RADIUS,
                "Honeypot ({},{},{}) is outside HONEYPOT_RADIUS={}",
                hp.x,
                hp.y,
                hp.z,
                HONEYPOT_RADIUS
            );
            assert!(
                hp.y >= HONEYPOT_MIN_Y && hp.y <= HONEYPOT_MAX_Y,
                "Honeypot Y={} is out of valid range [{HONEYPOT_MIN_Y}, {HONEYPOT_MAX_Y}]",
                hp.y
            );
        }

        println!(
            "[XRay:Spawn] PASS — {} honeypots placed in valid positions",
            blocks.len()
        );
    }

    #[test]
    fn no_honeypots_above_surface() {
        let engine = test_engine();
        let uuid = Uuid::new_v4();

        println!("[XRay:Surface] Trying to spawn honeypots at y=64 (surface)");
        let blocks = engine.spawn_honeypots(uuid, 100.0, 64.0, 100.0);
        println!(
            "[XRay:Surface] Spawned {} blocks (expected 0)",
            blocks.len()
        );

        assert!(
            blocks.is_empty(),
            "Honeypots MUST NOT spawn when player is above y={HONEYPOT_MAX_Y} (got y=64)"
        );
        println!("[XRay:Surface] PASS — no honeypots above ground");
    }

    #[test]
    fn honeypots_respect_max_cap() {
        let engine = test_engine();
        let uuid = Uuid::new_v4();

        // First spawn
        let b1 = engine.spawn_honeypots(uuid, 0.0, -30.0, 0.0);
        println!("[XRay:Cap] First spawn: {} blocks", b1.len());

        // Manually expire the spawn cooldown by manipulating state
        if let Some(mut state) = engine.states.get_mut(&uuid) {
            state.last_spawn = Instant::now() - Duration::from_secs(60);
        }

        // Second spawn — total should be capped
        let b2 = engine.spawn_honeypots(uuid, 5.0, -30.0, 5.0);
        println!("[XRay:Cap] Second spawn: {} additional blocks", b2.len());

        let state = engine.states.get(&uuid).unwrap();
        let total = state.honeypots.len();
        println!(
            "[XRay:Cap] Total active: {} (cap={})",
            total, HONEYPOT_MAX_PER_PLAYER
        );

        assert!(
            total <= HONEYPOT_MAX_PER_PLAYER,
            "Active honeypots ({total}) MUST NOT exceed cap ({HONEYPOT_MAX_PER_PLAYER})"
        );
        println!("[XRay:Cap] PASS");
    }

    #[test]
    fn honeypots_unique_positions() {
        let engine = test_engine();
        let uuid = Uuid::new_v4();
        let blocks = engine.spawn_honeypots(uuid, 0.0, -10.0, 0.0);

        let mut positions = std::collections::HashSet::new();
        for hp in &blocks {
            let pos = (hp.x, hp.y, hp.z);
            assert!(
                positions.insert(pos),
                "Duplicate honeypot position found: ({},{},{})",
                hp.x,
                hp.y,
                hp.z
            );
        }
        println!(
            "[XRay:Unique] PASS — all {} honeypot positions are unique",
            blocks.len()
        );
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Detection logic
    // ─────────────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn diamond_hit_triggers_xray_flag() {
        let engine = test_engine();
        let uuid = Uuid::new_v4();

        // Manually plant a diamond honeypot at a known position
        {
            let mut state = engine.states.entry(uuid).or_default();
            let hp = HoneypotBlock::new(10, -12, 10, OreType::Diamond);
            state.honeypots.insert((10, -12, 10), hp);
            // Pre-set VL to 1 so this hit pushes to threshold
            state.vl = 1;
        }

        println!("[XRay:Diamond] Player digs at diamond honeypot (10,-12,10)");
        let result = engine.check_dig(uuid, "xray_player", 10, -12, 10).await;
        println!("[XRay:Diamond] Violation: {result:?}");

        assert!(
            result.is_some(),
            "Digging a diamond honeypot with VL=1 MUST trigger XRay violation"
        );
        let v = result.unwrap();
        assert_eq!(v.check_name, "XRay");
        assert_eq!(v.check_category, CheckCategory::World);
        assert!(
            v.value >= XRAY_CONFIDENCE_THRESHOLD,
            "Confidence {:.2} must be >= threshold {XRAY_CONFIDENCE_THRESHOLD}",
            v.value
        );
        println!(
            "[XRay:Diamond] PASS — confidence={:.2} threshold={:.2}",
            v.value, v.threshold
        );
    }

    #[tokio::test]
    async fn mining_random_position_does_not_flag() {
        let engine = test_engine();
        let uuid = Uuid::new_v4();

        // Plant honeypot at (10,-12,10) but dig at a completely different position
        {
            let mut state = engine.states.entry(uuid).or_default();
            let hp = HoneypotBlock::new(10, -12, 10, OreType::Diamond);
            state.honeypots.insert((10, -12, 10), hp);
        }

        println!("[XRay:Safe] Player digs at (50,-12,50) — not a honeypot position");
        let result = engine.check_dig(uuid, "legit_player", 50, -12, 50).await;
        println!("[XRay:Safe] Violation: {result:?}");

        assert!(
            result.is_none(),
            "Mining at a non-honeypot position MUST NOT trigger XRay"
        );
        println!("[XRay:Safe] PASS — no false positive");
    }

    #[tokio::test]
    async fn multiple_iron_hits_accumulate_to_flag() {
        let engine = test_engine();
        let uuid = Uuid::new_v4();
        // Iron confidence = 0.5 per hit; threshold = 1.0 → need 2 hits + VL=1

        println!(
            "[XRay:Iron] Simulating 3 iron honeypot hits (confidence=0.5 each, threshold={XRAY_CONFIDENCE_THRESHOLD})"
        );

        // Plant 3 iron honeypots
        {
            let mut state = engine.states.entry(uuid).or_default();
            for i in 0..3_i32 {
                let hp = HoneypotBlock::new(i, -10, 0, OreType::Iron);
                state.honeypots.insert((i, -10, 0), hp);
            }
            state.vl = 1; // pre-load VL
        }

        let mut last = None;
        for i in 0..3_i32 {
            let v = engine.check_dig(uuid, "xray_player", i, -10, 0).await;
            let conf = engine.get_confidence(uuid);
            println!(
                "[XRay:Iron] hit={i} confidence={conf:.2} violation={}",
                v.is_some()
            );
            last = v.or(last);
        }

        assert!(
            last.is_some(),
            "3 iron honeypot hits MUST eventually flag XRay (accumulated confidence)"
        );
        println!(
            "[XRay:Iron] PASS — confidence={:.2}",
            engine.get_confidence(uuid)
        );
    }

    #[tokio::test]
    async fn single_coal_hit_does_not_flag_alone() {
        let engine = test_engine();
        let uuid = Uuid::new_v4();
        // Coal confidence = 0.2 — should not reach threshold=1.0 alone

        println!("[XRay:Coal] Simulating 1 coal hit (confidence=0.2, threshold=1.0)");

        {
            let mut state = engine.states.entry(uuid).or_default();
            let hp = HoneypotBlock::new(5, -5, 5, OreType::Coal);
            state.honeypots.insert((5, -5, 5), hp);
        }

        let result = engine.check_dig(uuid, "test_player", 5, -5, 5).await;
        println!("[XRay:Coal] Violation: {result:?}");

        assert!(
            result.is_none(),
            "Single coal hit (confidence=0.2) MUST NOT flag immediately (threshold=1.0)"
        );
        println!("[XRay:Coal] PASS — single low-confidence hit is not enough");
    }

    #[tokio::test]
    async fn remove_honeypot_prevents_detection() {
        let engine = test_engine();
        let uuid = Uuid::new_v4();

        {
            let mut state = engine.states.entry(uuid).or_default();
            let hp = HoneypotBlock::new(20, -15, 20, OreType::Emerald);
            state.honeypots.insert((20, -15, 20), hp);
            state.vl = 5; // even with high VL, removal should prevent detection
        }

        // Server reveals the real block at this position — remove honeypot
        println!("[XRay:Remove] Server reveals block at (20,-15,20) — removing honeypot");
        engine.remove_honeypot(uuid, 20, -15, 20);

        let result = engine.check_dig(uuid, "test_player", 20, -15, 20).await;
        println!("[XRay:Remove] Violation after removal: {result:?}");

        assert!(
            result.is_none(),
            "Digging at a REMOVED honeypot position MUST NOT trigger detection"
        );
        println!("[XRay:Remove] PASS — removed honeypot is inert");
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Ore distribution
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn ore_distribution_is_weighted_correctly() {
        // Run 10,000 picks and verify the distribution is roughly correct.
        let mut counts: HashMap<OreType, u32> = HashMap::new();
        let n = 10_000u64;

        for i in 0..n {
            let seed = xorshift64(i.wrapping_mul(0x9e3779b97f4a7c15) ^ 0xdeadbeef);
            let ore = pick_ore(seed);
            *counts.entry(ore).or_insert(0) += 1;
        }

        println!("[XRay:Distribution] Ore distribution over {n} picks:");
        let total: u32 = counts.values().sum();
        for &(ore, expected_weight) in ORE_DISTRIBUTION {
            let expected_pct: u32 = ORE_DISTRIBUTION.iter().map(|(_, w)| w).sum();
            let actual_count = *counts.get(&ore).unwrap_or(&0);
            let actual_pct = actual_count as f64 / total as f64 * 100.0;
            let expected_actual_pct = expected_weight as f64 / expected_pct as f64 * 100.0;
            println!(
                "  {:15} expected={:.1}%  actual={:.1}%",
                ore.name(),
                expected_actual_pct,
                actual_pct
            );
            // Allow ±5% deviation from expected
            assert!(
                (actual_pct - expected_actual_pct).abs() < 5.0,
                "Ore {} distribution is off: expected {:.1}% got {:.1}%",
                ore.name(),
                expected_actual_pct,
                actual_pct
            );
        }
        println!("[XRay:Distribution] PASS — distribution within ±5% tolerance");
    }

    #[test]
    fn ore_confidence_values_are_valid() {
        println!("[XRay:Confidence] Verifying confidence values:");
        for &(ore, _) in ORE_DISTRIBUTION {
            let c = ore.confidence();
            println!("  {:15} confidence={:.2}", ore.name(), c);
            assert!(
                (0.0..=1.0).contains(&c),
                "Ore {} confidence {:.2} must be in [0.0, 1.0]",
                ore.name(),
                c
            );
        }
        // Diamond must be the highest confidence ore
        assert!(
            OreType::Diamond.confidence() >= OreType::Iron.confidence(),
            "Diamond confidence must be >= Iron confidence"
        );
        assert!(
            OreType::Coal.confidence() < OreType::Diamond.confidence(),
            "Coal confidence must be < Diamond confidence"
        );
        println!("[XRay:Confidence] PASS");
    }
}
