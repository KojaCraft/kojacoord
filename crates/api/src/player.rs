use uuid::Uuid;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ApiPlayer {
    pub uuid: Uuid,
    pub username: String,
    pub current_server: Option<String>,
    pub protocol_version: u32,
}
