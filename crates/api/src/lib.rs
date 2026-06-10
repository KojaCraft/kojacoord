//! Public event-listener API.
//!
//! External code (plugins, embedders, integration tests) implements
//! [`EventListener`] to be notified of lifecycle events on
//! [`player::ApiPlayer`] / [`server::ApiServer`]. The trait is
//! intentionally read-only — there's no callback that can mutate
//! proxy state, since the host has its own internal hook system for
//! that.

#![deny(clippy::all)]

pub mod events;
pub mod player;
pub mod server;

pub trait EventListener: Send + Sync {
    fn on_player_connect(&self, _player: &player::ApiPlayer) {}
    fn on_player_disconnect(&self, _player: &player::ApiPlayer) {}
    fn on_player_switch_server(&self, _player: &player::ApiPlayer, _from: &str, _to: &str) {}
    fn on_chat_message(&self, _player: &player::ApiPlayer, _message: &str) {}
}
