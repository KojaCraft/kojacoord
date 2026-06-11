//! Netty-style channel pipeline primitives.
//!
//! A minimal Rust port of the Netty channel/handler/pipeline shape
//! so callers familiar with the Java side have something they
//! recognise. The proxy proper doesn't use this directly — it's here
//! for tooling and for plugins that want a higher-level abstraction
//! than raw `tokio::io` for protocol experiments.

#![deny(clippy::all)]

pub mod cipher;
pub mod error;
pub mod frame;
pub mod handlers;
pub mod pipeline;

pub use error::{HandlerError, PipelineError};
pub use handlers::{ChannelContext, ChannelHandler, Direction};
pub use pipeline::ChannelPipeline;
