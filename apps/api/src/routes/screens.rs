//! Screens: creation, access, membership, geometry, and lifecycle.

use axum::extract::{Path, Query, State};
use chrono::{NaiveDate, Utc};
use axum::routing::{get, post};
use axum::{Json, Router};
use rand::seq::SliceRandom;
use rand::Rng;
use sqlx::PgPool;
use uuid::Uuid;

use crate::access;
use crate::auth::extract::{AdminUser, AuthUser};
use crate::board;
use crate::dto::{
    AddMemberRequest, AdminScreenEntry, CloneScreenRequest, CreateScreenRequest,
    DeleteScreenRequest, GeometryPreview, GeometryPreviewRequest, MemberSummary, RemovalImpact,
    RemoveMemberRequest, ScreenDetail, ScreenList, ScreenSummary, UpdateScreenRequest,
};
use crate::error::{AppError, AppResult};
use crate::geometry;
use crate::models::{Picture, Screen};
use crate::state::AppState;

const SLUG_ALPHABET: &[u8] = b"abcdefghijkmnpqrstuvwxyz23456789";
const SLUG_LEN: usize = 10;
const MAX_TITLE_LEN: usize = 120;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/screens", get(list).post(create))
        .route("/admin/screens", get(directory))
        .route("/screens/geometry-preview", get(geometry_preview))
        .route(
            "/screens/{slug}",
            get(detail).patch(update).delete(destroy),
        )
        .route("/screens/{slug}/members", get(members).post(add_member))
        .route(
            "/screens/{slug}/members/{user_id}",
            axum::routing::delete(remove_member),
        )
        .route(
            "/screens/{slug}/members/{user_id}/removal-impact",
            get(removal_impact),
        )
        .route("/screens/{slug}/clone", post(clone_screen))
        .route("/screens/{slug}/archive", post(archive))
        .route("/screens/{slug}/unarchive", post(unarchive))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Random, unguessable, and not caller-supplied.
///
/// The alphabet omits characters that are easy to confuse when read aloud or
/// copied by hand.
fn generate_slug() -> String {
    let mut rng = rand::thread_rng();
    (0..SLUG_LEN)
        .map(|_| SLUG_ALPHABET[rng.gen_range(0..SLUG_ALPHABET.len())] as char)
        .collect()
}

async fn unique_slug(pool: &PgPool) -> AppResult<String> {
    for _ in 0..10 {
        let candidate = generate_slug();
        let existing: Option<(i32,)> = sqlx::query_as("SELECT 1 FROM screens WHERE slug = $1")
            .bind(&candidate)
            .fetch_optional(pool)
            .await?;
        if existing.is_none() {
            return Ok(candidate);
        }
    }
    Err(AppError::internal("could not allocate a unique slug"))
}

/// The fixed reveal order for a grid of `n` tiles.
fn random_permutation(n: i32) -> Vec<i32> {
    let mut tiles: Vec<i32> = (0..n).collect();
    tiles.shuffle(&mut rand::thread_rng());
    tiles
}

fn validate_title(title: &str) -> AppResult<String> {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        return Err(AppError::bad_request("a title is required"));
    }
    if trimmed.chars().count() > MAX_TITLE_LEN {
        return Err(AppError::bad_request(format!(
            "the title must be {MAX_TITLE_LEN} characters or fewer"
        )));
    }
    Ok(trimmed.to_string())
}

fn validate_unit(unit_minutes: i32) -> AppResult<()> {
    if unit_minutes < 1 {
        return Err(AppError::bad_request(
            "time per tile must be at least 1 minute",
        ));
    }
    Ok(())
}

/// Checks an active period before it reaches the database.
///
/// The same rules the table's CHECK constraints enforce, stated here so the
/// administrator gets a sentence rather than a constraint violation.
fn validate_window(
    starts_on: Option<NaiveDate>,
    ends_on: Option<NaiveDate>,
    auto_archive_days: Option<i32>,
) -> AppResult<()> {
    if let (Some(starts_on), Some(ends_on)) = (starts_on, ends_on) {
        if ends_on < starts_on {
            return Err(AppError::bad_request(
                "the end date cannot fall before the start date",
            ));
        }
    }
    if auto_archive_days.is_some_and(|days| days < 0) {
        return Err(AppError::bad_request(
            "the auto-archive term cannot be negative",
        ));
    }
    if auto_archive_days.is_some() && ends_on.is_none() {
        return Err(AppError::bad_request(
            "an auto-archive term needs an end date to count from",
        ));
    }
    Ok(())
}

