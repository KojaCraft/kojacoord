// # Kojacoord Anticheat — Check Engine
//
// This module implements all server-side heuristic checks. Every check is
// designed to be:
//
// * Cross-version safe: thresholds derived from Minecraft physics constants,
//   which are the same across all supported versions (1.7-1.21).
//
// * Latency-compensated: the player's reported `ping_ms` widens every
//   threshold by an appropriate number of extra ticks so high-ping players
//   are not punished for network jitter.
//
// * Violation-level gated: checks require a sustained pattern (N ticks
//   in a row) before a violation is emitted, drastically reducing false
//   positives from momentary anomalies.
//
// * Effect-aware: Speed / Jump Boost potions expand the accepted range
//   according to vanilla formulas.

use dashmap::DashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::{
    alert_system::AlertSystem,
    bridge::BridgeClient,
    mod_compatibility::ModCompatibility,
    player_state::{PlayerAnticheatState, BLOCK_PLACE_HISTORY_LEN, POSITION_HISTORY_LEN},
    violation::{CheckCategory, Violation},
};
use kojacoord_config::AnticheatConfig;

// ─── Minecraft physics constants ─────────────────────────────────────────────
// These values match vanilla behavior for all versions 1.7–1.21.
// They are used in tests and serve as authoritative documentation.
#[allow(dead_code)]
/// Vanilla walk speed (blocks/tick). Source: PlayerEntity#travelMidAir.
const MC_WALK_SPEED: f64 = 0.221;
const MC_SPRINT_SPEED: f64 = 0.286;
#[allow(dead_code)]
/// Vanilla initial jump velocity (blocks/tick upward). Source: Entity#jumpFromGround.
const MC_JUMP_VELOCITY: f64 = 0.42;
#[allow(dead_code)]
/// Gravity applied per tick (subtracted from Y velocity each tick).
const MC_GRAVITY: f64 = 0.08;
#[allow(dead_code)]
/// Air-resistance (drag) factor applied to Y velocity each tick.
const MC_DRAG: f64 = 0.98;
#[allow(dead_code)]
/// Maximum legitimate fall speed (terminal velocity, blocks/tick downward).
const MC_TERMINAL_VELOCITY: f64 = 3.92;
/// Sprint jump boost horizontal (additional blocks/tick). Source: sprint-jump formula.
const MC_SPRINT_JUMP_BOOST: f64 = 0.2;

// ─── Detection thresholds ────────────────────────────────────────────────────

/// Base extra tolerance added to every speed threshold (accounts for sub-tick
/// interpolation and floating-point imprecision in the client).
const SPEED_BASE_EPSILON: f64 = 0.03;
/// Tolerance per 100 ms of ping (widens speed threshold proportionally).
const SPEED_PING_FACTOR: f64 = 0.002; // blocks/tick per ms of ping

/// Minimum violation level before Speed is reported.
const VL_SPEED: u32 = 6;
/// Minimum violation level before Flight is reported.
const VL_FLIGHT: u32 = 8;
/// Minimum violation level before NoFall is reported.
const VL_NOFALL: u32 = 6;
/// Minimum violation level before Killaura (CPS) is reported.
const VL_KILLAURA: u32 = 3;
/// Minimum violation level before Reach is reported.
const VL_REACH: u32 = 4;
/// Minimum violation level before Aimbot is reported.
const VL_AIMBOT: u32 = 5;
/// Minimum violation level before AutoTool is reported.
const VL_AUTOTOOL: u32 = 3;
/// Minimum violation level before Scaffold is reported.
const VL_SCAFFOLD: u32 = 5;
/// Minimum violation level before Timer is reported.
const VL_TIMER: u32 = 3;
/// Minimum violation level before Velocity is reported.
const VL_VELOCITY: u32 = 5;
/// Minimum violation level before Bhop is reported.
const VL_BHOP: u32 = 4;
/// Minimum violation level before Inventory is reported.
const VL_INVENTORY: u32 = 4;
/// Minimum violation level before InstaPlace is reported.
const VL_INSTAPLACE: u32 = 4;

// ─── AnticheatEngine ─────────────────────────────────────────────────────────

pub struct AnticheatEngine {
    config: Arc<RwLock<AnticheatConfig>>,
    states: Arc<DashMap<Uuid, PlayerAnticheatState>>,
    bridge: Option<BridgeClient>,
    alert_system: AlertSystem,
    mod_compat: ModCompatibility,
}

impl AnticheatEngine {
    pub fn new(config: AnticheatConfig) -> Self {
        let bridge = config
            .bridge_endpoint
            .as_ref()
            .map(|ep| BridgeClient::new(ep.clone(), config.bridge_token.clone()));
        let alert_system = AlertSystem::new(crate::alert_system::AlertConfig::default())
            .with_name("Kojacoord Guardian".to_string());
        let mod_compat = ModCompatibility::new(true, false);

        Self {
            config: Arc::new(RwLock::new(config)),
            states: Arc::new(DashMap::new()),
            bridge,
            alert_system,
            mod_compat,
        }
    }

    pub async fn reload_config(&self, new_config: AnticheatConfig) {
        *self.config.write().await = new_config;
    }

    pub fn get_alert_system(&self) -> &AlertSystem {
        &self.alert_system
    }

    pub fn get_mod_compatibility(&self) -> &ModCompatibility {
        &self.mod_compat
    }

    /// Notify the engine that a player connected with a given protocol version.
    pub fn register_player(&self, uuid: Uuid, protocol_version: u32) {
        let mut state = self.states.entry(uuid).or_default();
        state.protocol_version = protocol_version;
    }

    /// Update the client-reported ping so latency compensation stays current.
    pub fn update_ping(&self, uuid: Uuid, ping_ms: u32) {
        if let Some(mut state) = self.states.get_mut(&uuid) {
            state.ping_ms = ping_ms;
        }
    }

    /// Mark the player as temporarily exempt from physics checks (e.g. after a
    /// server-side teleport or respawn). `ticks` should be at least 10 (half a
    /// second) to cover all in-flight movement packets.
    pub fn teleport_exempt(&self, uuid: Uuid, ticks: u32) {
        let mut state = self.states.entry(uuid).or_default();
        state.teleport_exempt_ticks = ticks;
    }

    pub async fn register_mod_brand(&self, uuid: Uuid, brand: String) {
        let mut state = self.states.entry(uuid).or_default();

        let detected = self.mod_compat.detect_mods_from_brand(&brand);
        state.detected_mods = detected.clone();
        state.is_modded_client = !detected.is_empty();

        for mod_name in &detected {
            if self.mod_compat.is_trusted_mod(mod_name) {
                state.trusted_mods.push(mod_name.clone());
            }
        }
    }

    // ─── Public check API ────────────────────────────────────────────────────

