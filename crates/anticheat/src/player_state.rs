use std::collections::{HashMap, VecDeque};
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct PlayerAnticheatState {
    pub last_x: f64,
    pub last_y: f64,
    pub last_z: f64,
    pub last_move: Instant,
    pub recent_attacks: VecDeque<Instant>,
    pub active_effects: Vec<StatusEffect>,
    pub on_ground: bool,
    pub detected_mods: Vec<String>,
    pub is_modded_client: bool,
    pub trusted_mods: Vec<String>,
    pub check_violations: HashMap<String, u32>,
    pub last_ground_time: Instant,
    pub air_ticks: u32,
    pub packets_sent_in_second: u32,
    pub last_packet_reset: Instant,
    pub last_yaw: f64,
    pub last_pitch: f64,
    pub last_aim_check: Instant,
    pub scaffold_ticks: u32,
}

impl Default for PlayerAnticheatState {
    fn default() -> Self {
        Self {
            last_x: 0.0,
            last_y: 0.0,
            last_z: 0.0,
            last_move: Instant::now(),
            recent_attacks: VecDeque::new(),
            active_effects: Vec::new(),
            on_ground: true,
            detected_mods: Vec::new(),
            is_modded_client: false,
            trusted_mods: Vec::new(),
            check_violations: HashMap::new(),
            last_ground_time: Instant::now(),
            air_ticks: 0,
            packets_sent_in_second: 0,
            last_packet_reset: Instant::now(),
            last_yaw: 0.0,
            last_pitch: 0.0,
            last_aim_check: Instant::now(),
            scaffold_ticks: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct StatusEffect {
    pub effect_id: u8,
    pub amplifier: u8,
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
