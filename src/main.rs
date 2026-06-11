mod config;
mod error;
mod gateway;

use std::{collections::HashMap, sync::Arc, time::Duration};

use anyhow::Context;
use axum::{
    Router,
    routing::{get, post},
};
use reqwest::Client;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing::info;
use tracing_subscriber::{EnvFilter, fmt};

use crate::{
    config::AppConfig,
    gateway::{AppState, chat_completions, healthz, metrics, models},
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    let config_path =
        std::env::var("NUSAROUTE_CONFIG").unwrap_or_else(|_| "config.example.yaml".to_string());
    let config = Arc::new(AppConfig::load(&config_path)?);

    let client = Client::builder()
        .pool_idle_timeout(Duration::from_secs(90))
        .pool_max_idle_per_host(64)
        .tcp_nodelay(true)
        .timeout(Duration::from_millis(config.server.request_timeout_ms))
        .build()
        .context("failed to build upstream HTTP client")?;

    let state = AppState {
        config: Arc::clone(&config),
        client,
        stats: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
    };

    let app = Router::new()
        .route("/healthz", get(healthz))
        .route("/metrics", get(metrics))
        .route("/v1/models", get(models))
        .route("/v1/chat/completions", post(chat_completions))
        .with_state(state)
        .layer(TraceLayer::new_for_http());

    let listener = TcpListener::bind(config.server.bind)
        .await
        .with_context(|| format!("failed to bind {}", config.server.bind))?;

    info!(
        bind = %config.server.bind,
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
