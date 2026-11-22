//! What the wire does with the caller's calendar day.
//!
//! The resolution rules are unit-tested beside the extractor. These are the
//! two things a unit test cannot see: that a real request's header reaches the
//! extractor at all, and that the CORS layer permits it — without which a
//! browser never sends the header and the whole mechanism silently reverts to
//! the server's UTC date.

use std::sync::Arc;

use axum::body::Body;
use axum::extract::Request;
use axum::http::{HeaderValue, Method, StatusCode};
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use chrono::Utc;
use grid_reveal_api::auth::extract::{ClientToday, CLIENT_DATE_HEADER};
use grid_reveal_api::config::{Config, StorageConfig};
use grid_reveal_api::state::AppState;
use grid_reveal_api::storage::Storage;
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;

const ECHO_HEADER: &str = "x-resolved-date";
const ORIGIN: &str = "http://localhost:5173";

/// Echoes the resolved day back in a header, so the assertion needs no body.
async fn echo(today: ClientToday) -> Response {
    let mut response = Response::new(Body::empty());
    response.headers_mut().insert(
        ECHO_HEADER,
        HeaderValue::from_str(&today.date().to_string()).unwrap(),
    );
    response
}

async fn resolved_with(header: Option<&str>) -> String {
    let app = Router::new().route("/", get(echo));

    let mut request = Request::builder().uri("/");
    if let Some(value) = header {
        request = request.header(CLIENT_DATE_HEADER, value);
    }

    let response = app
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap();

    response
        .headers()
        .get(ECHO_HEADER)
        .unwrap()
        .to_str()
        .unwrap()
        .to_string()
}

/// Relative to the server's own date, since that is what the bound is measured
/// against and what the fallback returns.
fn server_day_offset(days: i64) -> String {
    (Utc::now().date_naive() + chrono::Duration::days(days)).to_string()
}

#[tokio::test]
async fn the_header_reaches_the_extractor() {
    let tomorrow = server_day_offset(1);
    assert_eq!(resolved_with(Some(&tomorrow)).await, tomorrow);

    let yesterday = server_day_offset(-1);
    assert_eq!(resolved_with(Some(&yesterday)).await, yesterday);
}

#[tokio::test]
async fn a_request_without_the_header_gets_the_servers_day() {
    assert_eq!(resolved_with(None).await, server_day_offset(0));
}

#[tokio::test]
async fn a_date_beyond_the_bound_is_discarded() {
    assert_eq!(
        resolved_with(Some(&server_day_offset(2))).await,
        server_day_offset(0)
    );
    assert_eq!(
        resolved_with(Some(&server_day_offset(-30))).await,
        server_day_offset(0)
    );
}

#[tokio::test]
async fn a_malformed_header_is_discarded_rather_than_rejected() {
    assert_eq!(resolved_with(Some("not-a-date")).await, server_day_offset(0));
}

/// Builds the real router. The pool is lazy and never connects: a CORS
/// preflight is answered by the layer, before any handler or query runs.
fn app_without_a_database() -> Router {
    let state = AppState {
        db: PgPoolOptions::new()
            .connect_lazy("postgres://unused:unused@127.0.0.1:1/unused")
            .unwrap(),
        config: Arc::new(Config {
            database_url: "postgres://unused".into(),
            bind_addr: "127.0.0.1:0".into(),
            cookie_domain: None,
            cookie_secure: false,
            session_ttl_days: 30,
            cors_allowed_origins: vec![ORIGIN.into()],
            admin_username: "admin".into(),
            admin_password: "admin-dev-password".into(),
            storage: StorageConfig::Local {
                dir: "/tmp/grid-reveal-test-pictures".into(),
            },
        }),
        storage: Arc::new(Storage::Local {
            dir: "/tmp/grid-reveal-test-pictures".into(),
        }),
    };

    grid_reveal_api::routes::app(state)
}

#[tokio::test]
async fn cors_permits_the_client_date_header() {
    let response = app_without_a_database()
        .oneshot(
            Request::builder()
                .method(Method::OPTIONS)
                .uri("/screens")
                .header("origin", ORIGIN)
                .header("access-control-request-method", "GET")
                .header("access-control-request-headers", CLIENT_DATE_HEADER)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let allowed = response
        .headers()
        .get("access-control-allow-headers")
        .expect("preflight must state which headers are allowed")
        .to_str()
        .unwrap()
        .to_lowercase();

    assert!(
        allowed.contains(CLIENT_DATE_HEADER),
        "a browser will not send {CLIENT_DATE_HEADER} unless the preflight allows it; got {allowed:?}"
    );
}
