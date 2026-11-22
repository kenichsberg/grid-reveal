//! A screen cannot exist without a period.
//!
//! The database enforces it with NOT NULL and the DTOs enforce it by type, so
//! these tests check the thing neither can: what a caller actually gets back
//! when it tries anyway, and that a rejected edit leaves the stored period
//! alone rather than half-applied.
//!
//! Needs PostgreSQL. `docker compose up -d db` provides it; the suite skips
//! with a notice when it is absent.

use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::extract::Request;
use axum::http::StatusCode;
use axum::Router;
use grid_reveal_api::auth::session;
use grid_reveal_api::config::{Config, StorageConfig};
use grid_reveal_api::state::AppState;
use grid_reveal_api::storage::Storage;
use serde_json::{json, Value};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

const DEFAULT_URL: &str = "postgres://grid:grid@127.0.0.1:55432/grid_reveal";

struct Admin {
    app: Router,
    token: String,
    picture_id: Uuid,
}

async fn pool() -> Option<PgPool> {
    let url = std::env::var("TEST_DATABASE_URL").unwrap_or_else(|_| DEFAULT_URL.to_string());

    match PgPoolOptions::new()
        .max_connections(4)
        .acquire_timeout(Duration::from_secs(3))
        .connect(&url)
        .await
    {
        Ok(pool) => {
            grid_reveal_api::db::migrate(&pool).await.expect("migrations");
            Some(pool)
        }
        Err(e) => {
            eprintln!("skipping: no test database at {url} ({e}); `docker compose up -d db`");
            None
        }
    }
}

fn state_for(pool: PgPool) -> AppState {
    AppState {
        db: pool,
        config: Arc::new(Config {
            database_url: String::new(),
            bind_addr: "127.0.0.1:0".into(),
            cookie_domain: None,
            cookie_secure: false,
            session_ttl_days: 30,
            cors_allowed_origins: vec!["http://localhost:5173".into()],
            admin_username: "admin".into(),
            admin_password: "admin-dev-password".into(),
            storage: StorageConfig::Local {
                dir: "/tmp/grid-reveal-test-pictures".into(),
            },
        }),
        storage: Arc::new(Storage::Local {
            dir: "/tmp/grid-reveal-test-pictures".into(),
        }),
    }
}

/// An administrator with a picture to build screens from.
async fn admin(pool: &PgPool) -> Admin {
    let tag = Uuid::new_v4().simple().to_string();

    let (user_id,): (Uuid,) = sqlx::query_as(
        "INSERT INTO users (username, password_hash, is_admin)
         VALUES ($1, 'not-a-real-hash', TRUE) RETURNING id",
    )
    .bind(format!("a{}", &tag[..8]))
    .fetch_one(pool)
    .await
    .expect("insert admin");

    let (picture_id,): (Uuid,) = sqlx::query_as(
        "INSERT INTO pictures (storage_key, width, height, uploaded_by)
         VALUES ($1, 800, 600, $2) RETURNING id",
    )
    .bind(format!("test/{tag}.webp"))
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("insert picture");

    Admin {
        app: grid_reveal_api::routes::app(state_for(pool.clone())),
        token: session::create(pool, user_id, 30).await.expect("session"),
        picture_id,
    }
}

async fn send(admin: &Admin, method: &str, uri: &str, body: Value) -> (StatusCode, String) {
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .header("cookie", format!("{}={}", session::COOKIE_NAME, admin.token))
        .body(Body::from(body.to_string()))
        .unwrap();

    let response = admin.app.clone().oneshot(request).await.expect("request");
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), 64 * 1024)
        .await
        .expect("body");

    (status, String::from_utf8_lossy(&bytes).to_string())
}

fn creation(admin: &Admin, window: Value) -> Value {
    let mut body = json!({
        "title": "Boundary",
        "picture_id": admin.picture_id,
        "target_total_minutes": 600,
        "unit_minutes": 5,
        "cols": null,
        "auto_archive_days": null,
    });
    for (k, v) in window.as_object().unwrap() {
        body[k] = v.clone();
    }
    body
}

