//! Invite links, end to end.
//!
//! The parts worth testing here are the ones no unit test reaches: that an
//! invite is spent exactly once, that a failure on the way through leaves it
//! unspent, and that its life really is the screen's life rather than a
//! column someone has to remember to update.
//!
//! Needs PostgreSQL. `docker compose up -d db` provides it; the suite skips
//! with a notice when it is absent.

use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::extract::Request;
use axum::http::StatusCode;
use axum::Router;
use chrono::{Duration as Days, NaiveDate, Utc};
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
    pool: PgPool,
    admin_token: String,
    slug: String,
    screen_id: Uuid,
}

async fn pool() -> Option<PgPool> {
    let url = std::env::var("TEST_DATABASE_URL").unwrap_or_else(|_| DEFAULT_URL.to_string());

    match PgPoolOptions::new()
        .max_connections(6)
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

/// A username nothing else in the database will collide with.
fn name(prefix: &str) -> String {
    format!("{prefix}{}", &Uuid::new_v4().simple().to_string()[..8])
}

fn today() -> NaiveDate {
    Utc::now().date_naive()
}

/// An administrator and a screen whose period is whatever the test needs.
async fn world(pool: &PgPool, starts_on: NaiveDate, ends_on: NaiveDate) -> World {
    let tag = Uuid::new_v4().simple().to_string();

    let (admin_id,): (Uuid,) = sqlx::query_as(
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
    .bind(admin_id)
    .fetch_one(pool)
    .await
    .expect("insert picture");

    let slug = format!("s{}", &tag[..8]);
    let (screen_id,): (Uuid,) = sqlx::query_as(
        "INSERT INTO screens
           (slug, title, picture_id, cols, rows, unit_minutes, permutation,
            created_by, starts_on, ends_on)
         VALUES ($1, 'Invited', $2, 2, 2, 5, ARRAY[0,1,2,3], $3, $4, $5)
         RETURNING id",
    )
    .bind(&slug)
    .bind(picture_id)
    .bind(admin_id)
    .bind(starts_on)
    .bind(ends_on)
    .fetch_one(pool)
    .await
    .expect("insert screen");

    World {
        app: grid_reveal_api::routes::app(state_for(pool.clone())),
        pool: pool.clone(),
        admin_token: session::create(pool, admin_id, 30).await.expect("session"),
        slug,
        screen_id,
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

async fn as_admin(world: &World, method: &str, uri: &str, body: Value) -> (StatusCode, String) {
    let request = builder(method, uri)
        .header(
            "cookie",
            format!("{}={}", session::COOKIE_NAME, world.admin_token),
        )
        .body(Body::from(body.to_string()))
        .unwrap();
    run(world, request).await
}

/// Nobody: no session at all, which is how the invited person arrives.
async fn as_stranger(world: &World, method: &str, uri: &str, body: Value) -> (StatusCode, String) {
    let request = builder(method, uri)
        .body(Body::from(body.to_string()))
        .unwrap();
    run(world, request).await
}

async fn new_invite(world: &World) -> String {
    let (status, body) = as_admin(
        world,
        "POST",
        &format!("/screens/{}/invites", world.slug),
        json!({ "label": "for Mika" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    serde_json::from_str::<Value>(&body).unwrap()["token"]
        .as_str()
        .expect("token")
        .to_string()
}

async fn peek(world: &World, token: &str) -> (StatusCode, Value) {
    let (status, body) = as_stranger(world, "GET", &format!("/invites/{token}"), json!({})).await;
    let parsed = serde_json::from_str::<Value>(&body).unwrap_or(Value::Null);
    (status, parsed)
}

async fn accept(world: &World, token: &str, username: &str) -> (StatusCode, String) {
    as_stranger(
        world,
        "POST",
        &format!("/invites/{token}/accept"),
        json!({ "username": username, "password": "a-good-enough-password" }),
    )
    .await
}

async fn member_count(world: &World) -> i64 {
    let (count,): (i64,) =
        sqlx::query_as("SELECT count(*)::bigint FROM screen_members WHERE screen_id = $1")
            .bind(world.screen_id)
            .fetch_one(&world.pool)
            .await
            .unwrap();
    count
}

async fn user_exists(world: &World, username: &str) -> bool {
    let row: Option<(Uuid,)> = sqlx::query_as("SELECT id FROM users WHERE username = $1")
        .bind(username)
        .fetch_optional(&world.pool)
        .await
        .unwrap();
    row.is_some()
}

/// 6.1 — the whole path, which is the thing the feature exists to do.
#[tokio::test]
async fn an_invite_turns_a_stranger_into_a_member() {
    let Some(pool) = pool().await else { return };
    let world = world(&pool, today(), today() + Days::days(30)).await;
    let token = new_invite(&world).await;

    let (status, peeked) = peek(&world, &token).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(peeked["screen_title"], "Invited");
    assert_eq!(peeked["open"], true);
    assert_eq!(peeked["closed_reason"], Value::Null);
    assert_eq!(peeked["viewer_is_signed_in"], false);

    let before = member_count(&world).await;
    let username = name("n");
    let (status, body) = accept(&world, &token, &username).await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let parsed: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(parsed["user"]["username"], username.as_str());
    assert_eq!(parsed["user"]["is_admin"], false, "invites never grant admin");
    assert_eq!(parsed["slug"], world.slug.as_str());
    assert_eq!(member_count(&world).await, before + 1);
}

/// 6.2 — single use.
#[tokio::test]
async fn an_invite_admits_one_person_only() {
    let Some(pool) = pool().await else { return };
    let world = world(&pool, today(), today() + Days::days(30)).await;
    let token = new_invite(&world).await;

    let first = name("f");
    let (status, body) = accept(&world, &token, &first).await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let second = name("s");
    let (status, body) = accept(&world, &token, &second).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert!(body.contains("already been used"), "{body}");
    assert!(!user_exists(&world, &second).await, "no second account");
}

/// 6.3 — the failure a real person is most likely to hit. A typo must not
/// cost them the invitation.
#[tokio::test]
async fn a_taken_username_leaves_the_invite_open() {
    let Some(pool) = pool().await else { return };
    let world = world(&pool, today(), today() + Days::days(30)).await;
    let token = new_invite(&world).await;

    let taken = name("t");
    sqlx::query("INSERT INTO users (username, password_hash, is_admin) VALUES ($1, 'x', FALSE)")
        .bind(&taken)
        .execute(&pool)
        .await
        .unwrap();

    let (status, body) = accept(&world, &token, &taken).await;
    assert!(status.is_client_error(), "{status}: {body}");

    let (_, peeked) = peek(&world, &token).await;
    assert_eq!(peeked["open"], true, "the invite must survive a typo");

    let second_try = name("u");
    let (status, body) = accept(&world, &token, &second_try).await;
    assert_eq!(status, StatusCode::OK, "{body}");
}

/// A short password is the other way to fail on the way through.
#[tokio::test]
async fn a_rejected_password_leaves_the_invite_open() {
    let Some(pool) = pool().await else { return };
    let world = world(&pool, today(), today() + Days::days(30)).await;
    let token = new_invite(&world).await;

    let (status, body) = as_stranger(
        &world,
        "POST",
        &format!("/invites/{token}/accept"),
        json!({ "username": "shortpw", "password": "x" }),
    )
    .await;
    assert!(status.is_client_error(), "{status}: {body}");

    let (_, peeked) = peek(&world, &token).await;
    assert_eq!(peeked["open"], true);
    assert!(!user_exists(&world, "shortpw").await);
}

/// 6.4 — the invite's life is the screen's life, at both ends.
#[tokio::test]
async fn an_invite_is_open_before_the_period_and_closed_after_it() {
    let Some(pool) = pool().await else { return };

    let upcoming = world(&pool, today() + Days::days(7), today() + Days::days(30)).await;
    let token = new_invite(&upcoming).await;
    let (_, peeked) = peek(&upcoming, &token).await;
    assert_eq!(peeked["open"], true, "invitations go out before a start date");

    let ended = world(&pool, today() - Days::days(30), today() - Days::days(1)).await;
    let token = new_invite(&ended).await;
    let (_, peeked) = peek(&ended, &token).await;
    assert_eq!(peeked["open"], false);
    assert!(peeked["closed_reason"]
        .as_str()
        .unwrap()
        .contains("period has ended"));

    let before = member_count(&ended).await;
    let (status, body) = accept(&ended, &token, "toolate").await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(member_count(&ended).await, before);
    assert!(!user_exists(&ended, "toolate").await);
}

/// 6.5 — the consequence of deriving rather than storing: moving the deadline
/// moves the invites with it.
#[tokio::test]
async fn extending_a_closed_screen_reopens_its_unused_invites() {
    let Some(pool) = pool().await else { return };
    let world = world(&pool, today() - Days::days(30), today() - Days::days(1)).await;
    let token = new_invite(&world).await;

    let (_, peeked) = peek(&world, &token).await;
    assert_eq!(peeked["open"], false);

    let (status, body) = as_admin(
        &world,
        "PATCH",
        &format!("/screens/{}", world.slug),
        json!({"window": {
            "starts_on": (today() - Days::days(30)).to_string(),
            "ends_on": (today() + Days::days(7)).to_string(),
            "auto_archive_days": null
        }}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let (_, peeked) = peek(&world, &token).await;
    assert_eq!(peeked["open"], true, "the invite's life was never its own");
}

/// 6.6 — revoking.
#[tokio::test]
async fn a_revoked_invite_admits_nobody_but_an_accepted_one_is_untouchable() {
    let Some(pool) = pool().await else { return };
    let world = world(&pool, today(), today() + Days::days(30)).await;

    let pending = new_invite(&world).await;
    let (_, body) = as_admin(
        &world,
        "GET",
        &format!("/screens/{}/invites", world.slug),
        json!({}),
    )
    .await;
    let id = serde_json::from_str::<Value>(&body).unwrap()[0]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let (status, body) = as_admin(
        &world,
        "DELETE",
        &format!("/screens/{}/invites/{id}", world.slug),
        json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let (_, peeked) = peek(&world, &pending).await;
    assert_eq!(peeked["open"], false);
    assert!(peeked["closed_reason"]
        .as_str()
        .unwrap()
        .contains("withdrawn"));

    let (status, _) = accept(&world, &pending, "revokedout").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(!user_exists(&world, "revokedout").await);

    // An accepted invite is a record, not a lever: revoking it must not
    // unmake the member.
    let spent = new_invite(&world).await;
    let joined = name("j");
    let (status, _) = accept(&world, &spent, &joined).await;
    assert_eq!(status, StatusCode::OK);
    let members = member_count(&world).await;

    let (_, body) = as_admin(
        &world,
        "GET",
        &format!("/screens/{}/invites", world.slug),
        json!({}),
    )
    .await;
    let list: Value = serde_json::from_str(&body).unwrap();
    let spent_id = list
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["accepted_by"] == joined.as_str())
        .expect("the accepted invite names who accepted it")["id"]
        .as_str()
        .unwrap()
        .to_string();

    as_admin(
        &world,
        "DELETE",
        &format!("/screens/{}/invites/{spent_id}", world.slug),
        json!({}),
    )
    .await;

    assert_eq!(member_count(&world).await, members, "the member stays");
    assert!(user_exists(&world, &joined).await);
}

/// 6.7 — a link for one run must not open the next one.
#[tokio::test]
async fn a_clone_inherits_no_invites() {
    let Some(pool) = pool().await else { return };
    let world = world(&pool, today(), today() + Days::days(30)).await;
    new_invite(&world).await;

    let (picture_id,): (Uuid,) = sqlx::query_as("SELECT picture_id FROM screens WHERE id = $1")
        .bind(world.screen_id)
        .fetch_one(&pool)
        .await
        .unwrap();

    let (status, body) = as_admin(
        &world,
        "POST",
        &format!("/screens/{}/clone", world.slug),
        json!({
            "title": "Next run",
            "picture_id": picture_id,
            "starts_on": today().to_string(),
            "ends_on": (today() + Days::days(30)).to_string(),
            "auto_archive_days": null
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let clone_slug = serde_json::from_str::<Value>(&body).unwrap()["slug"]
        .as_str()
        .unwrap()
        .to_string();

    let (status, body) = as_admin(
        &world,
        "GET",
        &format!("/screens/{clone_slug}/invites"),
        json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        serde_json::from_str::<Value>(&body).unwrap().as_array().unwrap().len(),
        0,
        "a clone is a new run, and the old links must not open it"
    );
}

/// 6.8 — looking is free.
#[tokio::test]
async fn peeking_never_spends_an_invite() {
    let Some(pool) = pool().await else { return };
    let world = world(&pool, today(), today() + Days::days(30)).await;
    let token = new_invite(&world).await;

    for _ in 0..5 {
        let (status, peeked) = peek(&world, &token).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(peeked["open"], true);
    }

    let username = name("p");
    let (status, body) = accept(&world, &token, &username).await;
    assert_eq!(status, StatusCode::OK, "{body}");
}

/// 6.9 — an unknown token discloses nothing, from either public endpoint.
#[tokio::test]
async fn an_unknown_token_is_a_404_from_both_public_endpoints() {
    let Some(pool) = pool().await else { return };
    let world = world(&pool, today(), today() + Days::days(30)).await;

    let (status, _) = peek(&world, "nosuchtokenatallxxxxxxxx").await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, _) = accept(&world, "nosuchtokenatallxxxxxxxx", "ghost").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(!user_exists(&world, "ghost").await);
}

/// Accepting while signed in joins that account rather than making a second,
/// which is what keeps a member from ending up with two.
#[tokio::test]
async fn a_signed_in_visitor_joins_with_the_account_they_have() {
    let Some(pool) = pool().await else { return };
    let world = world(&pool, today(), today() + Days::days(30)).await;
    let token = new_invite(&world).await;

    let existing = name("e");
    let (user_id,): (Uuid,) = sqlx::query_as(
        "INSERT INTO users (username, password_hash, is_admin)
         VALUES ($1, 'not-a-real-hash', FALSE) RETURNING id",
    )
    .bind(&existing)
    .fetch_one(&pool)
    .await
    .unwrap();
    let their_token = session::create(&pool, user_id, 30).await.unwrap();

    let before = member_count(&world).await;
    let request = builder("POST", &format!("/invites/{token}/accept"))
        .header(
            "cookie",
            format!("{}={}", session::COOKIE_NAME, their_token),
        )
        .body(Body::from(json!({}).to_string()))
        .unwrap();
    let (status, body) = run(&world, request).await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let parsed: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(parsed["user"]["username"], existing.as_str());
    assert_eq!(member_count(&world).await, before + 1);

    let (count,): (i64,) = sqlx::query_as("SELECT count(*)::bigint FROM users WHERE username = $1")
        .bind(&existing)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1, "no second account for the same person");
}

/// A screen an administrator cannot see has no invites to speak of.
#[tokio::test]
async fn invites_are_administrator_only() {
    let Some(pool) = pool().await else { return };
    let world = world(&pool, today(), today() + Days::days(30)).await;

    let (member_id,): (Uuid,) = sqlx::query_as(
        "INSERT INTO users (username, password_hash, is_admin)
         VALUES ($1, 'not-a-real-hash', FALSE) RETURNING id",
    )
    .bind(name("m"))
    .fetch_one(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO screen_members (screen_id, user_id) VALUES ($1, $2)")
        .bind(world.screen_id)
        .bind(member_id)
        .execute(&pool)
        .await
        .unwrap();
    let member_token = session::create(&pool, member_id, 30).await.unwrap();

    for (method, uri) in [
        ("GET", format!("/screens/{}/invites", world.slug)),
        ("POST", format!("/screens/{}/invites", world.slug)),
    ] {
        let request = builder(method, &uri)
            .header(
                "cookie",
                format!("{}={}", session::COOKIE_NAME, member_token),
            )
            .body(Body::from(json!({"label": null}).to_string()))
            .unwrap();
        let (status, body) = run(&world, request).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{method} {uri}: {body}");
    }
}
