use axum::{
    routing::{delete, get, post, put},
    Router,
};
use std::sync::Arc;

mod admin_users;
mod backups;
mod console;
mod events;
mod files;
mod login;
mod logs;
mod players;
mod servers;
mod templates;

pub use login::login_handler;

pub struct AppState {
    pub proxy: Arc<kojacoord_proxy_core::proxy::ProxyState>,
    pub pool: crate::db::DbPool,
    pub s3: Arc<crate::s3::S3Client>,
    pub cfg: Arc<crate::config::Config>,
    pub event_bus: Arc<crate::events::EventBus>,
}

pub fn public_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/auth/login", post(login_handler))
        .route(
            "/api/events/wrong-modpack",
            post(events::wrong_modpack_event),
        )
}

pub fn protected_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/servers", get(servers::list_servers))
        .route("/api/servers/:id/start", post(servers::start_server))
        .route("/api/servers/:id/stop", post(servers::stop_server))
        .route(
            "/api/servers/:id/backups",
            get(backups::list_server_backups),
        )
        .route("/api/backups", get(backups::list_backups))
        .route("/api/backups/download", get(backups::download_backup))
        .route("/api/backups/:key", delete(backups::delete_backup))
        .route("/api/players", get(players::list_players))
        .route("/api/players/banned", get(players::list_active_bans))
        .route("/api/players/:uuid/kick", post(players::kick_player))
        .route("/api/players/:uuid/ban", post(players::ban_player))
        .route("/api/players/:uuid/unban", post(players::unban_player))
        .route("/api/players/:uuid/bans", get(players::list_player_bans))
        .route("/api/admin-users", get(admin_users::list_admin_users))
        .route("/api/admin-users", post(admin_users::create_admin_user))
        .route(
            "/api/admin-users/:id",
            put(admin_users::update_admin_user_role),
        )
        .route(
            "/api/admin-users/:id",
            delete(admin_users::delete_admin_user),
        )
        .route("/api/templates", get(templates::list_templates))
        .route("/api/templates", post(templates::create_template))
        .route("/api/templates/:id", put(templates::update_template))
        .route("/api/templates/:id", delete(templates::delete_template))
        .route(
            "/api/templates/:id/download-modpack",
            post(templates::download_modpack),
        )
        .route(
            "/api/templates/:id/modpack-status",
            get(templates::modpack_status),
        )
        .route("/api/logs", get(logs::list_errors))
        .route("/api/files", get(files::list_files))
        .route("/api/files/upload", post(files::upload_file))
        .route("/api/files/server", post(files::upload_server_files))
        .route("/api/files/:key", delete(files::delete_file))
        .route("/api/console/:server_id", get(console::console_ws))
        .route("/api/events", get(events::sse_events))
        .with_state(state)
}