/// Loads a picture that may be attached to a screen. Soft-deleted pictures are
/// refused for new screens while remaining attached to existing ones.
async fn selectable_picture(pool: &PgPool, id: Uuid) -> AppResult<Picture> {
    sqlx::query_as::<_, Picture>("SELECT * FROM pictures WHERE id = $1 AND deleted_at IS NULL")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::bad_request("that picture is not available"))
}

async fn picture_of(pool: &PgPool, id: Uuid) -> AppResult<Picture> {
    sqlx::query_as::<_, Picture>("SELECT * FROM pictures WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::NotFound)
}

async fn to_summary(
    state: &AppState,
    screen: &Screen,
    today: NaiveDate,
) -> AppResult<ScreenSummary> {
    let (total_minutes, revealed_count) = access::board_state(&state.db, screen).await?;
    let picture = picture_of(&state.db, screen.picture_id).await?;

    Ok(ScreenSummary {
        slug: screen.slug.clone(),
        title: screen.title.clone(),
        picture_url: state.storage.public_url(&picture.storage_key),
        archived: screen.is_archived_on(today),
        phase: screen.phase(today).into(),
        ends_on: screen.ends_on,
        total_minutes,
        revealed_count,
        total_tiles: screen.total_tiles(),
    })
}

// ---------------------------------------------------------------------------
// Reads
// ---------------------------------------------------------------------------

/// Only the caller's own screens, split by archive status.
async fn list(State(state): State<AppState>, auth: AuthUser) -> AppResult<Json<ScreenList>> {
    let screens = sqlx::query_as::<_, Screen>(
        "SELECT s.* FROM screens s
         JOIN screen_members m ON m.screen_id = s.id
         WHERE m.user_id = $1
         ORDER BY s.created_at DESC",
    )
    .bind(auth.user.id)
    .fetch_all(&state.db)
    .await?;

    // Automatic archiving is derived, not stored, so the split is made here
    // rather than in SQL: a screen whose retention term has expired belongs in
    // the archived section even though nothing wrote `archived_at`.
    let today = Utc::now().date_naive();
    let mut active = Vec::new();
    let mut archived = Vec::new();
    for screen in &screens {
        let summary = to_summary(&state, screen, today).await?;
        if screen.is_archived_on(today) {
            archived.push(summary);
        } else {
            active.push(summary);
        }
    }

    Ok(Json(ScreenList { active, archived }))
}

/// Every screen, for administrators only.
///
/// The personal list at `GET /screens` stays member-scoped; this is the
/// separate question of what an administrator can manage.
async fn directory(
    State(state): State<AppState>,
    admin: AdminUser,
) -> AppResult<Json<Vec<AdminScreenEntry>>> {
    let screens = sqlx::query_as::<_, Screen>("SELECT * FROM screens ORDER BY created_at DESC")
        .fetch_all(&state.db)
        .await?;

    let today = Utc::now().date_naive();
    let mut out = Vec::with_capacity(screens.len());
    for screen in &screens {
        let summary = to_summary(&state, screen, today).await?;
        out.push(AdminScreenEntry {
            slug: summary.slug,
            title: summary.title,
            picture_url: summary.picture_url,
            archived: summary.archived,
            phase: summary.phase,
            ends_on: summary.ends_on,
            total_minutes: summary.total_minutes,
            revealed_count: summary.revealed_count,
            total_tiles: summary.total_tiles,
            viewer_is_member: access::is_member(&state.db, screen.id, admin.user.id).await?,
        });
    }

    Ok(Json(out))
}

async fn detail(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(slug): Path<String>,
) -> AppResult<Json<ScreenDetail>> {
    let screen = access::screen_for_viewer(&state.db, &slug, &auth.user).await?;
    let picture = picture_of(&state.db, screen.picture_id).await?;
    let (total_minutes, revealed_count) = access::board_state(&state.db, &screen).await?;
    let participant_count = access::participant_count(&state.db, screen.id).await?;
    let today = Utc::now().date_naive();

    Ok(Json(ScreenDetail {
        slug: screen.slug.clone(),
        title: screen.title.clone(),
        picture_url: state.storage.public_url(&picture.storage_key),
        picture_width: picture.width,
        picture_height: picture.height,
        cols: screen.cols,
        rows: screen.rows,
        unit_minutes: screen.unit_minutes,
        total_tiles: screen.total_tiles(),
        participant_count,
        total_minutes,
        revealed_count,
        // The revealed PREFIX only. Sending the rest would hand the client the
        // future reveal order.
        revealed_tiles: board::revealed_tiles(&screen.permutation, revealed_count),
        completed: revealed_count >= screen.total_tiles(),
        archived: screen.is_archived_on(today),
        phase: screen.phase(today).into(),
        starts_on: screen.starts_on,
        ends_on: screen.ends_on,
        auto_archive_days: screen.auto_archive_days,
        viewer_is_admin: auth.user.is_admin,
    }))
}

/// Live preview for the creation form.
async fn geometry_preview(
    State(state): State<AppState>,
    _admin: AdminUser,
    Query(query): Query<GeometryPreviewRequest>,
) -> AppResult<Json<GeometryPreview>> {
    validate_unit(query.unit_minutes)?;
    let picture = selectable_picture(&state.db, query.picture_id).await?;
    let aspect = picture.aspect();

    let solved = match query.cols {
        Some(cols) => geometry::solve_with_cols(query.target_total_minutes, query.unit_minutes, cols),
        None => geometry::solve(query.target_total_minutes, query.unit_minutes, aspect),
    };

    Ok(Json(GeometryPreview {
        cols: solved.cols,
        rows: solved.rows,
        total_tiles: solved.total_tiles(),
        exact_total_minutes: solved.exact_total_minutes(query.unit_minutes),
        requested_total_minutes: query.target_total_minutes,
        cell_squareness: geometry::cell_squareness(solved, aspect),
    }))
}

// ---------------------------------------------------------------------------
// Creation
// ---------------------------------------------------------------------------

async fn create(
    State(state): State<AppState>,
    admin: AdminUser,
    Json(body): Json<CreateScreenRequest>,
) -> AppResult<Json<ScreenDetail>> {
    let title = validate_title(&body.title)?;
    validate_unit(body.unit_minutes)?;
    validate_window(body.starts_on, body.ends_on, body.auto_archive_days)?;

    let picture = selectable_picture(&state.db, body.picture_id).await?;
    let solved = match body.cols {
        Some(cols) => geometry::solve_with_cols(body.target_total_minutes, body.unit_minutes, cols),
        None => geometry::solve(body.target_total_minutes, body.unit_minutes, picture.aspect()),
    };

    let slug = unique_slug(&state.db).await?;
    let permutation = random_permutation(solved.total_tiles());

    let mut tx = state.db.begin().await?;

    let screen = sqlx::query_as::<_, Screen>(
        "INSERT INTO screens
           (slug, title, picture_id, cols, rows, unit_minutes, permutation, created_by,
            starts_on, ends_on, auto_archive_days)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) RETURNING *",
    )
    .bind(&slug)
    .bind(&title)
    .bind(picture.id)
    .bind(solved.cols)
    .bind(solved.rows)
    .bind(body.unit_minutes)
    .bind(&permutation)
    .bind(admin.user.id)
    .bind(body.starts_on)
    .bind(body.ends_on)
    .bind(body.auto_archive_days)
    .fetch_one(&mut *tx)
    .await?;

    // The creating administrator is a participant, not just an operator.
    sqlx::query("INSERT INTO screen_members (screen_id, user_id) VALUES ($1, $2)")
        .bind(screen.id)
        .bind(admin.user.id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    detail(State(state), AuthUser { user: admin.user, token: admin.token }, Path(slug)).await
}

// ---------------------------------------------------------------------------
// Editing
// ---------------------------------------------------------------------------

/// Edits the title, and optionally the geometry or the active period.
///
/// Deliberately not gated on [`access::ensure_writable`]: editing the window is
/// exactly how an administrator reopens a screen whose period has closed, so
/// refusing the edit because the period has closed would be circular.
///
/// A geometry change leaves every entry untouched, so the total logged time is
/// preserved exactly. The permutation is regenerated because the tile set
/// itself changed, and the revealed count re-derives — which can mean the
/// picture becomes *less* revealed than it was. That is deliberate; the client
/// warns before confirming.
async fn update(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(slug): Path<String>,
    Json(body): Json<UpdateScreenRequest>,
) -> AppResult<Json<ScreenDetail>> {
    let screen = access::screen_for_admin(&state.db, &slug, &admin.user).await?;

    if let Some(title) = &body.title {
        let title = validate_title(title)?;
        sqlx::query("UPDATE screens SET title = $1 WHERE id = $2")
            .bind(&title)
            .bind(screen.id)
            .execute(&state.db)
            .await?;
    }

    let geometry_changed =
        body.cols.is_some() || body.rows.is_some() || body.unit_minutes.is_some();

    if geometry_changed {
        let cols = body.cols.unwrap_or(screen.cols);
        let rows = body.rows.unwrap_or(screen.rows);
        let unit = body.unit_minutes.unwrap_or(screen.unit_minutes);

        validate_unit(unit)?;
        if cols < 1 || rows < 1 {
            return Err(AppError::bad_request("the grid must have at least one tile"));
        }
        if cols > geometry::MAX_DIMENSION || rows > geometry::MAX_DIMENSION {
            return Err(AppError::bad_request(format!(
                "the grid may be at most {} tiles on a side",
                geometry::MAX_DIMENSION
            )));
        }
        if cols * rows > geometry::MAX_TILES {
            return Err(AppError::bad_request(format!(
                "the grid may have at most {} tiles",
                geometry::MAX_TILES
            )));
        }

        let permutation = random_permutation(cols * rows);
        sqlx::query(
            "UPDATE screens SET cols = $1, rows = $2, unit_minutes = $3, permutation = $4
             WHERE id = $5",
        )
        .bind(cols)
        .bind(rows)
        .bind(unit)
        .bind(&permutation)
        .bind(screen.id)
        .execute(&state.db)
        .await?;
    }

    // The window is replaced whole rather than field by field, so that sending
    // it with a null bound clears that bound while omitting it entirely leaves
    // the period alone.
    if let Some(window) = &body.window {
        validate_window(window.starts_on, window.ends_on, window.auto_archive_days)?;
        sqlx::query(
            "UPDATE screens SET starts_on = $1, ends_on = $2, auto_archive_days = $3
             WHERE id = $4",
        )
        .bind(window.starts_on)
        .bind(window.ends_on)
        .bind(window.auto_archive_days)
        .bind(screen.id)
        .execute(&state.db)
        .await?;
    }

    detail(State(state), AuthUser { user: admin.user, token: admin.token }, Path(slug)).await
}

// ---------------------------------------------------------------------------
// Membership
// ---------------------------------------------------------------------------

/// Every member of a screen may see who else is on it.
///
/// Managing that membership stays administrator-only; this is the read, and it
/// is the one place a member learns more than the participant count. Resolved
/// through `screen_for_viewer`, so a non-member still gets 404 rather than a
/// 403 that would confirm the slug names a real screen.
async fn members(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(slug): Path<String>,
) -> AppResult<Json<Vec<MemberSummary>>> {
    let screen = access::screen_for_viewer(&state.db, &slug, &auth.user).await?;

    // Administrators first, then alphabetically: the list is read to find out
    // who to ask about the screen as often as to see who is taking part.
    let rows = sqlx::query_as::<_, (Uuid, String, chrono::DateTime<chrono::Utc>, bool)>(
        "SELECT u.id, u.username, m.joined_at, u.is_admin
         FROM screen_members m JOIN users u ON u.id = m.user_id
         WHERE m.screen_id = $1
         ORDER BY u.is_admin DESC, lower(u.username)",
    )
    .bind(screen.id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(
        rows.into_iter()
            .map(|(user_id, username, joined_at, is_admin)| MemberSummary {
                user_id,
                username,
                joined_at,
                is_admin,
            })
            .collect(),
    ))
}

async fn add_member(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(slug): Path<String>,
    Json(body): Json<AddMemberRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let screen = access::screen_for_admin(&state.db, &slug, &admin.user).await?;

    sqlx::query(
        "INSERT INTO screen_members (screen_id, user_id) VALUES ($1, $2)
         ON CONFLICT DO NOTHING",
    )
    .bind(screen.id)
    .bind(body.user_id)
    .execute(&state.db)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(db) if db.is_foreign_key_violation() => {
            AppError::bad_request("no such user")
        }
        _ => AppError::Database(e),
    })?;

    Ok(Json(serde_json::json!({ "ok": true })))
}

