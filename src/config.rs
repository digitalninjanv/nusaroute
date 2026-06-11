use std::{collections::HashMap, env, fs, net::SocketAddr, path::Path};

use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub providers: HashMap<String, ProviderConfig>,
    #[serde(default)]
    pub models: HashMap<String, ModelRoute>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_bind")]
    pub bind: SocketAddr,
    #[serde(default = "default_request_timeout_ms")]
    pub request_timeout_ms: u64,
    #[serde(default)]
    pub gateway_api_keys: Vec<String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind: default_bind(),
            request_timeout_ms: default_request_timeout_ms(),
            gateway_api_keys: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProviderConfig {
    #[serde(rename = "type")]
    pub kind: ProviderKind,
    pub base_url: String,
    pub api_key_env: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_priority")]
    pub priority: u32,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ProviderKind {
    OpenaiCompatible,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ModelRoute {
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_strategy")]
    pub strategy: RouteStrategy,
    #[serde(default = "default_true")]
    pub fallback: bool,
    #[serde(default)]
    pub candidates: Vec<ModelCandidate>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RouteStrategy {
    LowestLatency,
    Priority,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ModelCandidate {
    pub provider: String,
    pub model: String,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to read config at {path}: {source}")]
    Read {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse YAML config at {path}: {source}")]
    Parse {
        path: String,
        #[source]
        source: serde_yaml::Error,
    },
    #[error("config has no models")]
    NoModels,
    #[error("model route '{model}' has no candidates")]
    EmptyRoute { model: String },
    #[error("model route '{model}' references unknown provider '{provider}'")]
    UnknownProvider { model: String, provider: String },
    #[error("provider '{provider}' expects missing environment variable '{env_var}'")]
    MissingProviderKey { provider: String, env_var: String },
}

impl AppConfig {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        let path_label = path.display().to_string();
        let contents = fs::read_to_string(path).map_err(|source| ConfigError::Read {
            path: path_label.clone(),
            source,
        })?;
        let config: AppConfig =
            serde_yaml::from_str(&contents).map_err(|source| ConfigError::Parse {
                path: path_label,
                source,
            })?;
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<(), ConfigError> {
        if self.models.is_empty() {
            return Err(ConfigError::NoModels);
        }

        for (model_name, route) in &self.models {
            if route.candidates.is_empty() {
                return Err(ConfigError::EmptyRoute {
                    model: model_name.clone(),
                });
            }

            for candidate in &route.candidates {
                let provider = self.providers.get(&candidate.provider).ok_or_else(|| {
                    ConfigError::UnknownProvider {
                        model: model_name.clone(),
                        provider: candidate.provider.clone(),
                    }
                })?;

                if provider.enabled && env::var(&provider.api_key_env).is_err() {
                    return Err(ConfigError::MissingProviderKey {
                        provider: candidate.provider.clone(),
                        env_var: provider.api_key_env.clone(),
                    });
                }
            }
        }

        Ok(())
    }
}

fn default_bind() -> SocketAddr {
    "127.0.0.1:1789"
        .parse()
        .expect("valid default bind address")
}

fn default_request_timeout_ms() -> u64 {
    45_000
}

fn default_strategy() -> RouteStrategy {
    RouteStrategy::LowestLatency
}

fn default_priority() -> u32 {
    100
}

fn default_true() -> bool {
    true
}
