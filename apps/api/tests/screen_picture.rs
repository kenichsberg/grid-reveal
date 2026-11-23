//! The screen list does not hand out a picture the board has not earned.
//!
//! A card that showed the picture — even dimmed — would disclose the one thing
//! the application exists to withhold, so `GET /screens` omits the URL until
//! every tile is revealed. The administrator directory is the exception:
//! administrators choose the pictures, so there is nothing there to spoil.
//!
//! Needs PostgreSQL. `docker compose up -d db` provides it; the suite skips
//! with a notice when it is absent.

use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::extract::Request;
use axum::http::StatusCode;
use axum::Router;
use chrono::{Duration as Days, Utc};
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
async fn admin(pool: &PgPool) -> (Admin, String) {
    let tag = Uuid::new_v4().simple().to_string();
    let storage_key = format!("test/{tag}.webp");

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
    .bind(&storage_key)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("insert picture");

    (
        Admin {
            app: grid_reveal_api::routes::app(state_for(pool.clone())),
            token: session::create(pool, user_id, 30).await.expect("session"),
            picture_id,
        },
        storage_key,
    )
}

async fn request(admin: &Admin, method: &str, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
    let builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .header("cookie", format!("{}={}", session::COOKIE_NAME, admin.token));

    let request = match body {
        Some(body) => builder.body(Body::from(body.to_string())).unwrap(),
        None => builder.body(Body::empty()).unwrap(),
    };

    let response = admin.app.clone().oneshot(request).await.expect("request");
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), 256 * 1024)
        .await
        .expect("body");

    let text = String::from_utf8_lossy(&bytes).to_string();
    let json = serde_json::from_str::<Value>(&text).unwrap_or(Value::String(text));
    (status, json)
}

/// The caller's own list, which is where the concealment lives.
async fn my_screens(admin: &Admin) -> Value {
    let (status, body) = request(admin, "GET", "/screens", None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body
}

/// The screen the list is about, however the directory is sorted.
fn row_for<'a>(rows: &'a Value, slug: &str) -> &'a Value {
    rows.as_array()
        .expect("array")
        .iter()
        .find(|row| row["slug"] == slug)
        .expect("the screen is listed")
}

#[tokio::test]
async fn the_list_withholds_the_picture_until_every_tile_is_revealed() {
    let Some(pool) = pool().await else { return };
    let (admin, storage_key) = admin(&pool).await;

    // A board small enough that one entry finishes it, inside a period that
    // covers the server's own day so entries are accepted.
    let today = Utc::now().date_naive();
    let (status, screen) = request(
        &admin,
        "POST",
        "/screens",
        Some(json!({
            "title": "Boundary",
            "picture_id": admin.picture_id,
            "target_total_minutes": 20,
            "unit_minutes": 5,
            "cols": 2,
            "starts_on": (today - Days::days(1)).to_string(),
            "ends_on": (today + Days::days(30)).to_string(),
            "auto_archive_days": null,
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{screen}");

    let slug = screen["slug"].as_str().expect("slug").to_string();
    let total_tiles = screen["total_tiles"].as_i64().expect("total_tiles");
    let unit_minutes = screen["unit_minutes"].as_i64().expect("unit_minutes");
    let to_finish = total_tiles * unit_minutes;
    assert!(
        to_finish <= 23 * 60 + 59,
        "the board must be finishable in one entry; {to_finish} minutes is not"
    );

    // Nothing logged yet: the card has nothing to show but its percentage.
    let screens = my_screens(&admin).await;
    let row = row_for(&screens["active"], &slug);
    assert_eq!(row["revealed_count"], 0);
    assert_eq!(
        row["picture_url"],
        Value::Null,
        "a covered board disclosed its picture: {row}"
    );

    // The card's one line beneath the title. The creating administrator is a
    // participant, not merely an operator, so a new screen has one.
    assert_eq!(row["participant_count"], 1, "{row}");

    // The administrator directory is a different question and keeps its
    // thumbnails throughout.
    let (status, directory) = request(&admin, "GET", "/admin/screens", None).await;
    assert_eq!(status, StatusCode::OK, "{directory}");
    assert!(
        row_for(&directory, &slug)["picture_url"]
            .as_str()
            .is_some_and(|url| url.contains(&storage_key)),
        "the directory lost its thumbnail: {directory}"
    );

    // Finish the board.
    let (status, body) = request(
        &admin,
        "POST",
        &format!("/screens/{slug}/entries"),
        Some(json!({
            "hours": to_finish / 60,
            "minutes": to_finish % 60,
            "occurred_at": null,
            "note": null,
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let screens = my_screens(&admin).await;
    let row = row_for(&screens["active"], &slug);
    assert_eq!(row["revealed_count"], Value::from(total_tiles));
    assert!(
        row["picture_url"]
            .as_str()
            .is_some_and(|url| url.contains(&storage_key)),
        "a finished board withheld its picture: {row}"
    );
}

/// A partly uncovered board is still a covered one.
///
/// The interesting case is not zero progress but most of it: the old card
/// faded its cover out as the percentage rose, so by halfway the photograph
/// was legible through it.
#[tokio::test]
async fn the_last_tile_is_the_one_that_discloses_the_picture() {
    let Some(pool) = pool().await else { return };
    let (admin, _) = admin(&pool).await;

    let today = Utc::now().date_naive();
    let (status, screen) = request(
        &admin,
        "POST",
        "/screens",
        Some(json!({
            "title": "Nearly",
            "picture_id": admin.picture_id,
            "target_total_minutes": 20,
            "unit_minutes": 5,
            "cols": 2,
            "starts_on": (today - Days::days(1)).to_string(),
            "ends_on": (today + Days::days(30)).to_string(),
            "auto_archive_days": null,
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{screen}");

    let slug = screen["slug"].as_str().expect("slug").to_string();
    let total_tiles = screen["total_tiles"].as_i64().expect("total_tiles");
    let unit_minutes = screen["unit_minutes"].as_i64().expect("unit_minutes");

    // One tile short of the whole board.
    let minutes = (total_tiles - 1) * unit_minutes;
    let (status, body) = request(
        &admin,
        "POST",
        &format!("/screens/{slug}/entries"),
        Some(json!({
            "hours": minutes / 60,
            "minutes": minutes % 60,
            "occurred_at": null,
            "note": null,
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let screens = my_screens(&admin).await;
    let row = row_for(&screens["active"], &slug);
    assert_eq!(row["revealed_count"], Value::from(total_tiles - 1));
    assert_eq!(
        row["picture_url"],
        Value::Null,
        "one tile short is still short: {row}"
    );
}
