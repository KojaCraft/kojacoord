//! Player authentication and forwarding.
//!
//! Covers the full Login sequence the proxy speaks to clients:
//!   - `encryption` — RSA keypair, AES session key (Notchian shared
//!     secret), CFB8 stream cipher
//!   - `microsoft` — online-mode `hasJoined` lookup against
//!     `sessionserver.mojang.com`
//!   - `offline` — deterministic UUID-from-username for `online_mode = false`
//!   - `pipeline` — the orchestrator that drives the above in order
//!   - `forwarding` — BungeeCord (legacy IP-forward suffix) and
//!     Velocity (signed plugin message) handoff to the backend so the
//!     real client IP / UUID survive past the proxy
//!   - `session` — the auth result shared with the rest of the proxy

#![deny(clippy::all)]

pub mod encryption;
pub mod error;
pub mod forwarding;
pub mod microsoft;
pub mod offline;
pub mod pipeline;
pub mod session;

pub use error::AuthError;
pub use pipeline::{
    AuthConfig, AuthEvent, AuthOutbound, AuthPipeline, AuthPipelineConfig, AuthState, AuthType,
};
pub use session::{AuthenticatedProfile, ProfileProperty};
