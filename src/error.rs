use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("missing or invalid gateway bearer token")]
    Unauthorized,
    #[error("request body must contain a string 'model' field")]
    MissingModel,
    #[error("unknown model alias '{0}'")]
    UnknownModel(String),
    #[error("no enabled provider candidate is available for model '{0}'")]
    NoEnabledProvider(String),
    #[error("all provider attempts failed: {0}")]
    UpstreamFailed(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match self {
            ApiError::Unauthorized => StatusCode::UNAUTHORIZED,
            ApiError::MissingModel | ApiError::UnknownModel(_) | ApiError::NoEnabledProvider(_) => {
                StatusCode::BAD_REQUEST
            }
            ApiError::UpstreamFailed(_) => StatusCode::BAD_GATEWAY,
        };

        let message = self.to_string();
        (
            status,
            Json(json!({
                "error": {
                    "message": message,
                    "type": "nusaroute_gateway_error"
                }
            })),
        )
            .into_response()
    }
}
