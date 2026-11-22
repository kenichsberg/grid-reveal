//! The skew, end to end, against a real database.
//!
//! Each test puts the server's UTC day and the caller's day on opposite sides
//! of a boundary and drives the real router, because the bug these guard
//! against lived in the wiring rather than in any one function: every piece was
//! correct given a date, and the call sites supplied the wrong one.
//!
//! Needs PostgreSQL. `docker compose up -d db` provides it; the suite skips
//! with a notice when it is absent, so a machine without one still runs the
//! rest of the tests.

use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::extract::Request;
use axum::http::StatusCode;
use axum::Router;
use chrono::{Duration as Days, NaiveDate, Utc};
use grid_reveal_api::auth::extract::CLIENT_DATE_HEADER;
use grid_reveal_api::auth::session;
use grid_reveal_api::config::{Config, StorageConfig};
use grid_reveal_api::state::AppState;
use grid_reveal_api::storage::Storage;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

const DEFAULT_URL: &str = "postgres://grid:grid@127.0.0.1:55432/grid_reveal";

/// A screen, a member, and a live session, isolated from every other test by
/// unique identifiers rather than by a transaction: the router takes its own
/// connections from the pool, so a wrapping transaction would be invisible to
/// it.
struct Fixture {
    app: Router,
    slug: String,
    token: String,
}

async fn pool() -> Option<PgPool> {
    let url = std::env::var("TEST_DATABASE_URL").unwrap_or_else(|_| DEFAULT_URL.to_string());

    let pool = PgPoolOptions::new()
        .max_connections(4)
        .acquire_timeout(Duration::from_secs(3))
        .connect(&url)
        .await;

    match pool {
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

/// A screen whose period is whatever the test needs, with one member in it.
async fn fixture(pool: &PgPool, starts_on: NaiveDate, ends_on: NaiveDate) -> Fixture {
    let tag = Uuid::new_v4().simple().to_string();
    let username = format!("m{}", &tag[..8]);
    let slug = format!("s{}", &tag[..8]);

    let (user_id,): (Uuid,) = sqlx::query_as(
        "INSERT INTO users (username, password_hash, is_admin)
         VALUES ($1, 'not-a-real-hash', FALSE) RETURNING id",
    )
    .bind(&username)
    .fetch_one(pool)
    .await
    .expect("insert user");

    let (picture_id,): (Uuid,) = sqlx::query_as(
        "INSERT INTO pictures (storage_key, width, height, uploaded_by)
         VALUES ($1, 800, 600, $2) RETURNING id",
    )
    .bind(format!("test/{tag}.webp"))
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("insert picture");

    let (screen_id,): (Uuid,) = sqlx::query_as(
        "INSERT INTO screens
           (slug, title, picture_id, cols, rows, unit_minutes, permutation,
            created_by, starts_on, ends_on)
         VALUES ($1, 'Boundary', $2, 2, 2, 5, ARRAY[0,1,2,3], $3, $4, $5)
         RETURNING id",
    )
    .bind(&slug)
    .bind(picture_id)
    .bind(user_id)
    .bind(starts_on)
    .bind(ends_on)
    .fetch_one(pool)
    .await
    .expect("insert screen");

    sqlx::query("INSERT INTO screen_members (screen_id, user_id) VALUES ($1, $2)")
        .bind(screen_id)
        .bind(user_id)
        .execute(pool)
        .await
        .expect("insert membership");

    let token = session::create(pool, user_id, 30).await.expect("session");

    Fixture {
        app: grid_reveal_api::routes::app(state_for(pool.clone())),
        slug,
        token,
    }
}

/// A request as the fixture's member, optionally declaring the caller's own day.
fn as_member(
    fixture: &Fixture,
    method: &str,
    path: &str,
    client_day: Option<NaiveDate>,
) -> axum::http::request::Builder {
    let mut request = Request::builder()
        .method(method)
        .uri(format!("/screens/{}{}", fixture.slug, path))
        .header("content-type", "application/json")
        .header(
            "cookie",
            format!("{}={}", session::COOKIE_NAME, fixture.token),
        );

    if let Some(day) = client_day {
        request = request.header(CLIENT_DATE_HEADER, day.to_string());
    }

    request
}

async fn run(fixture: &Fixture, request: Request) -> (StatusCode, String) {
    let response = fixture.app.clone().oneshot(request).await.expect("request");

    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), 64 * 1024)
        .await
        .expect("body");

    (status, String::from_utf8_lossy(&bytes).to_string())
}

/// Logs 30 minutes, optionally declaring the caller's own day.
async fn log_progress(
    fixture: &Fixture,
    occurred_at: NaiveDate,
    client_day: Option<NaiveDate>,
) -> (StatusCode, String) {
    let body = serde_json::json!({
        "hours": 0,
        "minutes": 30,
        "occurred_at": occurred_at.to_string(),
    });

    let request = as_member(fixture, "POST", "/entries", client_day)
        .body(Body::from(body.to_string()))
        .unwrap();

    run(fixture, request).await
}

