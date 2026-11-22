//! Screen access resolution.
//!
//! The rule that matters: a caller who may not see a screen gets 404, never
//! 403. A 403 would confirm that a slug names a real screen, which is exactly
//! what a private screen must not disclose.

use chrono::NaiveDate;
use sqlx::PgPool;
use uuid::Uuid;

use crate::board;
use crate::error::{AppError, AppResult};
use crate::models::{Screen, ScreenPhase, User};

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

/// Rejects writes to a screen that is not currently open.
///
/// The single write gate for both entries and goals, so widening it here is
/// what makes an ended screen read-only in the same way an archived one
/// already was. Each phase gets its own message: "archived" and "the period
/// ended" are different facts to the member looking at the board.
pub fn ensure_writable(screen: &Screen, today: NaiveDate) -> AppResult<()> {
    match screen.phase(today) {
        ScreenPhase::Active => Ok(()),
        ScreenPhase::Archived => Err(AppError::bad_request(
            "this screen is archived and cannot be changed",
        )),
        ScreenPhase::Ended => Err(AppError::bad_request(
            "this screen's period has ended and it can no longer be changed",
        )),
        ScreenPhase::Upcoming => Err(AppError::bad_request(
            "this screen's period has not started yet",
        )),
    }
}

/// Rejects an entry dated outside the screen's active period.
///
/// Separate from [`ensure_writable`], which asks whether *today* permits a
/// write. This asks whether the date being claimed is one the window covers:
/// without it, a member could log time during the period but backdate it to
/// before the period began.
pub fn ensure_in_window(screen: &Screen, occurred_at: NaiveDate) -> AppResult<()> {
    if screen.covers(occurred_at) {
        return Ok(());
    }
    Err(AppError::bad_request(
        "that date falls outside this screen's period",
    ))
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    fn date(iso: &str) -> NaiveDate {
        iso.parse().expect("test date")
    }

    fn screen(starts: &str, ends: &str, auto: Option<i32>) -> Screen {
        Screen {
            id: Uuid::nil(),
            slug: "test".into(),
            title: "Test".into(),
            picture_id: Uuid::nil(),
            cols: 10,
            rows: 10,
            unit_minutes: 5,
            permutation: (0..100).collect(),
            archived_at: None,
            created_by: Uuid::nil(),
            created_at: Utc::now(),
            starts_on: date(starts),
            ends_on: date(ends),
            auto_archive_days: auto,
            progress_title: None,
            entry_noun: None,
        }
    }

    #[test]
    fn a_screen_inside_its_period_is_writable() {
        let screen = screen("2026-01-01", "2026-12-31", None);
        assert!(ensure_writable(&screen, date("2026-06-01")).is_ok());
    }

    #[test]
    fn each_closed_phase_explains_itself_differently() {
        let screen = screen("2026-01-10", "2026-01-20", Some(15));

        let upcoming = ensure_writable(&screen, date("2026-01-01")).unwrap_err();
        assert!(
            format!("{upcoming:?}").contains("not started"),
            "upcoming message was {upcoming:?}"
        );

        let ended = ensure_writable(&screen, date("2026-01-25")).unwrap_err();
        assert!(
            format!("{ended:?}").contains("period has ended"),
            "ended message was {ended:?}"
        );

        let archived = ensure_writable(&screen, date("2026-03-01")).unwrap_err();
        assert!(
            format!("{archived:?}").contains("archived"),
            "archived message was {archived:?}"
        );
    }

    #[test]
    fn a_manually_archived_screen_is_not_writable() {
        let screen = Screen {
            archived_at: Some(Utc::now()),
            ..screen("2026-01-01", "2026-12-31", None)
        };
        assert!(ensure_writable(&screen, date("2026-06-01")).is_err());
    }

    #[test]
    fn the_window_bounds_which_dates_an_entry_may_claim() {
        let screen = screen("2026-01-10", "2026-01-20", None);
        assert!(ensure_in_window(&screen, date("2026-01-10")).is_ok());
        assert!(ensure_in_window(&screen, date("2026-01-20")).is_ok());
        assert!(ensure_in_window(&screen, date("2026-01-09")).is_err());
        assert!(ensure_in_window(&screen, date("2026-01-21")).is_err());
    }

}
