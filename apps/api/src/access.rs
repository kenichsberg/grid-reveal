//! Screen access resolution.
//!
//! The rule that matters: a caller who may not see a screen gets 404, never
//! 403. A 403 would confirm that a slug names a real screen, which is exactly
//! what a private screen must not disclose.

use sqlx::PgPool;
use uuid::Uuid;

use crate::board;
use crate::error::{AppError, AppResult};
use crate::models::{Screen, User};

/// Resolves a screen the caller is allowed to view.
///
/// Administrators resolve any screen whether or not they are members. Without
/// this they could not manage a screen they had not joined, having no way to
/// see it in order to add themselves.
pub async fn screen_for_viewer(pool: &PgPool, slug: &str, user: &User) -> AppResult<Screen> {
    let screen = sqlx::query_as::<_, Screen>("SELECT * FROM screens WHERE slug = $1")
        .bind(slug)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::NotFound)?;

    if user.is_admin || is_member(pool, screen.id, user.id).await? {
        Ok(screen)
    } else {
        Err(AppError::NotFound)
    }
}

/// Resolves a screen for an administrative action.
///
/// Still 404 rather than 403 for a non-administrator, so probing with an
/// administrative verb discloses no more than a read would.
pub async fn screen_for_admin(pool: &PgPool, slug: &str, user: &User) -> AppResult<Screen> {
    if !user.is_admin {
        // Resolve as a viewer first so a member learns their action was
        // refused, while a non-member still learns nothing.
        let _ = screen_for_viewer(pool, slug, user).await?;
        return Err(AppError::Forbidden);
    }
    screen_for_viewer(pool, slug, user).await
}

pub async fn is_member(pool: &PgPool, screen_id: Uuid, user_id: Uuid) -> AppResult<bool> {
    let found: Option<(i32,)> =
        sqlx::query_as("SELECT 1 FROM screen_members WHERE screen_id = $1 AND user_id = $2")
            .bind(screen_id)
            .bind(user_id)
            .fetch_optional(pool)
            .await?;
    Ok(found.is_some())
}

/// Rejects writes to an archived screen.
pub fn ensure_writable(screen: &Screen) -> AppResult<()> {
    if screen.is_archived() {
        return Err(AppError::bad_request(
            "this screen is archived and cannot be changed",
        ));
    }
    Ok(())
}

/// Total live minutes logged against a screen.
pub async fn total_minutes(pool: &PgPool, screen_id: Uuid) -> AppResult<i64> {
    let (total,): (i64,) = sqlx::query_as(
        "SELECT COALESCE(SUM(minutes), 0)::bigint FROM entries
         WHERE screen_id = $1 AND deleted_at IS NULL",
    )
    .bind(screen_id)
    .fetch_one(pool)
    .await?;
    Ok(total)
}

pub async fn participant_count(pool: &PgPool, screen_id: Uuid) -> AppResult<i64> {
    let (count,): (i64,) =
        sqlx::query_as("SELECT COUNT(*)::bigint FROM screen_members WHERE screen_id = $1")
            .bind(screen_id)
            .fetch_one(pool)
            .await?;
    Ok(count)
}

/// The derived board state: total minutes logged and how many tiles that
/// reveals.
pub async fn board_state(pool: &PgPool, screen: &Screen) -> AppResult<(i64, i32)> {
    let total = total_minutes(pool, screen.id).await?;
    let revealed = board::revealed_count(total, screen.unit_minutes, screen.total_tiles());
    Ok((total, revealed))
}
