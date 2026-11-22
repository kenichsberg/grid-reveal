//! Progress entries.

use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use chrono::NaiveDate;
use uuid::Uuid;

use crate::access;
use crate::auth::extract::{AuthUser, ClientToday};
use crate::board;
use crate::dto::{CreateEntryRequest, EntrySummary, SubmitEntryResponse, UpdateEntryRequest};
use crate::error::{AppError, AppResult};
use crate::models::{Entry, Screen};
use crate::state::AppState;

pub const MAX_HOURS: i32 = 23;
pub const MAX_MINUTES: i32 = 59;
pub const MAX_NOTE_LEN: usize = 500;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/screens/{slug}/entries",
            get(list_own).post(create),
        )
        .route(
            "/entries/{id}",
            axum::routing::patch(update).delete(destroy),
        )
}

fn to_summary(entry: Entry) -> EntrySummary {
    EntrySummary {
        id: entry.id,
        minutes: entry.minutes,
        occurred_at: entry.occurred_at,
        note: entry.note,
        created_at: entry.created_at,
        edited_at: entry.edited_at,
    }
}

/// Combines the hours and minutes fields into the stored integer.
///
/// The split exists only in the input widget; a single entry is bounded at
/// 23h59m, which the database also enforces.
fn to_minutes(hours: i32, minutes: i32) -> AppResult<i32> {
    if !(0..=MAX_HOURS).contains(&hours) {
        return Err(AppError::bad_request(format!(
            "hours must be between 0 and {MAX_HOURS}"
        )));
    }
    if !(0..=MAX_MINUTES).contains(&minutes) {
        return Err(AppError::bad_request(format!(
            "minutes must be between 0 and {MAX_MINUTES}"
        )));
    }
    let total = hours * 60 + minutes;
    if total < 1 {
        return Err(AppError::bad_request("log at least 1 minute"));
    }
    Ok(total)
}

/// Defaults to today and refuses the future.
///
/// A plain date, never an instant: there is no sub-day resolution here, so
/// storing one would only invite timezone bugs.
///
/// `today` is the caller's day, not the server's. Taking it as an argument is
/// what lets a member east of UTC log at 00:30 without being told their entry
/// is in the future — and what makes this function testable at all.
fn resolve_date(provided: Option<NaiveDate>, today: NaiveDate) -> AppResult<NaiveDate> {
    match provided {
        None => Ok(today),
        Some(date) if date > today => Err(AppError::bad_request(
            "progress cannot be logged for a future date",
        )),
        Some(date) => Ok(date),
    }
}

fn validate_note(note: Option<String>) -> AppResult<Option<String>> {
    match note.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        None => Ok(None),
        Some(text) if text.chars().count() > MAX_NOTE_LEN => Err(AppError::bad_request(format!(
            "notes must be {MAX_NOTE_LEN} characters or fewer"
        ))),
        Some(text) => Ok(Some(text.to_string())),
    }
}

/// A member sees only their own entries. There is no leaderboard and no
/// per-member breakdown; only the combined screen total is shared.
async fn list_own(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(slug): Path<String>,
) -> AppResult<Json<Vec<EntrySummary>>> {
    let screen = access::screen_for_viewer(&state.db, &slug, &auth.user).await?;

    let entries = sqlx::query_as::<_, Entry>(
        "SELECT * FROM entries
         WHERE screen_id = $1 AND user_id = $2 AND deleted_at IS NULL
         ORDER BY occurred_at DESC, created_at DESC",
    )
    .bind(screen.id)
    .bind(auth.user.id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(entries.into_iter().map(to_summary).collect()))
}

/// Records progress and reports what it revealed.
///
/// The entry is always attributed to the authenticated caller; there is no
/// parameter for logging on another member's behalf.
async fn create(
    State(state): State<AppState>,
    auth: AuthUser,
    client_today: ClientToday,
    Path(slug): Path<String>,
    Json(body): Json<CreateEntryRequest>,
) -> AppResult<Json<SubmitEntryResponse>> {
    let today = client_today.date();
    let screen = access::screen_for_viewer(&state.db, &slug, &auth.user).await?;
    access::ensure_writable(&screen, today)?;

    // Members log time; an administrator who is not a member may view the
    // screen but has nothing to contribute to it.
    if !access::is_member(&state.db, screen.id, auth.user.id).await? {
        return Err(AppError::bad_request(
            "you are not a participant on this screen",
        ));
    }

    let minutes = to_minutes(body.hours, body.minutes)?;
    let occurred_at = resolve_date(body.occurred_at, today)?;
    access::ensure_in_window(&screen, occurred_at)?;
    let note = validate_note(body.note)?;

    let mut tx = state.db.begin().await?;

    // The board state is derived, so correctness never depended on a lock.
    // This one exists so that `your_tiles` is exact: without it two concurrent
    // submissions could read the same prior total and each claim the same
    // tiles in their response, even though the board itself stayed right.
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 0))")
        .bind(screen.id.to_string())
        .execute(&mut *tx)
        .await?;

    let (total_before,): (i64,) = sqlx::query_as(
        "SELECT COALESCE(SUM(minutes), 0)::bigint FROM entries
         WHERE screen_id = $1 AND deleted_at IS NULL",
    )
    .bind(screen.id)
    .fetch_one(&mut *tx)
    .await?;

    let entry = sqlx::query_as::<_, Entry>(
        "INSERT INTO entries (screen_id, user_id, minutes, occurred_at, note)
         VALUES ($1, $2, $3, $4, $5) RETURNING *",
    )
    .bind(screen.id)
    .bind(auth.user.id)
    .bind(minutes)
    .bind(occurred_at)
    .bind(note.as_deref())
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    let tiles = screen.total_tiles();
    let positions = board::positions_revealed_by(total_before, minutes, screen.unit_minutes, tiles);
    let your_tiles = board::tiles_at(&screen.permutation, positions);

    let total_minutes = total_before + i64::from(minutes);
    let revealed_count = board::revealed_count(total_minutes, screen.unit_minutes, tiles);

    Ok(Json(SubmitEntryResponse {
        entry: to_summary(entry),
        total_minutes,
        revealed_count,
        your_tiles,
        revealed_tiles: board::revealed_tiles(&screen.permutation, revealed_count),
        completed: revealed_count >= tiles,
    }))
}

