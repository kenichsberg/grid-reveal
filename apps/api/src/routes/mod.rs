//! Router assembly.

pub mod auth;
pub mod entries;
pub mod goals;
pub mod pictures;
pub mod screens;
pub mod users;

use axum::http::{header, HeaderValue, Method};
use axum::routing::get;
use axum::{Json, Router};
use tower_http::cors::CorsLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::trace::TraceLayer;

use crate::images;
use crate::state::AppState;

pub fn app(state: AppState) -> Router {
    let cors = build_cors(&state);

    Router::new()
        .route("/health", get(health))
        .merge(auth::router())
        .merge(users::router())
        .merge(pictures::router())
        .merge(screens::router())
        .merge(entries::router())
        .merge(goals::router())
        // Generous enough for a 10 MB image plus multipart framing; the image
        // pipeline enforces the real limit with a clearer error.
        .layer(RequestBodyLimitLayer::new(images::MAX_UPLOAD_BYTES + 1024 * 1024))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// Credentialed CORS cannot use a wildcard origin, so the allowed origins are
/// listed explicitly in configuration.
fn build_cors(state: &AppState) -> CorsLayer {
    let origins: Vec<HeaderValue> = state
        .config
        .cors_allowed_origins
        .iter()
        .filter_map(|o| o.parse::<HeaderValue>().ok())
        .collect();

    if origins.is_empty() {
        tracing::warn!(
            "CORS_ALLOWED_ORIGINS is empty; browser clients on another origin will be refused"
        );
    }

    CorsLayer::new()
        .allow_origin(origins)
        .allow_credentials(true)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([header::CONTENT_TYPE, header::ACCEPT])
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok" }))
}
