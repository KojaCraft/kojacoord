#![deny(clippy::all)]

pub mod codec;
pub mod error;
pub mod negotiation;
pub mod registry;
pub mod types;
pub mod versions;

pub use codec::{Decode, Encode, PacketId};
pub use error::ProtocolError;
pub use negotiation::{ProtocolVersion, VersionRegistry};
pub use registry::{build_default_registry, Direction, PacketMeta, PacketRegistry, ProtocolState};
pub use types::{Position, Slot, VarInt, VarLong};
