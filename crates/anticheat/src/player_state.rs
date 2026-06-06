use std::collections::{HashMap, VecDeque};
use std::time::Instant;

/// Maximum number of past positions kept for movement analysis.
pub const POSITION_HISTORY_LEN: usize = 20;
/// Maximum number of recent block-place timestamps kept.
pub const BLOCK_PLACE_HISTORY_LEN: usize = 16;
/// Maximum number of recent packet timestamps kept for timer analysis.
pub const PACKET_TIME_HISTORY_LEN: usize = 40;

#[derive(Debug, Clone)]
pub struct PlayerAnticheatState {
    // ─── Position & Movement ────────────────────────────────────────────────
    pub last_x: f64,
    pub last_y: f64,
    pub last_z: f64,
    /// Ring-buffer of (x, y, z, on_ground) for the last N ticks.
    pub position_history: VecDeque<(f64, f64, f64, bool)>,
    /// Computed horizontal speed (blocks/tick) for the last N ticks.
    pub speed_history: VecDeque<f64>,
    /// Y-velocity (dy per tick) history, used for gravity-curve validation.
    pub dy_history: VecDeque<f64>,
    pub last_move: Instant,
    pub on_ground: bool,
    pub last_ground_time: Instant,
    /// Continuous ticks spent in the air (resets to 0 on landing).
    pub air_ticks: u32,
    /// Ticks since last teleport; exempts physics checks while non-zero.
    pub teleport_exempt_ticks: u32,
    /// Ticks continuously placing blocks while airborne (scaffold heuristic).
    pub scaffold_ticks: u32,
    /// Consecutive ticks where the player jumped exactly at the highest point
    /// of each hop (bhop detection).
    pub bhop_ticks: u32,
    /// Y position at the moment the player left the ground.
    pub jump_start_y: f64,

    // ─── Combat ─────────────────────────────────────────────────────────────
    /// Timestamps of every attack within the last second.
    pub recent_attacks: VecDeque<Instant>,
    /// Last yaw value reported by the client.
    pub last_yaw: f64,
    /// Last pitch value reported by the client.
    pub last_pitch: f64,
    pub last_aim_check: Instant,
    /// GCD accumulator for aimbot analysis (tracks yaw deltas).
    pub yaw_gcd_samples: VecDeque<f64>,
    /// GCD accumulator for aimbot analysis (tracks pitch deltas).
    pub pitch_gcd_samples: VecDeque<f64>,

    // ─── Network / Timer ────────────────────────────────────────────────────
    pub packets_sent_in_second: u32,
    pub last_packet_reset: Instant,
    /// Timestamps of recent packets for fine-grained timer analysis.
    pub packet_times: VecDeque<Instant>,

    // ─── Inventory ───────────────────────────────────────────────────────────
    /// Timestamps of recent inventory click events.
    pub inventory_click_times: VecDeque<Instant>,
    /// Timestamp of the last hotbar slot change.
    pub last_slot_change: Option<Instant>,
    /// How many slot changes occurred in the last analysis window.
    pub slot_change_count: u32,

    // ─── Block Interaction ───────────────────────────────────────────────────
    /// Timestamps of recent block-place events.
    pub block_place_times: VecDeque<Instant>,

    // ─── Mod / Effect Context ────────────────────────────────────────────────
    pub active_effects: Vec<StatusEffect>,
    pub detected_mods: Vec<String>,
    pub is_modded_client: bool,
    pub trusted_mods: Vec<String>,

    // ─── Violation Levels ────────────────────────────────────────────────────
    /// Per-check violation level (VL) counters.
    pub check_violations: HashMap<String, u32>,

    // ─── Session Info ────────────────────────────────────────────────────────
    /// Client-reported ping in milliseconds (used for latency compensation).
    pub ping_ms: u32,
    /// Protocol version of the connecting client.
    pub protocol_version: u32,
}

impl Default for PlayerAnticheatState {
    fn default() -> Self {
        let now = Instant::now();
        Self {
            last_x: 0.0,
            last_y: 0.0,
            last_z: 0.0,
            position_history: VecDeque::with_capacity(POSITION_HISTORY_LEN),
            speed_history: VecDeque::with_capacity(POSITION_HISTORY_LEN),
            dy_history: VecDeque::with_capacity(POSITION_HISTORY_LEN),
            last_move: now,
            on_ground: true,
            last_ground_time: now,
            air_ticks: 0,
            teleport_exempt_ticks: 0,
            scaffold_ticks: 0,
            bhop_ticks: 0,
            jump_start_y: 0.0,

            recent_attacks: VecDeque::new(),
            last_yaw: 0.0,
            last_pitch: 0.0,
            last_aim_check: now,
            yaw_gcd_samples: VecDeque::with_capacity(20),
            pitch_gcd_samples: VecDeque::with_capacity(20),

            packets_sent_in_second: 0,
            last_packet_reset: now,
            packet_times: VecDeque::with_capacity(PACKET_TIME_HISTORY_LEN),

            inventory_click_times: VecDeque::new(),
            last_slot_change: None,
            slot_change_count: 0,

            block_place_times: VecDeque::with_capacity(BLOCK_PLACE_HISTORY_LEN),

            active_effects: Vec::new(),
            detected_mods: Vec::new(),
            is_modded_client: false,
            trusted_mods: Vec::new(),
            check_violations: HashMap::new(),

            ping_ms: 0,
            protocol_version: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct StatusEffect {
    pub effect_id: u8,
    pub amplifier: u8,
    /// Remaining duration in ticks (0 = unknown/infinite).
    pub remaining_ticks: u32,
}

pub const TRUSTED_MODS: &[&str] = &[
    "fabric",
    "forge",
    "neoforge",
    "liteloader",
    "quilt",
    "optifine",
    "sodium",
    "iris",
    "lithium",
    "phosphor",
    "rubidium",
    "entityradar",
    "journeymap",
    "xaero",
    "minihud",
    "replaymod",
    "shaders",
    "modmenu",
    "roughlyenoughitems",
    "jei",
];
