#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ApiServer {
    pub name: String,
    pub address: String,
    pub player_count: usize,
    pub online: bool,
}
