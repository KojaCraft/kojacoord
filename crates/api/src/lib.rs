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
