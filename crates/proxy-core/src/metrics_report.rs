use crate::proxy::ProxyState;
use std::sync::Arc;
use std::time::Duration;

pub fn start_reporting(state: Arc<ProxyState>) {
    let url = state.config.metrics_backend.url.clone();
    let token = state.config.metrics_backend.token.clone();
    if url.is_empty() || token.is_empty() {
        tracing::info!("Metrics backend reporting not configured or missing token. Skipping modpack online tracking.");
        return;
    }

    tokio::spawn(async move {
        let client = reqwest::Client::new();
        loop {
            // Report modpack player counts every 30 seconds.
            tokio::time::sleep(Duration::from_secs(30)).await;

            for s in &state.config.servers {
                if let Some(slug) = &s.modpack {
                    if let Some(srv) = state.server_registry.get(&s.name) {
                        let count = srv.player_count() as i32;

                        let target_url =
                            format!("{}/v1/proxy/modpacks/online", url.trim_end_matches('/'));
                        let res = client
                            .post(&target_url)
                            .bearer_auth(&token)
                            .json(&serde_json::json!({
                                "slug": slug,
                                "server_name": s.name,
                                "online_count": count
                            }))
                            .send()
                            .await;

                        match res {
                            Ok(resp) => {
                                if !resp.status().is_success() {
                                    tracing::warn!(
                                        status = ?resp.status(),
                                        server = %s.name,
                                        modpack = %slug,
                                        "Failed to report modpack online players to metrics backend"
                                    );
                                }
                            },
                            Err(e) => {
                                tracing::error!(
                                    error = %e,
                                    server = %s.name,
                                    modpack = %slug,
                                    "Failed to send modpack online players report"
                                );
                            },
                        }
                    }
                }
            }
        }
    });
}