/// Adds a goal. Goals share the write gate with entries but reach it through
/// their own handler, so that wiring is worth exercising on its own.
async fn add_goal(fixture: &Fixture, client_day: Option<NaiveDate>) -> (StatusCode, String) {
    let body = serde_json::json!({ "text": "finish the chapter" });

    let request = as_member(fixture, "POST", "/goals", client_day)
        .body(Body::from(body.to_string()))
        .unwrap();

    run(fixture, request).await
}

/// The phase the screen reports to this caller, read off the detail response.
async fn reported_phase(fixture: &Fixture, client_day: Option<NaiveDate>) -> String {
    let request = as_member(fixture, "GET", "", client_day)
        .body(Body::empty())
        .unwrap();

    let (status, body) = run(fixture, request).await;
    assert_eq!(status, StatusCode::OK, "{body}");

    serde_json::from_str::<serde_json::Value>(&body).expect("detail json")["phase"]
        .as_str()
        .expect("phase")
        .to_string()
}

fn server_day() -> NaiveDate {
    Utc::now().date_naive()
}

/// 4.1 — the caller's day is a day ahead of the server's, which is what every
/// member east of UTC sees for the first hours of their day. An entry dated
/// their today is the present, not the future.
#[tokio::test]
async fn an_entry_on_the_callers_day_is_not_the_future() {
    let Some(pool) = pool().await else { return };

    let caller_day = server_day() + Days::days(1);
    let fixture = fixture(&pool, server_day() - Days::days(10), caller_day).await;

    let (status, body) = log_progress(&fixture, caller_day, Some(caller_day)).await;
    assert_eq!(status, StatusCode::OK, "{body}");

    // Without the header the same request is the old bug, verbatim.
    let (status, body) = log_progress(&fixture, caller_day, None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body.contains("future date"), "{body}");
}

/// 4.2 — the period opens on the caller's day while the server is still on
/// yesterday. The screen has started for the person looking at it.
#[tokio::test]
async fn a_period_opening_today_accepts_the_callers_first_entry() {
    let Some(pool) = pool().await else { return };

    let caller_day = server_day() + Days::days(1);
    let fixture = fixture(&pool, caller_day, caller_day + Days::days(10)).await;

    let (status, body) = log_progress(&fixture, caller_day, Some(caller_day)).await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let (status, body) = log_progress(&fixture, caller_day, None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body.contains("has not started"), "{body}");
}

/// 4.3 — the mirror image, west of UTC: the server has rolled over to tomorrow
/// while the member is still in the final evening of the period.
#[tokio::test]
async fn the_last_day_of_a_period_is_the_callers_last_day() {
    let Some(pool) = pool().await else { return };

    let caller_day = server_day() - Days::days(1);
    let fixture = fixture(&pool, caller_day - Days::days(10), caller_day).await;

    let (status, body) = log_progress(&fixture, caller_day, Some(caller_day)).await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let (status, body) = log_progress(&fixture, caller_day, None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body.contains("ended"), "{body}");
}

/// The bound, where it actually matters: a claimed date cannot reopen a screen
/// whose period closed long ago.
#[tokio::test]
async fn a_long_closed_period_cannot_be_reopened_by_claiming_a_date() {
    let Some(pool) = pool().await else { return };

    let closed_on = server_day() - Days::days(30);
    let fixture = fixture(&pool, closed_on - Days::days(10), closed_on).await;

    let (status, body) = log_progress(&fixture, closed_on, Some(closed_on)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert!(body.contains("ended"), "{body}");
}

/// 4.2, the other half: a goal is gated by the same day as an entry, and the
/// goals route resolves it through its own extractor.
#[tokio::test]
async fn a_goal_is_accepted_on_the_callers_first_day() {
    let Some(pool) = pool().await else { return };

    let caller_day = server_day() + Days::days(1);
    let fixture = fixture(&pool, caller_day, caller_day + Days::days(10)).await;

    let (status, body) = add_goal(&fixture, Some(caller_day)).await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let (status, body) = add_goal(&fixture, None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body.contains("has not started"), "{body}");
}

/// The read path, which the write tests do not touch: a phase is derived for
/// whoever asked, and the label a member is shown agrees with what they may do.
/// Two members a day apart genuinely see different phases here.
#[tokio::test]
async fn the_phase_a_screen_reports_is_the_callers_own() {
    let Some(pool) = pool().await else { return };

    let caller_day = server_day() - Days::days(1);
    let fixture = fixture(&pool, caller_day - Days::days(10), caller_day).await;

    assert_eq!(reported_phase(&fixture, Some(caller_day)).await, "active");
    assert_eq!(reported_phase(&fixture, None).await, "ended");

    // The label is not decoration: the caller told "active" can write.
    let (status, body) = log_progress(&fixture, caller_day, Some(caller_day)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
}
