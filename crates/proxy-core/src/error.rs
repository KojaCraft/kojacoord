//! Top-level error types for the proxy.
//!
//! [`ConnectionError`] is what every connection IO path returns.
//! Variants are coarse on purpose — callers care about "retry,
//! close, or kick?" more than the specific bytes that went wrong.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConnectionError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("protocol error: {0}")]
    Protocol(#[from] kojacoord_protocol::ProtocolError),

    #[error("authentication failed: {0}")]
    Auth(String),

    #[error("no backend server available")]
    NoBackend,

    #[error("connection closed")]
    Closed,

    #[error("backend reconnect needed")]
    Reconnect,
}
