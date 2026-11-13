//! Login, logout, identity, and self-service password change.

use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use axum_extra::extract::CookieJar;

use crate::auth::extract::AuthUser;
use crate::auth::{password, session};
use crate::dto::{ChangePasswordRequest, CurrentUser, LoginRequest};
use crate::error::{AppError, AppResult};
use crate::models::User;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
        .route("/auth/me", get(me))
        .route("/auth/password", post(change_password))
}

fn to_current_user(user: &User) -> CurrentUser {
    CurrentUser {
        id: user.id,
        username: user.username.clone(),
        email: user.email.clone(),
        is_admin: user.is_admin,
    }
}

/// Authenticates and establishes a session.
///
/// An unknown username and a wrong password produce the same error and do the
/// same hashing work, so the two cannot be distinguished by response or timing.
async fn login(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(body): Json<LoginRequest>,
) -> AppResult<(CookieJar, Json<CurrentUser>)> {
    let username = body.username.trim();

    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE lower(username) = lower($1)")
        .bind(username)
        .fetch_optional(&state.db)
        .await?;

    let Some(user) = user else {
        password::verify_against_dummy(&body.password);
        return Err(AppError::Unauthorized);
    };

    if !password::verify_password(&body.password, &user.password_hash) {
        return Err(AppError::Unauthorized);
    }

    let token = session::create(&state.db, user.id, state.config.session_ttl_days).await?;
    let jar = jar.add(session::build_cookie(&state.config, token));

    Ok((jar, Json(to_current_user(&user))))
}

async fn logout(
    State(state): State<AppState>,
    jar: CookieJar,
) -> AppResult<(CookieJar, Json<serde_json::Value>)> {
    if let Some(cookie) = jar.get(session::COOKIE_NAME) {
        session::delete(&state.db, cookie.value()).await?;
    }
    let jar = jar.add(session::build_removal_cookie(&state.config));
    Ok((jar, Json(serde_json::json!({ "ok": true }))))
}

async fn me(auth: AuthUser) -> Json<CurrentUser> {
    Json(to_current_user(&auth.user))
}

/// Changes the caller's own password, which requires proving the current one.
///
/// Other sessions are revoked; this one survives, so changing a password does
/// not log you out of the tab you changed it in.
async fn change_password(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<ChangePasswordRequest>,
) -> AppResult<Json<serde_json::Value>> {
    if !password::verify_password(&body.current_password, &auth.user.password_hash) {
        return Err(AppError::bad_request("current password is incorrect"));
    }
    password::validate_password_len(&body.new_password)?;

    let hash = password::hash_password(&body.new_password)?;
    sqlx::query("UPDATE users SET password_hash = $1 WHERE id = $2")
        .bind(&hash)
        .bind(auth.user.id)
        .execute(&state.db)
        .await?;

    let revoked = session::delete_others_for_user(&state.db, auth.user.id, &auth.token).await?;

    Ok(Json(serde_json::json!({
        "ok": true,
        "other_sessions_revoked": revoked,
    })))
}
