//! What this installation calls itself.
//!
//! The name is the first thing here that is instance-wide and editable, and
//! the first setting a stranger may read: the sign-in page prints it to people
//! who have no session. So the two questions these tests ask are whether the
//! read is genuinely public and whether the write is genuinely not.
//!
//! The settings are one shared row, so these tests assert against what they
//! themselves read first rather than against the seeded name — another file,
//! or an earlier run, may have left it renamed.
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

struct World {
    app: Router,
    admin_token: String,
    member_token: String,
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

/// An administrator and an ordinary account, so that 401 and 403 are
/// distinguishable.
async fn world(pool: &PgPool) -> World {
    let tag = Uuid::new_v4().simple().to_string();

    let mut tokens = Vec::new();
    for (prefix, is_admin) in [("a", true), ("m", false)] {
        let (id,): (Uuid,) = sqlx::query_as(
            "INSERT INTO users (username, password_hash, is_admin)
             VALUES ($1, 'not-a-real-hash', $2) RETURNING id",
        )
        .bind(format!("{prefix}{}", &tag[..8]))
        .bind(is_admin)
        .fetch_one(pool)
        .await
        .expect("insert user");

        tokens.push(session::create(pool, id, 30).await.expect("session"));
    }

    World {
        app: grid_reveal_api::routes::app(state_for(pool.clone())),
        member_token: tokens.pop().expect("member"),
        admin_token: tokens.pop().expect("admin"),
    }
}

async fn run(world: &World, request: Request) -> (StatusCode, String) {
    let response = world.app.clone().oneshot(request).await.expect("request");
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), 64 * 1024)
        .await
        .expect("body");
    (status, String::from_utf8_lossy(&bytes).to_string())
}

fn builder(method: &str, uri: &str) -> axum::http::request::Builder {
    Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
}

async fn as_user(world: &World, token: &str, method: &str, body: Value) -> (StatusCode, String) {
    let request = builder(method, "/settings")
        .header("cookie", format!("{}={}", session::COOKIE_NAME, token))
        .body(Body::from(body.to_string()))
        .unwrap();
    run(world, request).await
}

/// Nobody: no session at all, which is how a visitor reaches the sign-in page.
async fn as_stranger(world: &World, method: &str, body: Value) -> (StatusCode, String) {
    let request = builder(method, "/settings")
        .body(Body::from(body.to_string()))
        .unwrap();
    run(world, request).await
}

async fn name_seen_by_stranger(world: &World) -> String {
    let (status, body) = as_stranger(world, "GET", json!({})).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    serde_json::from_str::<Value>(&body).unwrap()["app_name"]
        .as_str()
        .expect("app_name")
        .to_string()
}

#[tokio::test]
async fn a_stranger_may_read_the_name_and_an_administrator_may_change_it() {
    let Some(pool) = pool().await else { return };
    let world = world(&pool).await;

    // Readable with no session at all: the sign-in page needs it before
    // anyone can have one.
    let before = name_seen_by_stranger(&world).await;
    assert!(!before.is_empty());

    let renamed = format!("Daimoku {}", Uuid::new_v4().simple());
    let (status, body) = as_user(
        &world,
        &world.admin_token,
        "PATCH",
        json!({ "app_name": format!("  {renamed}  ") }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    // Trimmed on the way in, and the response is what was stored.
    let stored = serde_json::from_str::<Value>(&body).unwrap();
    assert_eq!(stored["app_name"], Value::from(renamed.clone()));
    assert_ne!(renamed, before, "the rename must actually change something");

    assert_eq!(name_seen_by_stranger(&world).await, renamed);

    let (status, body) = as_user(&world, &world.admin_token, "GET", json!({})).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        serde_json::from_str::<Value>(&body).unwrap()["app_name"],
        Value::from(renamed)
    );

    // Put it back. Every other row these suites leave behind is invisible
    // noise; this one would rename the development installation.
    let (status, body) = as_user(
        &world,
        &world.admin_token,
        "PATCH",
        json!({ "app_name": before }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
}

/// Every way of failing to rename it, and none of them leaves a mark.
///
/// What "no mark" means is that none of the refused names becomes the name —
/// not that the name is unchanged. The row is shared, so the test running
/// beside this one is entitled to rename it meanwhile.
#[tokio::test]
async fn a_rename_is_administrator_only_and_a_blank_name_is_refused() {
    let Some(pool) = pool().await else { return };
    let world = world(&pool).await;

    let attempts = [
        (
            StatusCode::UNAUTHORIZED,
            None,
            json!({ "app_name": "By nobody" }),
        ),
        (
            StatusCode::FORBIDDEN,
            Some(world.member_token.clone()),
            json!({ "app_name": "By a member" }),
        ),
        (
            StatusCode::BAD_REQUEST,
            Some(world.admin_token.clone()),
            json!({ "app_name": "   " }),
        ),
        (
            StatusCode::BAD_REQUEST,
            Some(world.admin_token.clone()),
            json!({ "app_name": "a".repeat(61) }),
        ),
    ];

    let mut refused = Vec::new();
    for (expected, token, body) in attempts {
        let (status, response) = match &token {
            Some(token) => as_user(&world, token, "PATCH", body.clone()).await,
            None => as_stranger(&world, "PATCH", body.clone()).await,
        };
        assert_eq!(status, expected, "{body} -> {response}");
        refused.push(body["app_name"].as_str().expect("app_name").trim().to_string());
    }

    let name = name_seen_by_stranger(&world).await;
    for attempt in refused {
        assert_ne!(name, attempt, "a refused rename took effect anyway");
    }
    assert!(!name.trim().is_empty(), "the name was emptied");
}
