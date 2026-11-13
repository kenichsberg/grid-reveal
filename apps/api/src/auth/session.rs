//! Server-side sessions.
//!
//! The cookie carries an opaque random token, not a signed claim. That costs a
//! database lookup per request — which we were making anyway — and buys
//! immediate revocation, which an administrator password reset requires.

use axum_extra::extract::cookie::{Cookie, SameSite};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::{Duration, Utc};
use rand::RngCore;
use sqlx::PgPool;
use uuid::Uuid;

use crate::config::Config;
use crate::error::AppResult;
use crate::models::User;

pub const COOKIE_NAME: &str = "grid_session";

/// 256 bits of entropy, URL-safe so it survives a cookie unescaped.
fn generate_token() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

pub async fn create(pool: &PgPool, user_id: Uuid, ttl_days: i64) -> AppResult<String> {
    let token = generate_token();
    let expires_at = Utc::now() + Duration::days(ttl_days);

    sqlx::query("INSERT INTO sessions (token, user_id, expires_at) VALUES ($1, $2, $3)")
        .bind(&token)
        .bind(user_id)
        .bind(expires_at)
        .execute(pool)
        .await?;

    Ok(token)
}

/// Resolves a token to its user, treating expired sessions as absent.
pub async fn lookup(pool: &PgPool, token: &str) -> AppResult<Option<User>> {
    let user = sqlx::query_as::<_, User>(
        r#"
        SELECT u.id, u.username, u.email, u.password_hash, u.is_admin, u.created_at
        FROM sessions s
        JOIN users u ON u.id = s.user_id
        WHERE s.token = $1 AND s.expires_at > now()
        "#,
    )
    .bind(token)
    .fetch_optional(pool)
    .await?;

    Ok(user)
}

pub async fn delete(pool: &PgPool, token: &str) -> AppResult<()> {
    sqlx::query("DELETE FROM sessions WHERE token = $1")
        .bind(token)
        .execute(pool)
        .await?;
    Ok(())
}

/// Revokes every session for a user. Used on administrator password reset.
pub async fn delete_all_for_user(pool: &PgPool, user_id: Uuid) -> AppResult<u64> {
    let result = sqlx::query("DELETE FROM sessions WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

/// Revokes a user's other sessions, keeping the one in hand. Used when a user
/// changes their own password: they stay logged in here, everywhere else drops.
pub async fn delete_others_for_user(
    pool: &PgPool,
    user_id: Uuid,
    keep_token: &str,
) -> AppResult<u64> {
    let result = sqlx::query("DELETE FROM sessions WHERE user_id = $1 AND token <> $2")
        .bind(user_id)
        .bind(keep_token)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

/// Removes expired rows. Called periodically; sessions are also filtered by
/// expiry on read, so this is housekeeping rather than enforcement.
pub async fn purge_expired(pool: &PgPool) -> AppResult<u64> {
    let result = sqlx::query("DELETE FROM sessions WHERE expires_at <= now()")
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

/// Builds the session cookie.
///
/// `SameSite=Lax` on a domain shared by the web and API hosts gives CSRF
/// resistance without a separate token. It is why the deployment needs a real
/// domain rather than `*.pages.dev` against a bare VM address.
pub fn build_cookie(config: &Config, token: String) -> Cookie<'static> {
    let mut cookie = Cookie::new(COOKIE_NAME, token);
    cookie.set_http_only(true);
    cookie.set_secure(config.cookie_secure);
    cookie.set_same_site(SameSite::Lax);
    cookie.set_path("/");
    cookie.set_max_age(time::Duration::days(config.session_ttl_days));
    if let Some(domain) = &config.cookie_domain {
        cookie.set_domain(domain.clone());
    }
    cookie
}

/// Matching cookie with an immediate expiry, so the browser drops it.
pub fn build_removal_cookie(config: &Config) -> Cookie<'static> {
    let mut cookie = Cookie::new(COOKIE_NAME, "");
    cookie.set_http_only(true);
    cookie.set_secure(config.cookie_secure);
    cookie.set_same_site(SameSite::Lax);
    cookie.set_path("/");
    cookie.set_max_age(time::Duration::seconds(0));
    if let Some(domain) = &config.cookie_domain {
        cookie.set_domain(domain.clone());
    }
    cookie
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_are_unique_and_url_safe() {
        let a = generate_token();
        let b = generate_token();
        assert_ne!(a, b);
        assert!(a.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
        // 32 bytes base64url without padding.
        assert_eq!(a.len(), 43);
    }
}