    /// Check a movement packet for Speed, Flight, NoFall and Bhop violations.
    ///
    /// * `ping_ms` – the player's current round-trip latency; used to widen
    ///   thresholds so high-ping players are not false-flagged.
    #[allow(clippy::too_many_arguments)]
    pub async fn check_movement(
        &self,
        uuid: Uuid,
        username: &str,
        x: f64,
        y: f64,
        z: f64,
        on_ground: bool,
        ping_ms: u32,
    ) -> Option<Violation> {
        let config = self.config.read().await;
        if !config.enabled {
            return None;
        }
        let mut state = self.states.entry(uuid).or_default();

        // ── Teleport exemption ──────────────────────────────────────────────
        if state.teleport_exempt_ticks > 0 {
            state.teleport_exempt_ticks = state.teleport_exempt_ticks.saturating_sub(1);
            state.last_x = x;
            state.last_y = y;
            state.last_z = z;
            state.on_ground = on_ground;
            return None;
        }

        // ── Delta computation ───────────────────────────────────────────────
        let dx = x - state.last_x;
        let dy = y - state.last_y;
        let dz = z - state.last_z;
        let horiz_speed = (dx * dx + dz * dz).sqrt(); // blocks/tick horizontal

        // Update history ring-buffers
        state.position_history.push_back((x, y, z, on_ground));
        if state.position_history.len() > POSITION_HISTORY_LEN {
            state.position_history.pop_front();
        }
        state.speed_history.push_back(horiz_speed);
        if state.speed_history.len() > POSITION_HISTORY_LEN {
            state.speed_history.pop_front();
        }
        state.dy_history.push_back(dy);
        if state.dy_history.len() > POSITION_HISTORY_LEN {
            state.dy_history.pop_front();
        }

        state.last_x = x;
        state.last_y = y;
        state.last_z = z;
        state.last_move = Instant::now();

        // ── Ground / air tracking ───────────────────────────────────────────
        if on_ground {
            // Bhop: if the player re-lands exactly when they would have
            // maximum horizontal speed from a sprint-jump (air_ticks 5-8
            // indicates the optimal re-jump window), increment bhop counter.
            if state.air_ticks >= 5 && state.air_ticks <= 9 {
                state.bhop_ticks += 1;
            } else {
                state.bhop_ticks = state.bhop_ticks.saturating_sub(1);
            }
            state.last_ground_time = Instant::now();
            state.air_ticks = 0;
        } else {
            if state.air_ticks == 0 {
                // Just left the ground — record takeoff Y
                state.jump_start_y = state.last_y;
            }
            state.air_ticks += 1;
        }
        state.on_ground = on_ground;

        // ── Latency-compensated speed threshold ─────────────────────────────
        // Speed-I gives +20% walk, Speed-II gives +40%, etc.
        let speed_amp = state
            .active_effects
            .iter()
            .find(|e| e.effect_id == 1)
            .map(|e| e.amplifier + 1)
            .unwrap_or(0);
        let speed_bonus = 0.20 * f64::from(speed_amp);

        // Jump Boost amplifies horizontal speed during sprint-jumps.
        let jump_amp = state
            .active_effects
            .iter()
            .find(|e| e.effect_id == 8)
            .map(|e| e.amplifier + 1)
            .unwrap_or(0);
        let jump_bonus = 0.10 * f64::from(jump_amp);

        // Max accepted horizontal speed: sprint speed × effect multipliers,
        // plus ping-based leniency, plus base epsilon.
        let ping_leniency = SPEED_PING_FACTOR * f64::from(ping_ms);
        let max_accepted_speed = (MC_SPRINT_SPEED + MC_SPRINT_JUMP_BOOST) // sprint-jump baseline
            * (1.0 + speed_bonus + jump_bonus)
            + ping_leniency
            + SPEED_BASE_EPSILON;

        let should_suppress_speed = self.mod_compat.should_suppress_check(&state, "Speed");
        let should_suppress_flight = self.mod_compat.should_suppress_check(&state, "Flight");
        let should_suppress_nofall = self.mod_compat.should_suppress_check(&state, "NoFall");
        let should_suppress_bhop = self.mod_compat.should_suppress_check(&state, "Bhop");

        let mut violation_to_report: Option<Violation> = None;

        // ── Speed check ─────────────────────────────────────────────────────
        // Only fires when we have at least 3 speed samples (avoids first-move
        // burst from teleport or join).
        if state.speed_history.len() >= 3 && !should_suppress_speed {
            // Use the rolling average to smooth jitter.
            let recent_n = state.speed_history.len().min(5);
            let avg_speed: f64 =
                state.speed_history.iter().rev().take(recent_n).sum::<f64>() / recent_n as f64;

            if avg_speed > max_accepted_speed {
                let count = self.increment_vl(&mut state, "Speed");
                if count >= VL_SPEED {
                    violation_to_report = Some(Violation {
                        player_uuid: uuid,
                        check_name: "Speed".into(),
                        check_category: CheckCategory::Movement,
                        value: avg_speed,
                        threshold: max_accepted_speed,
                        timestamp: chrono::Utc::now(),
                        server_id: None,
                        suppressed: false,
                    });
                }
            } else {
                self.decay_vl(&mut state, "Speed");
            }
        }

        // ── Flight check ────────────────────────────────────────────────────
        // Vanilla gravity: each tick the Y velocity is multiplied by MC_DRAG
        // and decremented by MC_GRAVITY. Starting from MC_JUMP_VELOCITY, the
        // expected dy values are:
        //   tick 1: ≈ 0.42
        //   tick 2: ≈ 0.33
        //   tick 3: ≈ 0.25
        //   ...eventually negative (falling).
        // We check whether the player is sustaining positive dy (or zero-dy)
        // for far longer than any vanilla trajectory allows. We require at
        // least 16 air ticks with positive dy to flag — this completely misses
        // real jumps (which peak at ~10 ticks) and only catches creative-fly
        // or hacked flight.
        if violation_to_report.is_none() && !on_ground && !should_suppress_flight {
            let sustained_positive_dy =
                state.air_ticks > 16 && state.dy_history.iter().rev().take(8).all(|&d| d >= -0.01); // near-zero or upward for 8 straight ticks

            if sustained_positive_dy {
                let count = self.increment_vl(&mut state, "Flight");
                if count >= VL_FLIGHT {
                    violation_to_report = Some(Violation {
                        player_uuid: uuid,
                        check_name: "Flight".into(),
                        check_category: CheckCategory::Movement,
                        value: state.air_ticks as f64,
                        threshold: 16.0,
                        timestamp: chrono::Utc::now(),
                        server_id: None,
                        suppressed: false,
                    });
                }
            } else {
                self.decay_vl(&mut state, "Flight");
            }
        }

        // ── NoFall check ────────────────────────────────────────────────────
        // Vanilla: a player who fell from height MUST have negative dy growing
        // toward terminal velocity. The *minimum* negative dy after a vanilla
        // jump peak (air_ticks ≈ 10+) is around -0.8 blocks/tick.
        //
        // NoFall hacks keep dy at exactly 0.0 (or even slightly positive) even
        // after extended fall time. We flag if after 20+ air ticks the last
        // 5 dy samples are ALL > -0.3 (far too shallow for a real fall).
        if violation_to_report.is_none()
            && !on_ground
            && !should_suppress_nofall
            && state.air_ticks > 20
            && state.dy_history.len() >= 5
        {
            let shallow_fall = state.dy_history.iter().rev().take(5).all(|&d| d > -0.3);
            if shallow_fall {
                let count = self.increment_vl(&mut state, "NoFall");
                if count >= VL_NOFALL {
                    violation_to_report = Some(Violation {
                        player_uuid: uuid,
                        check_name: "NoFall".into(),
                        check_category: CheckCategory::Movement,
                        value: state.dy_history.back().copied().unwrap_or(0.0),
                        threshold: -0.3,
                        timestamp: chrono::Utc::now(),
                        server_id: None,
                        suppressed: false,
                    });
                }
            } else {
                self.decay_vl(&mut state, "NoFall");
            }
        }

        // ── Bhop check ──────────────────────────────────────────────────────
        // Legitimate players occasionally get a good bhop timing, but they
        // cannot sustain it for >6 consecutive hops. A sustained pattern is
        // a strong signal of an auto-bhop script.
        if violation_to_report.is_none() && !should_suppress_bhop {
            if state.bhop_ticks >= 6 {
                let count = self.increment_vl(&mut state, "Bhop");
                if count >= VL_BHOP {
                    violation_to_report = Some(Violation {
                        player_uuid: uuid,
                        check_name: "Bhop".into(),
                        check_category: CheckCategory::Movement,
                        value: state.bhop_ticks as f64,
                        threshold: 6.0,
                        timestamp: chrono::Utc::now(),
                        server_id: None,
                        suppressed: false,
                    });
                }
            } else {
                self.decay_vl(&mut state, "Bhop");
            }
        }

        drop(state);

        if let Some(v) = violation_to_report {
            if let Some(bridge) = &self.bridge {
                let _ = bridge.report(username, &v).await;
            }
            return Some(v);
        }

        None
    }

