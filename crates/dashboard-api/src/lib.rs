mod auth;
mod config;
mod db;
mod error;
mod events;
mod modpack;
mod routes;
mod s3;
mod ws;

use std::sync::Arc;

use axum::http::{header, HeaderValue, Method};
use axum::Router;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::trace::TraceLayer;

use kojacoord_proxy_core::proxy::ProxyState;

pub use config::Config as DashboardConfig;

pub async fn serve(proxy: Arc<ProxyState>, config_path: &str) -> anyhow::Result<()> {
    let cfg = config::Config::load(config_path)?;

    let db = proxy.db.as_ref().ok_or_else(|| {
        anyhow::anyhow!("dashboard API requires a database, but the proxy has none configured")
    })?;
    let pool = db
        .mysql_pool()
        .ok_or_else(|| {
            anyhow::anyhow!("dashboard API requires MySQL database, but proxy is using SQLite")
        })?
        .clone();

    let s3 = Arc::new(s3::S3Client::new(&cfg.s3).await?);
    let event_bus = Arc::new(events::EventBus::new());

    let state = Arc::new(routes::AppState {
        proxy: Arc::clone(&proxy),
        pool,
        s3,
        cfg: Arc::new(cfg.clone()),
        event_bus,
    });

    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::exact(HeaderValue::from_static(
            "http://localhost:3000",
        )))
        .allow_credentials(true)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION]);

    let app = Router::new()
        .merge(routes::public_routes())
        .merge(routes::protected_routes(state.clone()))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state);

    let bind = cfg.server.bind.clone();
    let listener = tokio::net::TcpListener::bind(&bind).await?;
    tracing::info!("Dashboard API listening on {}", bind);
    axum::serve(listener, app).await?;
    Ok(())
}
