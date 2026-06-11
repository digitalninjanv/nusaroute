mod config;
mod error;
mod gateway;

use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, SystemTime},
};

use anyhow::Context;
use axum::{
    Json,
    Router,
    extract::State,
    http::StatusCode,
    routing::{get, post},
};
use reqwest::Client;
use serde_json::json;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing::{info, warn};
use tracing_subscriber::{EnvFilter, fmt};

use crate::{
    config::AppConfig,
    gateway::{AppState, chat_completions, healthz, metrics, models},
};

type SharedConfig = tokio::sync::RwLock<Arc<AppConfig>>;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    let config_path =
        std::env::var("NUSAROUTE_CONFIG").unwrap_or_else(|_| "config.example.yaml".to_string());
    let app_config = AppConfig::load(&config_path)?;
    let request_timeout = app_config.server.request_timeout_ms;
    let config = Arc::new(tokio::sync::RwLock::new(Arc::new(app_config)));

    let client = Client::builder()
        .pool_idle_timeout(Duration::from_secs(90))
        .pool_max_idle_per_host(64)
        .tcp_nodelay(true)
        .timeout(Duration::from_millis(request_timeout))
        .build()
        .context("failed to build upstream HTTP client")?;

    let state = AppState {
        config: Arc::clone(&config),
        client,
        stats: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        config_path: config_path.clone(),
    };

    let bind = config.read().await.server.bind;

    spawn_config_watcher(config_path.clone(), Arc::clone(&config));

    let app = Router::new()
        .route("/healthz", get(healthz))
        .route("/metrics", get(metrics))
        .route("/v1/models", get(models))
        .route("/v1/chat/completions", post(chat_completions))
        .route("/v1/admin/reload", post(reload_config))
        .with_state(state)
        .layer(TraceLayer::new_for_http());
    let listener = TcpListener::bind(bind)
        .await
        .with_context(|| format!("failed to bind {bind}"))?;

    info!(
        bind = %bind,
        config = %config_path,
        "nusaroute ai gateway started"
    );

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("server failed")?;

    Ok(())
}

fn init_tracing() {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("nusaroute_ai_gateway=info,tower_http=info"));

    fmt()
        .json()
        .with_env_filter(env_filter)
        .with_current_span(false)
        .with_span_list(false)
        .init();
}

fn spawn_config_watcher(path: String, config: Arc<SharedConfig>) {
    tokio::spawn(async move {
        let mut last_modified: Option<SystemTime> = None;
        loop {
            tokio::time::sleep(Duration::from_secs(2)).await;
            let Ok(metadata) = tokio::fs::metadata(&path).await else {
                continue;
            };
            let modified = metadata.modified().ok();
            if modified != last_modified && last_modified.is_some() {
                match AppConfig::load(&path) {
                    Ok(new_config) => {
                        *config.write().await = Arc::new(new_config);
                        info!("config reloaded from {}", path);
                    }
                    Err(e) => {
                        warn!("config reload failed for {}: {e}; keeping previous config", path);
                    }
                }
            }
            last_modified = modified;
        }
    });
}

async fn reload_config(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    match AppConfig::load(&state.config_path) {
        Ok(new_config) => {
            *state.config.write().await = Arc::new(new_config);
            info!("config reloaded via API from {}", state.config_path);
            Ok(Json(json!({
                "status": "ok",
                "message": "config reloaded successfully"
            })))
        }
        Err(e) => {
            warn!("config reload via API failed: {e}");
            Err((
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": {
                        "message": format!("{e}"),
                        "type": "nusaroute_gateway_error"
                    }
                })),
            ))
        }
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
