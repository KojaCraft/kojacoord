use axum::response::sse::Event;
use serde::Serialize;
use tokio::sync::broadcast;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
#[allow(dead_code)]
pub enum DashboardEvent {
    WrongModpack {
        player_uuid: String,
        player_name: String,
        server: String,
        required_modpack: String,
        client_modpack: String,
    },
    PlayerJoined {
        player_uuid: String,
        player_name: String,
        server: String,
    },
    PlayerLeft {
        player_uuid: String,
        player_name: String,
    },
    ServerStatus {
        server_id: i64,
        server_name: String,
        status: String,
    },
}

impl DashboardEvent {
    pub fn to_sse_event(&self) -> Result<Event, serde_json::Error> {
        let json = serde_json::to_string(self)?;
        Ok(Event::default().data(json))
    }
}

#[derive(Clone)]
pub struct EventBus {
    tx: broadcast::Sender<DashboardEvent>,
}

impl EventBus {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(256);
        Self { tx }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<DashboardEvent> {
        self.tx.subscribe()
    }

    pub fn publish(&self, event: DashboardEvent) {
        let _ = self.tx.send(event);
    }
}
