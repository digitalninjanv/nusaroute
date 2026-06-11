use std::{
    cmp::Ordering,
    collections::HashMap,
    env,
    sync::Arc,
    time::{Duration, Instant},
};

use axum::{
    Json,
    body::Body,
    extract::State,
    http::{HeaderMap, header},
    response::{IntoResponse, Response},
};
use futures_util::TryStreamExt;
use serde::Serialize;
use serde_json::{Value, json};
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::{
    config::{AppConfig, ModelCandidate, ProviderKind, RouteStrategy},
    error::ApiError,
};

type SharedConfig = tokio::sync::RwLock<Arc<AppConfig>>;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<SharedConfig>,
    pub client: reqwest::Client,
    pub stats: Arc<RwLock<HashMap<String, ProviderStats>>>,
    pub config_path: String,
}

#[derive(Debug, Clone, Default)]
pub struct ProviderStats {
    pub requests: u64,
    pub failures: u64,
    pub latency_ewma_ms: Option<f64>,
}

#[derive(Debug, Serialize)]
struct ModelsResponse {
    object: &'static str,
    data: Vec<ModelDescriptor>,
}

#[derive(Debug, Serialize)]
struct ModelDescriptor {
    id: String,
    object: &'static str,
    owned_by: &'static str,
    description: String,
}

pub async fn healthz() -> impl IntoResponse {
    Json(json!({ "status": "ok" }))
}

pub async fn models(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let config = state.config.read().await.clone();

    authorize(&config, &headers)?;

    let mut data = config
        .models
        .iter()
        .map(|(id, route)| ModelDescriptor {
            id: id.clone(),
            object: "model",
            owned_by: "nusaroute",
            description: route.description.clone(),
        })
        .collect::<Vec<_>>();
    data.sort_by(|a, b| a.id.cmp(&b.id));

    Ok(Json(ModelsResponse {
        object: "list",
        data,
    }))
}

pub async fn metrics(State(state): State<AppState>) -> impl IntoResponse {
    let stats = state.stats.read().await;
    let mut lines = Vec::new();
    lines.push("# TYPE nusaroute_provider_requests_total counter".to_string());
    lines.push("# TYPE nusaroute_provider_failures_total counter".to_string());
    lines.push("# TYPE nusaroute_provider_latency_ewma_ms gauge".to_string());

    for (provider, stat) in stats.iter() {
        lines.push(format!(
            "nusaroute_provider_requests_total{{provider=\"{}\"}} {}",
            escape_label(provider),
            stat.requests
        ));
        lines.push(format!(
            "nusaroute_provider_failures_total{{provider=\"{}\"}} {}",
            escape_label(provider),
            stat.failures
        ));
        if let Some(latency) = stat.latency_ewma_ms {
            lines.push(format!(
                "nusaroute_provider_latency_ewma_ms{{provider=\"{}\"}} {:.3}",
                escape_label(provider),
                latency
            ));
        }
    }

    (
        [(header::CONTENT_TYPE, "text/plain; version=0.0.4")],
        lines.join("\n"),
    )
}

pub async fn chat_completions(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(mut payload): Json<Value>,
) -> Result<Response, ApiError> {
    let config = state.config.read().await.clone();

    authorize(&config, &headers)?;

    let requested_model = payload
        .get("model")
        .and_then(Value::as_str)
        .ok_or(ApiError::MissingModel)?
        .to_string();

    let route = config
        .models
        .get(&requested_model)
        .ok_or_else(|| ApiError::UnknownModel(requested_model.clone()))?
        .clone();

    let candidates = ordered_candidates(&config, &state.stats, route.strategy.clone(), &route.candidates).await;
    if candidates.is_empty() {
        return Err(ApiError::NoEnabledProvider(requested_model));
    }

    let mut last_error = String::new();
    for candidate in candidates {
        payload["model"] = Value::String(candidate.model.clone());
        match forward_openai_compatible(&state, &candidate, &payload).await {
            Ok(response) => return Ok(response),
            Err(error) => {
                last_error = error;
                record_failure(&state, &candidate.provider).await;
                if !route.fallback {
                    break;
                }
            }
        }
    }

    Err(ApiError::UpstreamFailed(last_error))
}

