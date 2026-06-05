use crate::player::ApiPlayer;

#[derive(Debug, Clone)]
pub enum ProxyEvent {
    PlayerConnect(ApiPlayer),

    PlayerDisconnect(ApiPlayer),

    PlayerSwitchServer {
        player: ApiPlayer,
        from: String,
        to: String,
    },

    ChatMessage {
        player: ApiPlayer,
        message: String,
    },
}