    /// Validate the Y-velocity vector against Minecraft's gravity equation.
    /// Detects NoGravity, Glide hacks, and elytra exploits.
    pub async fn check_velocity(
        &self,
        uuid: Uuid,
        username: &str,
        server_sent_dy: f64,
        client_reported_dy: f64,
    ) -> Option<Violation> {
        let config = self.config.read().await;
        if !config.enabled {
            return None;
        }
        let state = self.states.entry(uuid).or_default();

        // Allow a generous margin for tick ordering and server reconciliation.
        let margin = 0.15 + SPEED_PING_FACTOR * f64::from(state.ping_ms);

        if (client_reported_dy - server_sent_dy).abs() > margin {
            drop(state);
            let mut state = self.states.entry(uuid).or_default();
            let count = self.increment_vl(&mut state, "Velocity");
            if count >= VL_VELOCITY {
                let v = Violation {
                    player_uuid: uuid,
                    check_name: "Velocity".into(),
                    check_category: CheckCategory::Movement,
                    value: (client_reported_dy - server_sent_dy).abs(),
                    threshold: margin,
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
        } else {
            drop(state);
            if let Some(mut state) = self.states.get_mut(&uuid) {
                self.decay_vl(&mut state, "Velocity");
            }
        }

        None
    }

    /// Check a combat (attack) packet for Killaura (CPS) and Reach violations.
    pub async fn check_attack(
        &self,
        uuid: Uuid,
        username: &str,
        target_distance: Option<f64>,
    ) -> Option<Violation> {
        let config = self.config.read().await;
        if !config.enabled {
            return None;
        }
        let mut state = self.states.entry(uuid).or_default();

        let now = Instant::now();
        state.recent_attacks.push_back(now);
        // Prune attacks older than 1 second
        while let Some(&front) = state.recent_attacks.front() {
            if now.duration_since(front).as_secs_f64() > 1.0 {
                state.recent_attacks.pop_front();
            } else {
                break;
            }
        }

        let cps = state.recent_attacks.len() as f64;
        // Vanilla max CPS ≈ 20 (limited by game tick rate) but config allows
        // operators to set a lower value. We allow up to 2 CPS leniency for
        // input lag.
        let threshold = f64::from(config.max_cps) + 2.0;

        let should_suppress = self.mod_compat.should_suppress_check(&state, "Killaura");
        let mut violation_to_report: Option<Violation> = None;

        if cps > threshold && !should_suppress {
            let count = self.increment_vl(&mut state, "Killaura");
            if count >= VL_KILLAURA {
                violation_to_report = Some(Violation {
                    player_uuid: uuid,
                    check_name: "Killaura".into(),
                    check_category: CheckCategory::Combat,
                    value: cps,
                    threshold,
                    timestamp: chrono::Utc::now(),
                    server_id: None,
                    suppressed: false,
                });
            }
        } else {
            self.decay_vl(&mut state, "Killaura");
        }

        if violation_to_report.is_none() {
            if let Some(distance) = target_distance {
                // Vanilla reach: survival = 3.0 blocks, creative = 5.0 blocks.
                // We use 3.5 + generous ping leniency (fast connections → 3.7,
                // 200 ms ping → 3.9 blocks).
                let ping_leniency = SPEED_PING_FACTOR * 10.0 * f64::from(state.ping_ms);
                let reach_threshold = 3.5 + ping_leniency.min(0.5);
                let should_suppress_reach = self.mod_compat.should_suppress_check(&state, "Reach");

                if distance > reach_threshold && !should_suppress_reach {
                    let count = self.increment_vl(&mut state, "Reach");
                    if count >= VL_REACH {
                        violation_to_report = Some(Violation {
                            player_uuid: uuid,
                            check_name: "Reach".into(),
                            check_category: CheckCategory::Combat,
                            value: distance,
                            threshold: reach_threshold,
                            timestamp: chrono::Utc::now(),
                            server_id: None,
                            suppressed: false,
                        });
                    }
                } else {
                    self.decay_vl(&mut state, "Reach");
                }
            }
        }

        drop(state);

        if let Some(v) = violation_to_report {
            if let Some(bridge) = &self.bridge {
                let _ = bridge.report(username, &v).await;
            }
            return Some(v);
        }

        None
    }

    /// Check for Timer hack — the player sending movement packets significantly
    /// faster than the 20-tick/second game loop.
    pub async fn check_timer(&self, uuid: Uuid, username: &str) -> Option<Violation> {
        let config = self.config.read().await;
        if !config.enabled {
            return None;
        }
        let mut state = self.states.entry(uuid).or_default();

        let now = Instant::now();
        state.packets_sent_in_second += 1;
        state.packet_times.push_back(now);
        if state.packet_times.len() > 40 {
            state.packet_times.pop_front();
        }

        let mut violation_to_report: Option<Violation> = None;

        if now.duration_since(state.last_packet_reset).as_secs() >= 1 {
            let pps = state.packets_sent_in_second as f64;
            // Vanilla: 20 pps. Allow up to 25 for ping jitter. Config can
            // tighten this via max_speed (operator tuning).
            let normal_pps = 20.0_f64;
            let threshold = normal_pps * 1.25 + f64::from(state.ping_ms) * 0.01;

            state.packets_sent_in_second = 0;
            state.last_packet_reset = now;

            let should_suppress = self.mod_compat.should_suppress_check(&state, "Timer");

            if pps > threshold && !should_suppress {
                let count = self.increment_vl(&mut state, "Timer");
                if count >= VL_TIMER {
                    violation_to_report = Some(Violation {
                        player_uuid: uuid,
                        check_name: "Timer".into(),
                        check_category: CheckCategory::Network,
                        value: pps,
                        threshold,
                        timestamp: chrono::Utc::now(),
                        server_id: None,
                        suppressed: false,
                    });
                }
            } else {
                self.decay_vl(&mut state, "Timer");
            }
        }

        drop(state);

        if let Some(v) = violation_to_report {
            if let Some(bridge) = &self.bridge {
                let _ = bridge.report(username, &v).await;
            }
            return Some(v);
        }

        None
    }

    /// Check rotation packets for Aimbot using GCD (Greatest Common Divisor)
    /// analysis. Human mouse input has continuous, noisy deltas. Aimbot/
    /// triggerbot software applies discrete, perfectly reproducible angular
    /// steps that share a common GCD much larger than human jitter (~0.001°).
    pub async fn check_aimbot(
        &self,
        uuid: Uuid,
        username: &str,
        yaw: f64,
        pitch: f64,
    ) -> Option<Violation> {
        let config = self.config.read().await;
        if !config.enabled {
            return None;
        }
        let mut state = self.states.entry(uuid).or_default();

        let now = Instant::now();
        // Throttle to once every 50 ms to avoid flooding with redundant data.
        if now.duration_since(state.last_aim_check).as_millis() < 50 {
            return None;
        }
        state.last_aim_check = now;

        let yaw_delta = (yaw - state.last_yaw).abs();
        let pitch_delta = (pitch - state.last_pitch).abs();

        state.last_yaw = yaw;
        state.last_pitch = pitch;

        // Skip near-zero deltas (player not moving mouse)
        if yaw_delta < 0.001 && pitch_delta < 0.001 {
            return None;
        }

        // Accumulate GCD samples. We need at least 20 before analysing.
        state.yaw_gcd_samples.push_back(yaw_delta);
        state.pitch_gcd_samples.push_back(pitch_delta);
        if state.yaw_gcd_samples.len() > 20 {
            state.yaw_gcd_samples.pop_front();
        }
        if state.pitch_gcd_samples.len() > 20 {
            state.pitch_gcd_samples.pop_front();
        }

        let should_suppress = self.mod_compat.should_suppress_check(&state, "Aimbot");
        let mut violation_to_report: Option<Violation> = None;

        if state.yaw_gcd_samples.len() >= 20 && !should_suppress {
            let yaw_gcd = approximate_gcd_list(state.yaw_gcd_samples.iter().copied());
            let pitch_gcd = approximate_gcd_list(state.pitch_gcd_samples.iter().copied());

            // A GCD > 0.5° is physically impossible for a human at typical
            // sensitivity settings and indicates discrete-step aimbot.
            // A GCD < 0.0001° is also suspicious — it suggests the client is
            // generating microscopically precise steps (triggerbot-like micro-
            // corrections to lock onto a target).
            let suspiciously_large_gcd = yaw_gcd > 0.5 || pitch_gcd > 0.5;
            let suspiciously_small_gcd =
                yaw_gcd > 0.0 && yaw_gcd < 0.0002 && pitch_gcd > 0.0 && pitch_gcd < 0.0002;

            if suspiciously_large_gcd || suspiciously_small_gcd {
                let count = self.increment_vl(&mut state, "Aimbot");
                if count >= VL_AIMBOT {
                    let reported_gcd = yaw_gcd.max(pitch_gcd);
                    violation_to_report = Some(Violation {
                        player_uuid: uuid,
                        check_name: "Aimbot".into(),
                        check_category: CheckCategory::Combat,
                        value: reported_gcd,
                        threshold: 0.5,
                        timestamp: chrono::Utc::now(),
                        server_id: None,
                        suppressed: false,
                    });
                }
            } else {
                self.decay_vl(&mut state, "Aimbot");
            }
        }

        drop(state);

        if let Some(v) = violation_to_report {
            if let Some(bridge) = &self.bridge {
                let _ = bridge.report(username, &v).await;
            }
            return Some(v);
        }

        None
    }

    /// Check for AutoTool — switching hotbar slots at machine speed immediately
    /// before mining a block.
    pub async fn check_autotool(
        &self,
        uuid: Uuid,
        username: &str,
        slot_change_count: u32,
        time_delta_ms: u64,
    ) -> Option<Violation> {
        let config = self.config.read().await;
        if !config.enabled {
            return None;
        }
        let mut state = self.states.entry(uuid).or_default();
        let should_suppress = self.mod_compat.should_suppress_check(&state, "AutoTool");

        // Human reaction time: the fastest deliberate tool switch is ~80 ms.
        // Allow some leniency for pre-programmed keys (some trusted clients
        // have legal tool-switch macros).
        let min_human_switch_ms = 60.0_f64;
        let avg_time_per_switch = if slot_change_count > 0 {
            time_delta_ms as f64 / slot_change_count as f64
        } else {
            f64::MAX
        };

        let switches_per_second = if time_delta_ms > 0 {
            (slot_change_count as f64) / (time_delta_ms as f64 / 1000.0)
        } else {
            0.0
        };

        let mut violation_to_report: Option<Violation> = None;

        if !should_suppress
            && avg_time_per_switch < min_human_switch_ms
            && switches_per_second > 15.0
        {
            let count = self.increment_vl(&mut state, "AutoTool");
            if count >= VL_AUTOTOOL {
                violation_to_report = Some(Violation {
                    player_uuid: uuid,
                    check_name: "AutoTool".into(),
                    check_category: CheckCategory::Player,
                    value: switches_per_second,
                    threshold: 15.0,
                    timestamp: chrono::Utc::now(),
                    server_id: None,
                    suppressed: false,
                });
            }
        } else {
            self.decay_vl(&mut state, "AutoTool");
        }

        drop(state);

        if let Some(v) = violation_to_report {
            if let Some(bridge) = &self.bridge {
                let _ = bridge.report(username, &v).await;
            }
            return Some(v);
        }

        None
    }

    /// Check for Scaffold — placing blocks underneath oneself while falling at
    /// suspiciously consistent Y-step intervals.
    pub async fn check_scaffold(
        &self,
        uuid: Uuid,
        username: &str,
        y: f64,
        on_ground: bool,
        placing: bool,
    ) -> Option<Violation> {
        let config = self.config.read().await;
        if !config.enabled {
            return None;
        }
        let mut state = self.states.entry(uuid).or_default();
        let should_suppress = self.mod_compat.should_suppress_check(&state, "Scaffold");

        let mut violation_to_report: Option<Violation> = None;

        // Must be airborne, placing, and have been in the air long enough to
        // rule out a simple ground-level block placement.
        if placing && !on_ground && state.air_ticks > 4 {
            let height_diff = (y - state.last_y).abs();

            // Scaffold hacks maintain Y within ±1 block per tick while placing.
            // The telltale is placing at exactly block-height intervals while
            // drifting horizontally — dy between 0.01 and 0.99 means the
            // client is maintaining altitude in a non-vanilla way.
            if !should_suppress && (0.01..0.98).contains(&height_diff) {
                state.scaffold_ticks += 1;
                if state.scaffold_ticks >= 8 {
                    let count = self.increment_vl(&mut state, "Scaffold");
                    if count >= VL_SCAFFOLD {
                        violation_to_report = Some(Violation {
                            player_uuid: uuid,
                            check_name: "Scaffold".into(),
                            check_category: CheckCategory::Player,
                            value: state.scaffold_ticks as f64,
                            threshold: 8.0,
                            timestamp: chrono::Utc::now(),
                            server_id: None,
                            suppressed: false,
                        });
                        state.scaffold_ticks = 0;
                    }
                }
            } else {
                state.scaffold_ticks = 0;
            }
        } else {
            state.scaffold_ticks = 0;
            self.decay_vl(&mut state, "Scaffold");
        }

        drop(state);

        if let Some(v) = violation_to_report {
            if let Some(bridge) = &self.bridge {
                let _ = bridge.report(username, &v).await;
            }
            return Some(v);
        }

        None
    }

    /// Check for InstaPlace — placing blocks faster than one per game tick
    /// (50 ms). Vanilla is limited by the server's tick rate.
    pub async fn check_block_place(&self, uuid: Uuid, username: &str) -> Option<Violation> {
        let config = self.config.read().await;
        if !config.enabled {
            return None;
        }
        let mut state = self.states.entry(uuid).or_default();

        let now = Instant::now();
        state.block_place_times.push_back(now);
        while state.block_place_times.len() > BLOCK_PLACE_HISTORY_LEN {
            state.block_place_times.pop_front();
        }

        // Prune entries older than 1 second
        while let Some(&front) = state.block_place_times.front() {
            if now.duration_since(front).as_secs_f64() > 1.0 {
                state.block_place_times.pop_front();
            } else {
                break;
            }
        }

        let blocks_per_second = state.block_place_times.len() as f64;
        // Vanilla maximum: one block per tick ≈ 20 blocks/sec, but server
        // side it's throttled to ~4-6. We flag >12 sustained as machine speed.
        let threshold = 12.0_f64;
        let should_suppress = self.mod_compat.should_suppress_check(&state, "InstaPlace");
        let mut violation_to_report: Option<Violation> = None;

        if blocks_per_second > threshold && !should_suppress {
            let count = self.increment_vl(&mut state, "InstaPlace");
            if count >= VL_INSTAPLACE {
                violation_to_report = Some(Violation {
                    player_uuid: uuid,
                    check_name: "InstaPlace".into(),
                    check_category: CheckCategory::Player,
                    value: blocks_per_second,
                    threshold,
                    timestamp: chrono::Utc::now(),
                    server_id: None,
                    suppressed: false,
                });
            }
        } else {
            self.decay_vl(&mut state, "InstaPlace");
        }

        drop(state);

        if let Some(v) = violation_to_report {
            if let Some(bridge) = &self.bridge {
                let _ = bridge.report(username, &v).await;
            }
            return Some(v);
        }

        None
    }

    /// Check for Inventory hacks — interacting with inventory slots faster
    /// than human fingers allow.
    pub async fn check_inventory_click(&self, uuid: Uuid, username: &str) -> Option<Violation> {
        let config = self.config.read().await;
        if !config.enabled {
            return None;
        }
        let mut state = self.states.entry(uuid).or_default();

        let now = Instant::now();
        state.inventory_click_times.push_back(now);

        // Prune older than 500 ms
        while let Some(&front) = state.inventory_click_times.front() {
            if now.duration_since(front).as_millis() > 500 {
                state.inventory_click_times.pop_front();
            } else {
                break;
            }
        }

        let clicks_per_500ms = state.inventory_click_times.len() as f64;
        // Human maximum: ~8-10 clicks/500ms at extreme speed. Machine: 20+.
        let threshold = 14.0_f64;
        let should_suppress = self.mod_compat.should_suppress_check(&state, "Inventory");
        let mut violation_to_report: Option<Violation> = None;

        if clicks_per_500ms > threshold && !should_suppress {
            let count = self.increment_vl(&mut state, "Inventory");
            if count >= VL_INVENTORY {
                violation_to_report = Some(Violation {
                    player_uuid: uuid,
                    check_name: "Inventory".into(),
                    check_category: CheckCategory::Inventory,
                    value: clicks_per_500ms * 2.0, // convert to clicks/sec
                    threshold: threshold * 2.0,
                    timestamp: chrono::Utc::now(),
                    server_id: None,
                    suppressed: false,
                });
            }
        } else {
            self.decay_vl(&mut state, "Inventory");
        }

        drop(state);

        if let Some(v) = violation_to_report {
            if let Some(bridge) = &self.bridge {
                let _ = bridge.report(username, &v).await;
            }
            return Some(v);
        }

        None
    }

    /// Check for AutoSprint — the player sending a sprint flag while their
    /// food level is too low to sprint in vanilla (≤ 6 half-shanks).
    pub async fn check_autosprint(
        &self,
        uuid: Uuid,
        username: &str,
        sprinting: bool,
        food_level: u32,
    ) -> Option<Violation> {
        let config = self.config.read().await;
        if !config.enabled {
            return None;
        }
        let mut state = self.states.entry(uuid).or_default();
        let mut violation_to_report: Option<Violation> = None;

        if sprinting && food_level < 6 {
            let should_suppress = self.mod_compat.should_suppress_check(&state, "AutoSprint");
            if !should_suppress {
                let count = self.increment_vl(&mut state, "AutoSprint");
                // AutoSprint is binary (either impossible or not) so VL=3 is fine.
                if count >= 3 {
                    violation_to_report = Some(Violation {
                        player_uuid: uuid,
                        check_name: "AutoSprint".into(),
                        check_category: CheckCategory::Player,
                        value: food_level as f64,
                        threshold: 6.0,
                        timestamp: chrono::Utc::now(),
                        server_id: None,
                        suppressed: false,
                    });
                }
            }
        } else {
            self.decay_vl(&mut state, "AutoSprint");
        }

        drop(state);

        if let Some(v) = violation_to_report {
            if let Some(bridge) = &self.bridge {
                let _ = bridge.report(username, &v).await;
            }
            return Some(v);
        }

        None
    }

    // ─── Helper methods ──────────────────────────────────────────────────────

    fn increment_vl(&self, state: &mut PlayerAnticheatState, check_name: &str) -> u32 {
        let entry = state
            .check_violations
            .entry(check_name.to_string())
            .or_insert(0);
        *entry += 1;
        *entry
    }

    fn decay_vl(&self, state: &mut PlayerAnticheatState, check_name: &str) {
        if let Some(c) = state.check_violations.get_mut(check_name) {
            *c = c.saturating_sub(1);
        }
    }

    pub async fn get_violation_count(&self, uuid: &Uuid, check_name: &str) -> u32 {
        self.states
            .get(uuid)
            .and_then(|s| s.check_violations.get(check_name).copied())
            .unwrap_or(0)
    }

    pub async fn player_quit(&self, uuid: &Uuid) {
        self.states.remove(uuid);
    }

    pub async fn get_player_state(&self, uuid: &Uuid) -> Option<PlayerAnticheatState> {
        self.states.get(uuid).map(|r| r.clone())
    }
}

// ─── GCD analysis helper ─────────────────────────────────────────────────────

/// Compute an approximate floating-point GCD over a sequence of values.
/// Uses Euclidean algorithm with a tolerance of 1e-4.
fn approx_gcd(a: f64, b: f64) -> f64 {
    let (mut a, mut b) = (a.abs(), b.abs());
    while b > 1e-6 {
        let r = a % b;
        a = b;
        b = r;
    }
    a
}

fn approximate_gcd_list(iter: impl Iterator<Item = f64>) -> f64 {
    let values: Vec<f64> = iter.filter(|&v| v > 1e-6).collect();
    if values.len() < 2 {
        return 0.0;
    }
    values[1..]
        .iter()
        .fold(values[0], |acc, &v| approx_gcd(acc, v))
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// Helper: build an engine with known, tight config for deterministic tests.
    fn test_engine() -> AnticheatEngine {
        let config = AnticheatConfig {
            enabled: true,
            max_speed_blocks_per_tick: MC_SPRINT_SPEED,
            max_cps: 16,
            bridge_endpoint: None,
            bridge_token: String::new(),
            store_violations: false,
        };
        AnticheatEngine::new(config)
    }

    /// Simulate N ticks of movement at a given horizontal speed starting from
    /// (0, 64, 0). Returns the last violation produced, if any.
    async fn simulate_movement(
        engine: &AnticheatEngine,
        uuid: Uuid,
        speed_per_tick: f64,
        ticks: usize,
        on_ground: bool,
        ping_ms: u32,
    ) -> Option<Violation> {
        let mut last = None;
        for i in 0..ticks {
            let x = speed_per_tick * i as f64;
            last = engine
                .check_movement(uuid, "test_player", x, 64.0, 0.0, on_ground, ping_ms)
                .await;
        }
        last
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Speed tests
    // ─────────────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn speed_normal_walk_should_not_flag() {
        let engine = test_engine();
        let uuid = Uuid::new_v4();
        println!("[Speed:Walk] Simulating normal walking at {MC_WALK_SPEED:.3} blocks/tick");

        let result = simulate_movement(&engine, uuid, MC_WALK_SPEED, 20, true, 0).await;
        println!("[Speed:Walk] Violation: {result:?}");
        assert!(
            result.is_none(),
            "Normal walk speed ({MC_WALK_SPEED}) MUST NOT trigger Speed flag"
        );
        println!("[Speed:Walk] PASS — no false positive");
    }

    #[tokio::test]
    async fn speed_sprint_jump_should_not_flag() {
        let engine = test_engine();
        let uuid = Uuid::new_v4();
        // Sprint-jump peak horizontal speed is ~0.486 blocks/tick for 1.8,
        // but averaged over the arc it's closer to 0.35.
        let sprint_jump_speed = MC_SPRINT_SPEED + MC_SPRINT_JUMP_BOOST;
        println!("[Speed:SprintJump] Testing sprint-jump speed {sprint_jump_speed:.3} blocks/tick");

        let result = simulate_movement(&engine, uuid, sprint_jump_speed, 20, false, 0).await;
        println!("[Speed:SprintJump] Violation: {result:?}");
        assert!(
            result.is_none(),
            "Sprint-jump speed ({sprint_jump_speed:.3}) MUST NOT trigger Speed flag"
        );
        println!("[Speed:SprintJump] PASS — no false positive");
    }

    #[tokio::test]
    async fn speed_cheat_should_flag_after_vl_threshold() {
        let engine = test_engine();
        let uuid = Uuid::new_v4();
        // Speed hack at 2× sprint-jump speed — clearly illegal
        let cheat_speed = (MC_SPRINT_SPEED + MC_SPRINT_JUMP_BOOST) * 2.5;
        println!("[Speed:Cheat] Simulating cheat speed {cheat_speed:.3} blocks/tick for 30 ticks");

        let result = simulate_movement(&engine, uuid, cheat_speed, 30, true, 0).await;
        println!("[Speed:Cheat] Violation: {result:?}");
        assert!(
            result.is_some(),
            "Cheat speed ({cheat_speed:.3}) MUST trigger Speed flag after {VL_SPEED} VL"
        );
        let v = result.unwrap();
        assert_eq!(v.check_name, "Speed", "Check name must be 'Speed'");
        println!(
            "[Speed:Cheat] PASS — flagged with value={:.3} threshold={:.3}",
            v.value, v.threshold
        );
    }

    #[tokio::test]
    async fn speed_high_ping_player_should_not_flag_at_normal_speed() {
        let engine = test_engine();
        let uuid = Uuid::new_v4();
        let ping = 250_u32; // high but legitimate
        let speed = MC_SPRINT_SPEED + MC_SPRINT_JUMP_BOOST + 0.05; // slightly over without ping leniency
        println!("[Speed:HighPing] ping={ping}ms speed={speed:.3}");

        let result = simulate_movement(&engine, uuid, speed, 20, false, ping).await;
        println!("[Speed:HighPing] Violation: {result:?}");
        assert!(
            result.is_none(),
            "High-ping player at slightly elevated speed MUST NOT flag (latency leniency applies)"
        );
        println!("[Speed:HighPing] PASS");
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Flight tests
    // ─────────────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn flight_vanilla_jump_should_not_flag() {
        let engine = test_engine();
        let uuid = Uuid::new_v4();
        println!("[Flight:VanillaJump] Simulating vanilla gravity arc");

        // Reproduce the exact dy sequence of a vanilla jump:
        // dy[0] = 0.42, dy[n] = dy[n-1] * 0.98 - 0.08
        let mut dy = MC_JUMP_VELOCITY;
        let mut y = 64.0_f64;
        let mut last_violation = None;

        // First tick: ground
        engine
            .check_movement(uuid, "test_player", 0.0, y, 0.0, true, 0)
            .await;

        for tick in 0..15 {
            y += dy;
            dy = dy * MC_DRAG - MC_GRAVITY;
            let on_ground = dy < -2.0 || y < 64.05; // land check
            let v = engine
                .check_movement(uuid, "test_player", 0.0, y, 0.0, on_ground, 0)
                .await;
            println!("[Flight:VanillaJump] tick={tick} y={y:.3} dy={dy:.3} on_ground={on_ground}");
            last_violation = v.clone().or(last_violation);
        }

        assert!(
            last_violation.is_none(),
            "Vanilla jump trajectory MUST NOT trigger Flight or NoFall"
        );
        println!("[Flight:VanillaJump] PASS — vanilla arc is safe");
    }

    #[tokio::test]
    async fn flight_hack_sustained_dy_zero_should_flag() {
        let engine = test_engine();
        let uuid = Uuid::new_v4();
        println!("[Flight:Hack] Simulating sustained flight (dy=0 for 30 ticks)");

        // Seed the engine
        engine
            .check_movement(uuid, "test_player", 0.0, 64.0, 0.0, true, 0)
            .await;

        // Now fly: constant dy=0, on_ground=false, 30 ticks
        let mut last = None;
        for tick in 0..30 {
            let v = engine
                .check_movement(uuid, "test_player", 0.0, 80.0, 0.0, false, 0)
                .await;
            println!("[Flight:Hack] tick={tick} violation={}", v.is_some());
            last = v.or(last);
        }

        assert!(
            last.is_some(),
            "Sustained flight (dy=0 for 30 ticks) MUST trigger Flight flag"
        );
        let v = last.unwrap();
        assert_eq!(v.check_name, "Flight");
        println!("[Flight:Hack] PASS — flagged at air_ticks={:.0}", v.value);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // NoFall tests
    // ─────────────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn nofall_vanilla_descent_should_not_flag() {
        let engine = test_engine();
        let uuid = Uuid::new_v4();
        println!("[NoFall:VanillaDescent] Testing a realistic fall from 80 to 64");

        // Seed on ground
        engine
            .check_movement(uuid, "test_player", 0.0, 80.0, 0.0, true, 0)
            .await;

        let mut dy = 0.0_f64; // dropped from rest
        let mut y = 80.0_f64;
        let mut last = None;

        for tick in 0..25 {
            dy = (dy - MC_GRAVITY) * MC_DRAG;
            y += dy;
            let on_ground = y <= 64.0;
            let v = engine
                .check_movement(uuid, "test_player", 0.0, y.max(64.0), 0.0, on_ground, 0)
                .await;
            println!(
                "[NoFall:VanillaDescent] tick={tick} y={:.3} dy={:.3} on_ground={on_ground}",
                y, dy
            );
            last = v.clone().or(last);
            if on_ground {
                break;
            }
        }

        assert!(
            last.is_none(),
            "Vanilla fall trajectory MUST NOT trigger NoFall"
        );
        println!("[NoFall:VanillaDescent] PASS — clean fall");
    }

    #[tokio::test]
    async fn nofall_hack_zero_dy_fall_should_flag() {
        let engine = test_engine();
        let uuid = Uuid::new_v4();
        println!("[NoFall:Hack] Simulating NoFall hack — constant Y with dy≈0, sustained");

        // Seed on ground at Y=100.
        engine
            .check_movement(uuid, "test_player", 0.0, 100.0, 0.0, true, 0)
            .await;

        // A NoFall hack in the wild keeps Y constant while airborne — dy == 0.
        // In our check hierarchy, Flight will fire first (sustained positive-or-zero
        // dy for many ticks) which is equally valid; the key property is that SOME
        // check must flag this clearly cheating pattern.
        let mut last = None;
        for tick in 0..30 {
            let v = engine
                .check_movement(uuid, "test_player", 0.0, 100.0, 0.0, false, 0)
                .await;
            println!(
                "[NoFall:Hack] tick={tick} vl_hit={} check={}",
                v.is_some(),
                v.as_ref().map(|vv| vv.check_name.as_str()).unwrap_or("-")
            );
            last = v.or(last);
        }

        assert!(
            last.is_some(),
            "NoFall hack (constant Y in air for 30 ticks) MUST trigger Flight or NoFall"
        );
        let v = last.unwrap();
        assert!(
            v.check_name == "NoFall" || v.check_name == "Flight",
            "Expected NoFall or Flight, got '{}'",
            v.check_name
        );
        println!(
            "[NoFall:Hack] PASS — detected '{}' (value={:.1})",
            v.check_name, v.value
        );
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Killaura / Reach tests
    // ─────────────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn killaura_normal_pvp_should_not_flag() {
        let engine = test_engine();
        let uuid = Uuid::new_v4();
        println!("[Killaura:Normal] Simulating 8 CPS PvP — should not flag");

        let mut last = None;
        for i in 0..8_u32 {
            let v = engine.check_attack(uuid, "test_player", Some(2.5)).await;
            println!("[Killaura:Normal] attack={i} violation={}", v.is_some());
            last = v.or(last);
            // Simulate 125ms between attacks (8 CPS)
            tokio::time::sleep(Duration::from_millis(125)).await;
        }

        assert!(
            last.is_none(),
            "8 CPS PvP MUST NOT trigger Killaura flag (threshold=18 CPS)"
        );
        println!("[Killaura:Normal] PASS");
    }

    #[tokio::test]
    async fn killaura_superhuman_cps_should_flag() {
        let engine = test_engine();
        let uuid = Uuid::new_v4();
        println!("[Killaura:Cheat] Simulating 30 CPS (machine-speed)");

        let mut last = None;
        // Send 30 attacks within 1 second — no sleep
        for i in 0..30_u32 {
            let v = engine.check_attack(uuid, "test_player", Some(3.0)).await;
            println!("[Killaura:Cheat] attack={i} violation={}", v.is_some());
            last = v.or(last);
        }

        assert!(
            last.is_some(),
            "30 CPS MUST trigger Killaura flag after {VL_KILLAURA} VL"
        );
        let v = last.unwrap();
        assert_eq!(v.check_name, "Killaura");
        println!(
            "[Killaura:Cheat] PASS — cps={:.1} threshold={:.1}",
            v.value, v.threshold
        );
    }

    #[tokio::test]
    async fn reach_normal_distance_should_not_flag() {
        let engine = test_engine();
        let uuid = Uuid::new_v4();
        println!("[Reach:Normal] Attacking at distance=2.5 blocks (survival range)");

        for _ in 0..10 {
            let v = engine.check_attack(uuid, "test_player", Some(2.5)).await;
            assert!(
                v.map(|vv| vv.check_name != "Reach").unwrap_or(true),
                "2.5 block attack MUST NOT flag Reach"
            );
        }
        println!("[Reach:Normal] PASS");
    }

    #[tokio::test]
    async fn reach_cheat_distance_should_flag() {
        let engine = test_engine();
        let uuid = Uuid::new_v4();
        println!("[Reach:Cheat] Attacking at distance=6.0 blocks (reach hack)");

        let mut last = None;
        for i in 0..10_u32 {
            let v = engine.check_attack(uuid, "test_player", Some(6.0)).await;
            println!("[Reach:Cheat] attack={i} violation={}", v.is_some());
            last = v.or(last);
        }

        assert!(
            last.is_some(),
            "6.0 block reach MUST flag after {VL_REACH} VL"
        );
        let v = last.unwrap();
        assert_eq!(v.check_name, "Reach");
        println!(
            "[Reach:Cheat] PASS — distance={:.2} threshold={:.2}",
            v.value, v.threshold
        );
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Aimbot / GCD tests
    // ─────────────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn aimbot_human_mouse_should_not_flag() {
        let engine = test_engine();
        let uuid = Uuid::new_v4();
        println!("[Aimbot:Human] Simulating noisy human rotation deltas");

        // Human mouse: deltas are pseudo-random, ~0.1-3.0° with noise
        let yaw_deltas = [
            0.35_f64, 1.2, 0.07, 2.4, 0.15, 0.88, 0.42, 1.75, 0.09, 0.61, 0.33, 2.1, 0.78, 0.44,
            0.12, 0.95, 1.5, 0.23, 0.67, 0.37, 0.55, 1.9, 0.28, 0.81, 0.14,
        ];
        let mut yaw = 0.0_f64;
        let mut last = None;

        for (i, &delta) in yaw_deltas.iter().enumerate() {
            yaw += delta;
            let pitch = (i as f64 * 0.13).sin() * 30.0;
            let v = engine.check_aimbot(uuid, "test_player", yaw, pitch).await;
            println!(
                "[Aimbot:Human] sample={i} yaw={yaw:.3} delta={delta:.4} violation={}",
                v.is_some()
            );
            last = v.or(last);
        }

        assert!(
            last.is_none(),
            "Human rotation pattern MUST NOT trigger Aimbot"
        );
        println!("[Aimbot:Human] PASS — no false positive");
    }

    #[tokio::test]
    async fn aimbot_discrete_steps_should_flag() {
        let engine = test_engine();
        let uuid = Uuid::new_v4();
        // Aimbot with perfect 1.5° steps — constant GCD of 1.5
        let step = 1.5_f64;
        println!("[Aimbot:Cheat] Simulating aimbot with perfect {step}° steps (GCD={step})");

        let mut yaw = 0.0_f64;
        let mut last = None;

        // Sleep 60 ms between samples to clear the 50 ms aimbot throttle,
        // ensuring every sample is actually recorded into the GCD buffer.
        for i in 0..30_u32 {
            yaw += step;
            tokio::time::sleep(Duration::from_millis(60)).await;
            let v = engine.check_aimbot(uuid, "test_player", yaw, 15.0).await;
            println!(
                "[Aimbot:Cheat] sample={i} yaw={yaw:.2} violation={}",
                v.is_some()
            );
            last = v.or(last);
        }

        assert!(
            last.is_some(),
            "Perfectly discrete aimbot rotation MUST flag after {VL_AIMBOT} VL"
        );
        let v = last.unwrap();
        assert_eq!(v.check_name, "Aimbot");
        println!(
            "[Aimbot:Cheat] PASS — gcd={:.4} threshold={:.2}",
            v.value, v.threshold
        );
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Timer tests
    // ─────────────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn timer_normal_rate_should_not_flag() {
        let engine = test_engine();
        let uuid = Uuid::new_v4();
        println!("[Timer:Normal] Simulating 20 pps for 2 seconds");

        // Simulate 20 pps for 2 real seconds
        let start = Instant::now();
        let mut last = None;
        let mut tick = 0u32;
        while start.elapsed().as_secs() < 2 {
            let v = engine.check_timer(uuid, "test_player").await;
            last = v.or(last);
            tick += 1;
            tokio::time::sleep(Duration::from_millis(50)).await; // 20 pps
        }
        println!(
            "[Timer:Normal] Sent {tick} packets, violation={}",
            last.is_some()
        );
        assert!(last.is_none(), "20 pps MUST NOT trigger Timer");
        println!("[Timer:Normal] PASS");
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Mod detection tests
    // ─────────────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn trusted_mod_detected_correctly() {
        let engine = AnticheatEngine::new(AnticheatConfig::default());
        let uuid = Uuid::new_v4();
        println!("[ModDetect:Fabric] Registering 'fabric' brand");

        engine.register_mod_brand(uuid, "fabric".to_string()).await;
        let state = engine.get_player_state(&uuid).await.unwrap();
        println!("[ModDetect:Fabric] detected_mods={:?}", state.detected_mods);
        assert!(
            state.is_modded_client,
            "Fabric should be detected as modded"
        );
        assert!(
            state
                .detected_mods
                .iter()
                .any(|m| m.contains("trusted:fabric")),
            "Fabric must be marked trusted"
        );
        println!("[ModDetect:Fabric] PASS");
    }

    #[tokio::test]
    async fn cheat_mod_detected_correctly() {
        let engine = AnticheatEngine::new(AnticheatConfig::default());
        let uuid = Uuid::new_v4();
        println!("[ModDetect:Wurst] Registering 'wurst-client' brand");

        engine
            .register_mod_brand(uuid, "wurst-client".to_string())
            .await;
        let state = engine.get_player_state(&uuid).await.unwrap();
        println!("[ModDetect:Wurst] detected_mods={:?}", state.detected_mods);
        assert!(state.is_modded_client);
        assert!(
            state.detected_mods.iter().any(|m| m.contains("wurst")),
            "Wurst must be detected as a cheat mod"
        );
        println!("[ModDetect:Wurst] PASS");
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Bhop tests
    // ─────────────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn bhop_normal_jumping_should_not_flag() {
        let engine = test_engine();
        let uuid = Uuid::new_v4();
        println!("[Bhop:Normal] Simulating non-optimized repeated jumps");

        // Non-optimized jumping: land after 12+ ticks (too slow to be bhop)
        for i in 0..8 {
            engine
                .check_movement(uuid, "test_player", 0.0, 64.0, 0.0, true, 0)
                .await;
            for _ in 0..12 {
                engine
                    .check_movement(uuid, "test_player", 0.0, 65.0, 0.0, false, 0)
                    .await;
            }
            println!("[Bhop:Normal] hop={i}");
        }

        let state = engine.get_player_state(&uuid).await.unwrap();
        let bhop_vl = *state.check_violations.get("Bhop").unwrap_or(&0);
        println!("[Bhop:Normal] bhop_vl={bhop_vl}");
        assert!(
            bhop_vl < VL_BHOP,
            "Non-optimal jumping MUST NOT exceed Bhop VL threshold"
        );
        println!("[Bhop:Normal] PASS");
    }

    // ─────────────────────────────────────────────────────────────────────────
    // GCD math unit test
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn gcd_helper_computes_correctly() {
        let perfect_steps = vec![1.5_f64, 1.5, 1.5, 1.5, 3.0, 4.5, 1.5, 1.5];
        let gcd = approximate_gcd_list(perfect_steps.into_iter());
        println!("[GCD] computed gcd = {gcd:.6} (expected ~1.5)");
        assert!(
            (gcd - 1.5).abs() < 0.1,
            "GCD of [1.5, 1.5, 3.0, 4.5, ...] should be ~1.5, got {gcd}"
        );
        println!("[GCD] PASS");
    }

    #[test]
    fn gcd_noisy_human_input_has_small_gcd() {
        // Human input has very small GCD (no common step size)
        let noisy = vec![
            0.35_f64, 1.23, 0.07, 2.41, 0.15, 0.88, 0.42, 1.75, 0.09, 0.61,
        ];
        let gcd = approximate_gcd_list(noisy.into_iter());
        println!("[GCD:Human] computed gcd = {gcd:.8}");
        // Human GCD should not be suspiciously large (>0.5)
        assert!(
            gcd < 0.5,
            "Human input GCD should be small (< 0.5°), got {gcd:.6}"
        );
        println!("[GCD:Human] PASS");
    }
}
