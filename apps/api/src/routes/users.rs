//! Account administration.
//!
//! There is no self-service registration and no password recovery: accounts are
//! created by administrators and reset by administrators. That is what lets the
//! application avoid email entirely.

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use uuid::Uuid;

use crate::auth::extract::AdminUser;
use crate::auth::{password, session};
use crate::dto::{CreateUserRequest, SetPasswordRequest, UserSummary};
use crate::error::{on_unique_violation, AppError, AppResult};
use crate::models::User;
use crate::state::AppState;

pub const MIN_USERNAME_LEN: usize = 3;
pub const MAX_USERNAME_LEN: usize = 32;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/users", get(list_users).post(create_user))
        .route("/users/{id}/password", post(set_password))
}

fn to_summary(user: User) -> UserSummary {
    UserSummary {
        id: user.id,
        username: user.username,
        email: user.email,
        is_admin: user.is_admin,
        created_at: user.created_at,
    }
}

pub(crate) fn validate_username(username: &str) -> AppResult<String> {
    let trimmed = username.trim();
    let len = trimmed.chars().count();
    if !(MIN_USERNAME_LEN..=MAX_USERNAME_LEN).contains(&len) {
        return Err(AppError::bad_request(format!(
            "username must be between {MIN_USERNAME_LEN} and {MAX_USERNAME_LEN} characters"
        )));
    }
    if trimmed.chars().any(char::is_whitespace) {
        return Err(AppError::bad_request(
            "username must not contain spaces",
        ));
    }
    Ok(trimmed.to_string())
}

async fn list_users(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> AppResult<Json<Vec<UserSummary>>> {
    let users = sqlx::query_as::<_, User>("SELECT * FROM users ORDER BY lower(username)")
        .fetch_all(&state.db)
        .await?;
    Ok(Json(users.into_iter().map(to_summary).collect()))
}

async fn create_user(
    State(state): State<AppState>,
    _admin: AdminUser,
    Json(body): Json<CreateUserRequest>,
) -> AppResult<Json<UserSummary>> {
    let username = validate_username(&body.username)?;
    password::validate_password_len(&body.password)?;

    let hash = password::hash_password(&body.password)?;
    let email = body.email.as_deref().map(str::trim).filter(|s| !s.is_empty());

    let user = sqlx::query_as::<_, User>(
        "INSERT INTO users (username, email, password_hash, is_admin)
         VALUES ($1, $2, $3, $4) RETURNING *",
    )
    .bind(&username)
    .bind(email)
    .bind(&hash)
    .bind(body.is_admin)
    .fetch_one(&state.db)
    .await
    .map_err(|e| on_unique_violation(e, "that username is already taken"))?;

    Ok(Json(to_summary(user)))
}

/// Sets a password without knowing the old one, and revokes every session the
/// account holds so a compromised login cannot outlive the reset.
async fn set_password(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
    Json(body): Json<SetPasswordRequest>,
) -> AppResult<Json<serde_json::Value>> {
    password::validate_password_len(&body.new_password)?;
    let hash = password::hash_password(&body.new_password)?;

    let result = sqlx::query("UPDATE users SET password_hash = $1 WHERE id = $2")
        .bind(&hash)
        .bind(id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    let revoked = session::delete_all_for_user(&state.db, id).await?;

    Ok(Json(serde_json::json!({
        "ok": true,
        "sessions_revoked": revoked,
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn username_length_is_enforced() {
        assert!(validate_username("ab").is_err());
        assert!(validate_username("abc").is_ok());
        assert!(validate_username(&"a".repeat(32)).is_ok());
        assert!(validate_username(&"a".repeat(33)).is_err());
    }

    #[test]
    fn usernames_are_trimmed_and_reject_internal_spaces() {
        assert_eq!(validate_username("  alice  ").unwrap(), "alice");
        assert!(validate_username("al ice").is_err());
    }
}
