//! Screen goals: free-text bullets any member may add.

use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use chrono::Utc;
use uuid::Uuid;

use crate::access;
use crate::auth::extract::AuthUser;
use crate::dto::{CreateGoalRequest, GoalSummary};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

pub const MAX_GOAL_LEN: usize = 200;
pub const MAX_GOALS_PER_SCREEN: i64 = 50;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/screens/{slug}/goals", get(list).post(create))
        .route("/goals/{id}", axum::routing::delete(destroy))
}

fn validate_text(text: &str) -> AppResult<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(AppError::bad_request("a goal cannot be empty"));
    }
    if trimmed.chars().count() > MAX_GOAL_LEN {
        return Err(AppError::bad_request(format!(
            "goals must be {MAX_GOAL_LEN} characters or fewer"
        )));
    }
    Ok(trimmed.to_string())
}

/// Lists a screen's goals.
///
/// Authorship is disclosed only to administrators. A member receives `is_mine`
/// so they can see a delete control on their own goals, which is the only
/// signal of authorship they get — including about themselves.
async fn list(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(slug): Path<String>,
) -> AppResult<Json<Vec<GoalSummary>>> {
    let screen = access::screen_for_viewer(&state.db, &slug, &auth.user).await?;

    let rows = sqlx::query_as::<_, (Uuid, String, chrono::DateTime<chrono::Utc>, Uuid, String)>(
        "SELECT g.id, g.text, g.created_at, g.created_by, u.username
         FROM goals g JOIN users u ON u.id = g.created_by
         WHERE g.screen_id = $1 AND g.deleted_at IS NULL
         ORDER BY g.position, g.created_at",
    )
    .bind(screen.id)
    .fetch_all(&state.db)
    .await?;

    let viewer_is_admin = auth.user.is_admin;

    Ok(Json(
        rows.into_iter()
            .map(|(id, text, created_at, created_by, username)| GoalSummary {
                id,
                text,
                created_at,
                is_mine: created_by == auth.user.id,
                author_username: viewer_is_admin.then_some(username),
            })
            .collect(),
    ))
}

async fn create(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(slug): Path<String>,
    Json(body): Json<CreateGoalRequest>,
) -> AppResult<Json<GoalSummary>> {
    let screen = access::screen_for_viewer(&state.db, &slug, &auth.user).await?;
    access::ensure_writable(&screen, Utc::now().date_naive())?;

    let text = validate_text(&body.text)?;

    let (count,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*)::bigint FROM goals WHERE screen_id = $1 AND deleted_at IS NULL",
    )
    .bind(screen.id)
    .fetch_one(&state.db)
    .await?;

    if count >= MAX_GOALS_PER_SCREEN {
        return Err(AppError::bad_request(format!(
            "this screen already has the maximum of {MAX_GOALS_PER_SCREEN} goals"
        )));
    }

    let (id, created_at): (Uuid, chrono::DateTime<chrono::Utc>) = sqlx::query_as(
        "INSERT INTO goals (screen_id, text, created_by, position)
         VALUES ($1, $2, $3, $4) RETURNING id, created_at",
    )
    .bind(screen.id)
    .bind(&text)
    .bind(auth.user.id)
    .bind(count as i32)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(GoalSummary {
        id,
        text,
        created_at,
        is_mine: true,
        author_username: auth
            .user
            .is_admin
            .then(|| auth.user.username.clone()),
    }))
}

/// Deletable by its author or by any administrator.
async fn destroy(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let row: Option<(Uuid, Uuid)> = sqlx::query_as(
        "SELECT screen_id, created_by FROM goals WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?;

    let (screen_id, created_by) = row.ok_or(AppError::NotFound)?;

    let screen = sqlx::query_as::<_, crate::models::Screen>("SELECT * FROM screens WHERE id = $1")
        .bind(screen_id)
        .fetch_one(&state.db)
        .await?;

    // Resolve through the viewer path first, so a non-member probing a goal id
    // learns nothing beyond "not found".
    if !auth.user.is_admin && !access::is_member(&state.db, screen.id, auth.user.id).await? {
        return Err(AppError::NotFound);
    }
    access::ensure_writable(&screen, Utc::now().date_naive())?;

    if created_by != auth.user.id && !auth.user.is_admin {
        return Err(AppError::Forbidden);
    }

    sqlx::query("UPDATE goals SET deleted_at = now() WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({ "ok": true })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn goal_text_is_trimmed_and_required() {
        assert!(validate_text("").is_err());
        assert!(validate_text("    ").is_err());
        assert_eq!(validate_text("  finish ch. 4 ").unwrap(), "finish ch. 4");
    }

    #[test]
    fn goal_text_length_is_bounded() {
        assert!(validate_text(&"a".repeat(MAX_GOAL_LEN)).is_ok());
        assert!(validate_text(&"a".repeat(MAX_GOAL_LEN + 1)).is_err());
    }
}
