use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct Violation {
    pub player_uuid: Uuid,
    pub check_name: String,
    pub check_category: CheckCategory,
    pub value: f64,
    pub threshold: f64,
    pub timestamp: DateTime<Utc>,
    pub server_id: Option<String>,
    pub suppressed: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum CheckCategory {
    Movement,
    Combat,
    Player,
    Network,
    Inventory,
}

impl CheckCategory {
    pub fn human_name(&self) -> &str {
        match self {
            CheckCategory::Movement => "Movement",
            CheckCategory::Combat => "Combat",
            CheckCategory::Player => "Player",
            CheckCategory::Network => "Network",
            CheckCategory::Inventory => "Inventory",
        }
    }
}