/// Loads an entry the caller owns, along with its screen.
async fn own_entry(state: &AppState, id: Uuid, user_id: Uuid) -> AppResult<(Entry, Screen)> {
    let entry = sqlx::query_as::<_, Entry>(
        "SELECT * FROM entries WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;

    // Editing someone else's entry is forbidden rather than hidden: the caller
    // already knows the entry id, so there is nothing left to conceal.
    if entry.user_id != user_id {
        return Err(AppError::Forbidden);
    }

    let screen = sqlx::query_as::<_, Screen>("SELECT * FROM screens WHERE id = $1")
        .bind(entry.screen_id)
        .fetch_one(&state.db)
        .await?;

    Ok((entry, screen))
}

async fn update(
    State(state): State<AppState>,
    auth: AuthUser,
    client_today: ClientToday,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateEntryRequest>,
) -> AppResult<Json<EntrySummary>> {
    let today = client_today.date();
    let (entry, screen) = own_entry(&state, id, auth.user.id).await?;
    access::ensure_writable(&screen, today)?;

    let minutes = to_minutes(body.hours, body.minutes)?;
    let occurred_at = resolve_date(body.occurred_at, today)?;
    access::ensure_in_window(&screen, occurred_at)?;
    let note = validate_note(body.note)?;

    let updated = sqlx::query_as::<_, Entry>(
        "UPDATE entries SET minutes = $1, occurred_at = $2, note = $3, edited_at = now()
         WHERE id = $4 RETURNING *",
    )
    .bind(minutes)
    .bind(occurred_at)
    .bind(note.as_deref())
    .bind(entry.id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(to_summary(updated)))
}

/// Soft delete. The row stays so per-tile attribution remains reconstructible;
/// every total excludes it immediately, so the board rewinds.
async fn destroy(
    State(state): State<AppState>,
    auth: AuthUser,
    client_today: ClientToday,
    Path(id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let (entry, screen) = own_entry(&state, id, auth.user.id).await?;
    access::ensure_writable(&screen, client_today.date())?;

    sqlx::query("UPDATE entries SET deleted_at = now() WHERE id = $1")
        .bind(entry.id)
        .execute(&state.db)
        .await?;

    let (total_minutes, revealed_count) = access::board_state(&state.db, &screen).await?;

    Ok(Json(serde_json::json!({
        "ok": true,
        "total_minutes": total_minutes,
        "revealed_count": revealed_count,
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hours_and_minutes_combine() {
        assert_eq!(to_minutes(0, 30).unwrap(), 30);
        assert_eq!(to_minutes(2, 15).unwrap(), 135);
        assert_eq!(to_minutes(23, 59).unwrap(), 1439);
    }

    #[test]
    fn zero_duration_is_rejected() {
        assert!(to_minutes(0, 0).is_err());
    }

    #[test]
    fn out_of_range_durations_are_rejected() {
        assert!(to_minutes(24, 0).is_err());
        assert!(to_minutes(0, 60).is_err());
        assert!(to_minutes(-1, 10).is_err());
        assert!(to_minutes(1, -1).is_err());
    }

    #[test]
    fn date_defaults_to_today_and_refuses_the_future() {
        let today = NaiveDate::from_ymd_opt(2026, 11, 19).unwrap();
        assert_eq!(resolve_date(None, today).unwrap(), today);
        assert_eq!(resolve_date(Some(today), today).unwrap(), today);

        let yesterday = today.pred_opt().unwrap();
        assert_eq!(resolve_date(Some(yesterday), today).unwrap(), yesterday);

        let tomorrow = today.succ_opt().unwrap();
        assert!(resolve_date(Some(tomorrow), today).is_err());
    }

    /// The skew case, stated as a unit test: the caller's day is a day ahead of
    /// the server's, and an entry dated that day is the present, not the future.
    #[test]
    fn the_callers_day_is_what_counts_as_today() {
        let caller_today = NaiveDate::from_ymd_opt(2026, 11, 20).unwrap();
        assert_eq!(
            resolve_date(Some(caller_today), caller_today).unwrap(),
            caller_today
        );
        assert_eq!(resolve_date(None, caller_today).unwrap(), caller_today);
    }

    #[test]
    fn notes_are_trimmed_bounded_and_optional() {
        assert_eq!(validate_note(None).unwrap(), None);
        assert_eq!(validate_note(Some("   ".into())).unwrap(), None);
        assert_eq!(validate_note(Some("  hi  ".into())).unwrap(), Some("hi".into()));
        assert!(validate_note(Some("x".repeat(MAX_NOTE_LEN + 1))).is_err());
    }
}