/// What deleting this member's entries would cost, shown before the
/// administrator chooses.
async fn removal_impact(
    State(state): State<AppState>,
    admin: AdminUser,
    Path((slug, user_id)): Path<(String, Uuid)>,
) -> AppResult<Json<RemovalImpact>> {
    let screen = access::screen_for_admin(&state.db, &slug, &admin.user).await?;

    let (entry_count, minutes): (i64, i64) = sqlx::query_as(
        "SELECT COUNT(*)::bigint, COALESCE(SUM(minutes), 0)::bigint FROM entries
         WHERE screen_id = $1 AND user_id = $2 AND deleted_at IS NULL",
    )
    .bind(screen.id)
    .bind(user_id)
    .fetch_one(&state.db)
    .await?;

    let total = access::total_minutes(&state.db, screen.id).await?;
    let tiles = screen.total_tiles();
    let before = board::revealed_count(total, screen.unit_minutes, tiles);
    let after = board::revealed_count(total - minutes, screen.unit_minutes, tiles);

    Ok(Json(RemovalImpact {
        entry_count,
        minutes,
        revealed_tiles_lost: before - after,
    }))
}

/// Removing a member requires an explicit choice about their logged time.
async fn remove_member(
    State(state): State<AppState>,
    admin: AdminUser,
    Path((slug, user_id)): Path<(String, Uuid)>,
    Json(body): Json<RemoveMemberRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let screen = access::screen_for_admin(&state.db, &slug, &admin.user).await?;

    let mut tx = state.db.begin().await?;

    if body.delete_entries {
        sqlx::query(
            "UPDATE entries SET deleted_at = now()
             WHERE screen_id = $1 AND user_id = $2 AND deleted_at IS NULL",
        )
        .bind(screen.id)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    }

    let removed = sqlx::query("DELETE FROM screen_members WHERE screen_id = $1 AND user_id = $2")
        .bind(screen.id)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    if removed.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    Ok(Json(serde_json::json!({
        "ok": true,
        "entries_deleted": body.delete_entries,
    })))
}

