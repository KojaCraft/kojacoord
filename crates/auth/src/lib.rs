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
