//! Screens: creation, access, membership, geometry, and lifecycle.

use axum::extract::{Path, Query, State};
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

async fn to_summary(state: &AppState, screen: &Screen) -> AppResult<ScreenSummary> {
    let (total_minutes, revealed_count) = access::board_state(&state.db, screen).await?;
    let picture = picture_of(&state.db, screen.picture_id).await?;

    Ok(ScreenSummary {
        slug: screen.slug.clone(),
        title: screen.title.clone(),
        picture_url: state.storage.public_url(&picture.storage_key),
        archived: screen.is_archived(),
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

    let mut active = Vec::new();
    let mut archived = Vec::new();
    for screen in &screens {
        let summary = to_summary(&state, screen).await?;
        if screen.is_archived() {
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

    let mut out = Vec::with_capacity(screens.len());
    for screen in &screens {
        let summary = to_summary(&state, screen).await?;
        out.push(AdminScreenEntry {
            slug: summary.slug,
            title: summary.title,
            picture_url: summary.picture_url,
            archived: summary.archived,
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
        archived: screen.is_archived(),
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
           (slug, title, picture_id, cols, rows, unit_minutes, permutation, created_by)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING *",
    )
    .bind(&slug)
    .bind(&title)
    .bind(picture.id)
    .bind(solved.cols)
    .bind(solved.rows)
    .bind(body.unit_minutes)
    .bind(&permutation)
    .bind(admin.user.id)
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

/// Edits the title, and optionally the geometry.
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

    detail(State(state), AuthUser { user: admin.user, token: admin.token }, Path(slug)).await
}

// ---------------------------------------------------------------------------
// Membership
// ---------------------------------------------------------------------------

/// Administrators only: members never learn who else is on a screen, only how
/// many people there are.
async fn members(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(slug): Path<String>,
) -> AppResult<Json<Vec<MemberSummary>>> {
    let screen = access::screen_for_admin(&state.db, &slug, &admin.user).await?;

    let rows = sqlx::query_as::<_, (Uuid, String, chrono::DateTime<chrono::Utc>)>(
        "SELECT u.id, u.username, m.joined_at
         FROM screen_members m JOIN users u ON u.id = m.user_id
         WHERE m.screen_id = $1
         ORDER BY lower(u.username)",
    )
    .bind(screen.id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(
        rows.into_iter()
            .map(|(user_id, username, joined_at)| MemberSummary {
                user_id,
                username,
                joined_at,
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
/// The title and picture are not inherited: both are required, because a clone
/// is a new period rather than a copy.
async fn clone_screen(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(slug): Path<String>,
    Json(body): Json<CloneScreenRequest>,
) -> AppResult<Json<ScreenDetail>> {
    let source = access::screen_for_admin(&state.db, &slug, &admin.user).await?;
    let title = validate_title(&body.title)?;
    let picture = selectable_picture(&state.db, body.picture_id).await?;

    let new_slug = unique_slug(&state.db).await?;
    let permutation = random_permutation(source.total_tiles());

    let mut tx = state.db.begin().await?;

    let created = sqlx::query_as::<_, Screen>(
        "INSERT INTO screens
           (slug, title, picture_id, cols, rows, unit_minutes, permutation, created_by)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING *",
    )
    .bind(&new_slug)
    .bind(&title)
    .bind(picture.id)
    .bind(source.cols)
    .bind(source.rows)
    .bind(source.unit_minutes)
    .bind(&permutation)
    .bind(admin.user.id)
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

async fn unarchive(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(slug): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let screen = access::screen_for_admin(&state.db, &slug, &admin.user).await?;
    sqlx::query("UPDATE screens SET archived_at = NULL WHERE id = $1")
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
}
