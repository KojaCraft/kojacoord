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

    /// A read/write against the *backend* server socket failed (I/O
    /// error, EOF, or a decode error), as opposed to the client
    /// socket. The distinction matters at the top of `connection.rs`:
    /// `client_gone()` treats bare `Io`/`Closed` as "nobody left to
    /// notify" and stays silent, which is correct when the *client*
    /// dropped but wrong when the *backend* did — the client is still
    /// there and deserves a real disconnect message instead of a
    /// silent socket close. Keeping the original error boxed inside
    /// preserves the underlying cause for logging.
    #[error("backend connection lost: {0}")]
    Backend(Box<ConnectionError>),
}
