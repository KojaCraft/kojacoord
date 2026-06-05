#![deny(clippy::all)]

pub mod alert_system;
pub mod bridge;
pub mod checks;
pub mod mod_compatibility;
pub mod packet_parser;
pub mod player_state;
pub mod violation;

pub use alert_system::{AlertConfig, AlertSystem};
pub use checks::AnticheatEngine;
pub use mod_compatibility::ModCompatibility;
pub use packet_parser::{parse_serverbound, AnticheatPacket, InteractData, MovementData};
pub use player_state::PlayerAnticheatState;
pub use violation::{CheckCategory, Violation};
