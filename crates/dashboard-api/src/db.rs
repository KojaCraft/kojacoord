use sqlx::{MySql, Pool};

pub type DbPool = Pool<MySql>;

pub async fn connect(url: &str, max: u32) -> anyhow::Result<DbPool> {
    Ok(sqlx::mysql::MySqlPoolOptions::new()
        .max_connections(max)
        .connect(url)
        .await?)
}

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
pub struct AdminUser {
    pub id: i64,

    pub username: String,

    #[serde(skip_serializing)]
    pub password_hash: String,

    pub role: String,

    pub last_login: Option<chrono::DateTime<chrono::Utc>>,
}

pub async fn find_admin(pool: &DbPool, username: &str) -> Result<Option<AdminUser>, sqlx::Error> {
    sqlx::query_as::<_, AdminUser>("SELECT * FROM admin_users WHERE username = ?")
        .bind(username)
        .fetch_optional(pool)
        .await
}

pub async fn update_last_login(pool: &DbPool, id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE admin_users SET last_login = NOW() WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
pub struct Server {
    pub id: i64,

    pub name: String,

    pub template: String,

    pub status: String,

    pub address: Option<String>,

    pub port: Option<u16>,

    pub docker_container_id: Option<String>,
}

pub async fn list_servers(pool: &DbPool) -> Result<Vec<Server>, sqlx::Error> {
    sqlx::query_as::<_, Server>("SELECT * FROM servers ORDER BY id DESC")
        .fetch_all(pool)
        .await
}

pub async fn get_server_by_id(pool: &DbPool, id: i64) -> Result<Option<Server>, sqlx::Error> {
    sqlx::query_as::<_, Server>("SELECT * FROM servers WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
}

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
pub struct ServerTemplate {
    pub id: i64,
    pub name: String,
    pub game_type: Option<String>,
    pub image: Option<String>,
    pub memory_mb: Option<i64>,
    pub max_players: Option<i32>,
    pub min_instances: Option<i32>,
    pub modpack_id: Option<String>,
    pub modpack_loader: Option<String>,
    pub modpack_mc_version: Option<String>,
    pub modpack_status: Option<String>,
    pub modpack_source: Option<String>,
}

pub async fn list_templates(pool: &DbPool) -> Result<Vec<ServerTemplate>, sqlx::Error> {
    sqlx::query_as::<_, ServerTemplate>(
        "SELECT id, name, game_type, image, memory_mb, max_players, min_instances, \
         modpack_id, modpack_loader, modpack_mc_version, modpack_status, modpack_source \
         FROM server_templates ORDER BY name"
    )
    .fetch_all(pool)
    .await
}

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
pub struct ServerError {
    pub id: i64,

    pub server_id: i64,

    pub timestamp: chrono::DateTime<chrono::Utc>,

    pub level: String,

    pub message: String,

    pub stack_trace: Option<String>,
}

pub async fn list_errors(
    pool: &DbPool,
    server_id: Option<i64>,
    limit: u32,
) -> Result<Vec<ServerError>, sqlx::Error> {
    if let Some(sid) = server_id {
        sqlx::query_as::<_, ServerError>(
            "SELECT * FROM server_errors WHERE server_id = ? ORDER BY timestamp DESC LIMIT ?",
        )
        .bind(sid)
        .bind(limit)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, ServerError>(
            "SELECT * FROM server_errors ORDER BY timestamp DESC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(pool)
        .await
    }
}

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
pub struct Player {
    pub id: i64,

    pub uuid: String,

    pub username: String,

    pub rank: String,

    pub metadata: Option<String>,
}

pub async fn list_players(
    pool: &DbPool,
    limit: u32,
    offset: u32,
) -> Result<Vec<Player>, sqlx::Error> {
    sqlx::query_as::<_, Player>("SELECT * FROM players ORDER BY username LIMIT ? OFFSET ?")
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
}

pub async fn list_admin_users(pool: &DbPool) -> anyhow::Result<Vec<AdminUser>> {
    Ok(sqlx::query_as::<_, AdminUser>(
        "SELECT id, username, '' AS password_hash, role, last_login FROM admin_users ORDER BY id",
    )
    .fetch_all(pool)
    .await?)
}

pub async fn create_admin_user(
    pool: &DbPool,
    username: &str,
    password_hash: &str,
    role: &str,
) -> anyhow::Result<i64> {
    let res = sqlx::query(
        "INSERT INTO admin_users (username, password_hash, role) VALUES (?, ?, ?)",
    )
    .bind(username)
    .bind(password_hash)
    .bind(role)
    .execute(pool)
    .await?;
    Ok(res.last_insert_id() as i64)
}

pub async fn update_admin_user_role(pool: &DbPool, id: i64, role: &str) -> anyhow::Result<()> {
    sqlx::query("UPDATE admin_users SET role=? WHERE id=?")
        .bind(role)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn delete_admin_user(pool: &DbPool, id: i64) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM admin_users WHERE id=?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}
