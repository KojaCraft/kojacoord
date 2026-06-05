use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub server: ServerConfig,

    pub database: DatabaseConfig,

    pub s3: S3Config,

    pub jwt: JwtConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_bind")]
    pub bind: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,

    #[serde(default = "default_pool")]
    pub max_connections: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct S3Config {
    pub bucket: String,
    pub region: String,
    pub access_key_id: String,
    pub secret_access_key: String,
    pub endpoint_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JwtConfig {
    pub secret: String,

    #[serde(default = "default_jwt_exp")]
    pub expiry_hours: u64,
}

fn default_bind() -> String {
    "0.0.0.0:3001".into()
}
fn default_pool() -> u32 {
    10
}
fn default_jwt_exp() -> u64 {
    24
}

impl Config {
    pub fn load(path: &str) -> anyhow::Result<Self> {
        use figment::{
            providers::{Env, Format, Toml},
            Figment,
        };
        Ok(Figment::new()
            .merge(Toml::file(path))
            .merge(Env::prefixed("DASH_").global())
            .extract()?)
    }
}