// ---------------------------------------------------------------------------
// Lifecycle
// ---------------------------------------------------------------------------

/// Clones configuration, members, and goals into a fresh, fully covered board.
///
/// The title, picture and active period are not inherited: a clone is a new
/// period rather than a copy, so it starts with nothing revealed and whatever
/// window the administrator gives it.
async fn clone_screen(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(slug): Path<String>,
    Json(body): Json<CloneScreenRequest>,
) -> AppResult<Json<ScreenDetail>> {
    let source = access::screen_for_admin(&state.db, &slug, &admin.user).await?;
    let title = validate_title(&body.title)?;
    validate_window(body.starts_on, body.ends_on, body.auto_archive_days)?;
    let picture = selectable_picture(&state.db, body.picture_id).await?;

    let new_slug = unique_slug(&state.db).await?;
    let permutation = random_permutation(source.total_tiles());

    let mut tx = state.db.begin().await?;

    let created = sqlx::query_as::<_, Screen>(
        "INSERT INTO screens
           (slug, title, picture_id, cols, rows, unit_minutes, permutation, created_by,
            starts_on, ends_on, auto_archive_days)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) RETURNING *",
    )
    .bind(&new_slug)
    .bind(&title)
    .bind(picture.id)
    .bind(source.cols)
    .bind(source.rows)
    .bind(source.unit_minutes)
    .bind(&permutation)
    .bind(admin.user.id)
    .bind(body.starts_on)
    .bind(body.ends_on)
    .bind(body.auto_archive_days)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query(
        "INSERT INTO screen_members (screen_id, user_id)
         SELECT $1, user_id FROM screen_members WHERE screen_id = $2
         ON CONFLICT DO NOTHING",
    )
    .bind(created.id)
    .bind(source.id)
    .execute(&mut *tx)
    .await?;

    // The cloning administrator joins even if they were not a member of the
    // source.
    sqlx::query(
        "INSERT INTO screen_members (screen_id, user_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(created.id)
    .bind(admin.user.id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "INSERT INTO goals (screen_id, text, created_by, position)
         SELECT $1, text, created_by, position FROM goals
         WHERE screen_id = $2 AND deleted_at IS NULL",
    )
    .bind(created.id)
    .bind(source.id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    detail(
        State(state),
        AuthUser { user: admin.user, token: admin.token },
        Path(new_slug),
    )
    .await
}

async fn archive(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(slug): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let screen = access::screen_for_admin(&state.db, &slug, &admin.user).await?;
    sqlx::query("UPDATE screens SET archived_at = now() WHERE id = $1 AND archived_at IS NULL")
        .bind(screen.id)
        .execute(&state.db)
        .await?;
    Ok(Json(serde_json::json!({ "ok": true, "archived": true })))
}

/// Returns a screen to the active list, and stops it archiving itself again.
///
/// Clearing `archived_at` alone would not be enough for a screen the automatic
/// archive retired: that state is derived from `ends_on + auto_archive_days`,
/// so it would re-archive on the very next read. Un-archiving therefore also
/// cancels the automatic archive. The period itself is left alone — an
/// administrator who wants the screen writable again edits the window.
async fn unarchive(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(slug): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let screen = access::screen_for_admin(&state.db, &slug, &admin.user).await?;
    sqlx::query("UPDATE screens SET archived_at = NULL, auto_archive_days = NULL WHERE id = $1")
        .bind(screen.id)
        .execute(&state.db)
        .await?;
    Ok(Json(serde_json::json!({ "ok": true, "archived": false })))
}

/// Permanent deletion, gated on retyping the title.
///
/// The picture stays in the shared library: it may be in use elsewhere, and it
/// was uploaded independently of this screen.
async fn destroy(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(slug): Path<String>,
    Json(body): Json<DeleteScreenRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let screen = access::screen_for_admin(&state.db, &slug, &admin.user).await?;

    if body.confirm_title.trim() != screen.title {
        return Err(AppError::bad_request(
            "the confirmation text does not match the screen title",
        ));
    }

    // Entries, goals, and membership cascade from the screen row.
    sqlx::query("DELETE FROM screens WHERE id = $1")
        .bind(screen.id)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({ "ok": true })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs_are_random_and_use_an_unambiguous_alphabet() {
        let a = generate_slug();
        let b = generate_slug();
        assert_eq!(a.len(), SLUG_LEN);
        assert_ne!(a, b);
        for c in a.chars() {
            assert!(SLUG_ALPHABET.contains(&(c as u8)), "unexpected char {c}");
            assert!(!"lo01".contains(c), "confusable char {c} in slug");
        }
    }

    #[test]
    fn permutation_covers_every_tile_exactly_once() {
        let perm = random_permutation(50);
        let mut sorted = perm.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, (0..50).collect::<Vec<_>>());
    }

    #[test]
    fn permutations_are_actually_shuffled() {
        // A 400-tile identity ordering by chance is not worth guarding against.
        let perm = random_permutation(400);
        assert_ne!(perm, (0..400).collect::<Vec<_>>());
    }

    #[test]
    fn titles_are_trimmed_and_required() {
        assert!(validate_title("   ").is_err());
        assert!(validate_title("").is_err());
        assert_eq!(validate_title("  Team Q4  ").unwrap(), "Team Q4");
        assert!(validate_title(&"a".repeat(121)).is_err());
    }

    #[test]
    fn unit_minutes_must_be_positive() {
        assert!(validate_unit(0).is_err());
        assert!(validate_unit(-5).is_err());
        assert!(validate_unit(1).is_ok());
    }

    fn date(iso: &str) -> NaiveDate {
        iso.parse().expect("test date")
    }

    #[test]
    fn a_window_may_be_absent_entirely() {
        assert!(validate_window(None, None, None).is_ok());
    }

    #[test]
    fn the_end_date_may_not_precede_the_start() {
        assert!(validate_window(Some(date("2026-01-10")), Some(date("2026-01-09")), None).is_err());
        assert!(validate_window(Some(date("2026-01-10")), Some(date("2026-01-10")), None).is_ok());
        assert!(validate_window(Some(date("2026-01-10")), Some(date("2026-01-11")), None).is_ok());
    }

    #[test]
    fn one_bound_alone_is_a_valid_window() {
        assert!(validate_window(Some(date("2026-01-10")), None, None).is_ok());
        assert!(validate_window(None, Some(date("2026-01-10")), None).is_ok());
    }

    #[test]
    fn an_auto_archive_term_must_be_non_negative_and_have_an_end_to_count_from() {
        assert!(validate_window(None, Some(date("2026-01-20")), Some(-1)).is_err());
        assert!(validate_window(None, Some(date("2026-01-20")), Some(0)).is_ok());
        assert!(validate_window(None, Some(date("2026-01-20")), Some(15)).is_ok());
        // Nothing to count from, so the term could never fire.
        assert!(validate_window(None, None, Some(15)).is_err());
    }
}