async fn ordered_candidates(
    config: &AppConfig,
    stats: &tokio::sync::RwLock<HashMap<String, ProviderStats>>,
    strategy: RouteStrategy,
    candidates: &[ModelCandidate],
) -> Vec<ModelCandidate> {
    let stats = stats.read().await;
    let mut enabled = candidates
        .iter()
        .filter(|candidate| {
            config
                .providers
                .get(&candidate.provider)
                .map(|provider| provider.enabled)
                .unwrap_or(false)
        })
        .cloned()
        .collect::<Vec<_>>();

    match strategy {
        RouteStrategy::LowestLatency => enabled.sort_by(|a, b| {
            let a_latency = stats
                .get(&a.provider)
                .and_then(|stat| stat.latency_ewma_ms)
                .unwrap_or(f64::MAX);
            let b_latency = stats
                .get(&b.provider)
                .and_then(|stat| stat.latency_ewma_ms)
                .unwrap_or(f64::MAX);

            a_latency
                .partial_cmp(&b_latency)
                .unwrap_or(Ordering::Equal)
                .then_with(|| provider_priority(config, a).cmp(&provider_priority(config, b)))
        }),
        RouteStrategy::Priority => {
            enabled.sort_by_key(|candidate| provider_priority(config, candidate));
        }
    }

    enabled
}

fn provider_priority(config: &AppConfig, candidate: &ModelCandidate) -> u32 {
    config
        .providers
        .get(&candidate.provider)
        .map(|provider| provider.priority)
        .unwrap_or(u32::MAX)
}

async fn forward_openai_compatible(
    state: &AppState,
    candidate: &ModelCandidate,
    payload: &Value,
) -> Result<Response, String> {
    let config = state.config.read().await.clone();
    let provider = config
        .providers
        .get(&candidate.provider)
        .ok_or_else(|| format!("unknown provider '{}'", candidate.provider))?
        .clone();
    match provider.kind {
        ProviderKind::OpenaiCompatible => {}
    }

    let api_key = env::var(&provider.api_key_env)
        .map_err(|_| format!("missing environment variable '{}'", provider.api_key_env))?;
    let url = format!(
        "{}/chat/completions",
        provider.base_url.trim_end_matches('/')
    );

    let request_id = uuid::Uuid::new_v4();
    let started = Instant::now();
    let upstream = state
        .client
        .post(url)
        .bearer_auth(api_key)
        .header("x-request-id", request_id.to_string())
        .json(payload)
        .send()
        .await
        .map_err(|error| format!("{} request failed: {error}", candidate.provider))?;

    let status = upstream.status();
    let response_headers = upstream.headers().clone();
    let elapsed = started.elapsed();

    if !status.is_success() {
        let body = upstream.text().await.unwrap_or_default();
        warn!(
            provider = candidate.provider,
            model = candidate.model,
            status = status.as_u16(),
            elapsed_ms = elapsed.as_millis(),
            "upstream returned error"
        );
        return Err(format!(
            "{} returned HTTP {}: {}",
            candidate.provider, status, body
        ));
    }

    record_success(state, &candidate.provider, elapsed).await;
    info!(
        provider = candidate.provider,
        model = candidate.model,
        status = status.as_u16(),
        elapsed_ms = elapsed.as_millis(),
        "upstream selected"
    );

    let stream = upstream
        .bytes_stream()
        .map_err(|error| std::io::Error::other(error.to_string()));
    let mut response = Response::builder().status(status);
    if let Some(content_type) = response_headers.get(header::CONTENT_TYPE) {
        response = response.header(header::CONTENT_TYPE, content_type);
    }
    if let Some(cache_control) = response_headers.get(header::CACHE_CONTROL) {
        response = response.header(header::CACHE_CONTROL, cache_control);
    }

    response
        .body(Body::from_stream(stream))
        .map_err(|error| format!("failed to build response: {error}"))
}

fn authorize(config: &AppConfig, headers: &HeaderMap) -> Result<(), ApiError> {
    if config.server.gateway_api_keys.is_empty() {
        return Ok(());
    }

    let token = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .ok_or(ApiError::Unauthorized)?;

    if config
        .server
        .gateway_api_keys
        .iter()
        .any(|configured| constant_time_eq(configured.as_bytes(), token.as_bytes()))
    {
        Ok(())
    } else {
        Err(ApiError::Unauthorized)
    }
}

async fn record_success(state: &AppState, provider: &str, latency: Duration) {
    let mut stats = state.stats.write().await;
    let stat = stats.entry(provider.to_string()).or_default();
    stat.requests += 1;
    let latency_ms = latency.as_secs_f64() * 1000.0;
    stat.latency_ewma_ms = Some(match stat.latency_ewma_ms {
        Some(current) => current * 0.8 + latency_ms * 0.2,
        None => latency_ms,
    });
}

async fn record_failure(state: &AppState, provider: &str) {
    let mut stats = state.stats.write().await;
    let stat = stats.entry(provider.to_string()).or_default();
    stat.requests += 1;
    stat.failures += 1;
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }

    left.iter()
        .zip(right.iter())
        .fold(0u8, |acc, (a, b)| acc | (a ^ b))
        == 0
}

fn escape_label(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}