/// Creates a screen with a period and returns its slug.
async fn screen_with_period(admin: &Admin) -> String {
    let (status, body) = send(
        admin,
        "POST",
        "/screens",
        creation(
            admin,
            json!({"starts_on": "2026-01-10", "ends_on": "2026-01-20"}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    serde_json::from_str::<Value>(&body).unwrap()["slug"]
        .as_str()
        .expect("slug")
        .to_string()
}

async fn period_of(admin: &Admin, slug: &str) -> (String, String) {
    let request = Request::builder()
        .method("GET")
        .uri(format!("/screens/{slug}"))
        .header("cookie", format!("{}={}", session::COOKIE_NAME, admin.token))
        .body(Body::empty())
        .unwrap();

    let response = admin.app.clone().oneshot(request).await.expect("request");
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 64 * 1024)
        .await
        .unwrap();
    let detail: Value = serde_json::from_slice(&bytes).unwrap();

    (
        detail["starts_on"].as_str().expect("starts_on").to_string(),
        detail["ends_on"].as_str().expect("ends_on").to_string(),
    )
}

/// 5.2 — a screen without an end date is not a screen.
#[tokio::test]
async fn creating_a_screen_without_an_end_date_is_refused() {
    let Some(pool) = pool().await else { return };
    let admin = admin(&pool).await;

    let (status, body) = send(
        &admin,
        "POST",
        "/screens",
        creation(&admin, json!({"starts_on": "2026-01-10"})),
    )
    .await;

    assert!(status.is_client_error(), "{status}: {body}");
    assert!(body.contains("ends_on"), "{body}");

    // And the same request with the date is accepted, so the refusal is about
    // the missing bound rather than anything else in the body.
    let (status, body) = send(
        &admin,
        "POST",
        "/screens",
        creation(
            &admin,
            json!({"starts_on": "2026-01-10", "ends_on": "2026-01-20"}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
}

/// 5.3 — a window that arrives with a bound missing is refused whole, rather
/// than clearing that bound as it used to.
#[tokio::test]
async fn a_window_missing_a_bound_is_refused_and_changes_nothing() {
    let Some(pool) = pool().await else { return };
    let admin = admin(&pool).await;
    let slug = screen_with_period(&admin).await;

    let (status, body) = send(
        &admin,
        "PATCH",
        &format!("/screens/{slug}"),
        json!({"window": {"starts_on": "2026-01-10", "auto_archive_days": null}}),
    )
    .await;
    assert!(status.is_client_error(), "{status}: {body}");
    assert!(body.contains("ends_on"), "{body}");

    assert_eq!(
        period_of(&admin, &slug).await,
        ("2026-01-10".to_string(), "2026-01-20".to_string()),
        "a refused edit must leave the stored period alone"
    );
}

/// An explicit null is the case the old API accepted as "clear this bound".
#[tokio::test]
async fn a_null_bound_is_refused_rather_than_clearing_it() {
    let Some(pool) = pool().await else { return };
    let admin = admin(&pool).await;
    let slug = screen_with_period(&admin).await;

    let (status, body) = send(
        &admin,
        "PATCH",
        &format!("/screens/{slug}"),
        json!({"window": {"starts_on": "2026-01-10", "ends_on": null, "auto_archive_days": null}}),
    )
    .await;
    assert!(status.is_client_error(), "{status}: {body}");

    assert_eq!(
        period_of(&admin, &slug).await,
        ("2026-01-10".to_string(), "2026-01-20".to_string())
    );
}

/// 5.4 — the two kinds of absence stay distinguishable: no window at all still
/// means "leave the period alone", which is how every other form on the admin
/// page saves without restating the dates.
#[tokio::test]
async fn an_edit_carrying_no_window_leaves_the_period_alone() {
    let Some(pool) = pool().await else { return };
    let admin = admin(&pool).await;
    let slug = screen_with_period(&admin).await;

    let (status, body) = send(
        &admin,
        "PATCH",
        &format!("/screens/{slug}"),
        json!({"title": "Renamed"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    assert_eq!(
        period_of(&admin, &slug).await,
        ("2026-01-10".to_string(), "2026-01-20".to_string())
    );
}

/// The period a screen is created with is the period it reports back.
#[tokio::test]
async fn a_new_screen_reports_the_period_it_was_given() {
    let Some(pool) = pool().await else { return };
    let admin = admin(&pool).await;
    let slug = screen_with_period(&admin).await;

    assert_eq!(
        period_of(&admin, &slug).await,
        ("2026-01-10".to_string(), "2026-01-20".to_string())
    );
}
