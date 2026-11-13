//! Request and response types.
//!
//! These derive `TS`, so `scripts/gen-types.sh` emits matching TypeScript into
//! `apps/web/src/lib/api/types/`. The contract is therefore generated from the
//! Rust definitions rather than maintained twice.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Auth
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct CurrentUser {
    #[ts(type = "string")]
    pub id: Uuid,
    pub username: String,
    pub email: Option<String>,
    pub is_admin: bool,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

// ---------------------------------------------------------------------------
// Users (administrator only)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateUserRequest {
    pub username: String,
    pub password: String,
    pub email: Option<String>,
    #[serde(default)]
    pub is_admin: bool,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct UserSummary {
    #[ts(type = "string")]
    pub id: Uuid,
    pub username: String,
    pub email: Option<String>,
    pub is_admin: bool,
    #[ts(type = "string")]
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct SetPasswordRequest {
    pub new_password: String,
}

// ---------------------------------------------------------------------------
// Pictures
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct PictureSummary {
    #[ts(type = "string")]
    pub id: Uuid,
    pub url: String,
    pub width: i32,
    pub height: i32,
    #[ts(type = "string")]
    pub created_at: DateTime<Utc>,
}

/// Returned when a hard delete is refused because screens still use the
/// picture, so the client can say how many.
#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct PictureReferences {
    #[ts(type = "number")]
    pub screen_count: i64,
}

// ---------------------------------------------------------------------------
// Geometry
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct GeometryPreviewRequest {
    #[ts(type = "string")]
    pub picture_id: Uuid,
    #[ts(type = "number")]
    pub target_total_minutes: i64,
    pub unit_minutes: i32,
    /// When present the solver keeps this column count and derives rows,
    /// which is what the creation form's column control sends.
    pub cols: Option<i32>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct GeometryPreview {
    pub cols: i32,
    pub rows: i32,
    pub total_tiles: i32,
    /// `cols * rows * unit_minutes`. May differ from the requested target:
    /// not every tile count factors into a rectangle matching the picture.
    #[ts(type = "number")]
    pub exact_total_minutes: i64,
    #[ts(type = "number")]
    pub requested_total_minutes: i64,
    /// 1.0 means exactly square cells; larger is more oblong.
    pub cell_squareness: f64,
}

// ---------------------------------------------------------------------------
// Screens
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateScreenRequest {
    pub title: String,
    #[ts(type = "string")]
    pub picture_id: Uuid,
    #[ts(type = "number")]
    pub target_total_minutes: i64,
    pub unit_minutes: i32,
    /// Optional override from the creation preview.
    pub cols: Option<i32>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct ScreenSummary {
    pub slug: String,
    pub title: String,
    pub picture_url: String,
    pub archived: bool,
    #[ts(type = "number")]
    pub total_minutes: i64,
    pub revealed_count: i32,
    pub total_tiles: i32,
}

/// One row of the administrator directory.
///
/// Administrators may open any screen, so they need a way to enumerate the
/// ones they are not a member of; the personal screen list stays
/// member-scoped for everyone.
#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct AdminScreenEntry {
    pub slug: String,
    pub title: String,
    pub picture_url: String,
    pub archived: bool,
    #[ts(type = "number")]
    pub total_minutes: i64,
    pub revealed_count: i32,
    pub total_tiles: i32,
    /// Whether the viewing administrator participates in this screen.
    pub viewer_is_member: bool,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct ScreenList {
    pub active: Vec<ScreenSummary>,
    pub archived: Vec<ScreenSummary>,
}

/// Everything needed to render a board.
///
/// `revealed_tiles` is the revealed PREFIX of the permutation only. The
/// unrevealed remainder is never sent: a member who could read it would know
/// the reveal order in advance.
#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct ScreenDetail {
    pub slug: String,
    pub title: String,
    pub picture_url: String,
    pub picture_width: i32,
    pub picture_height: i32,
    pub cols: i32,
    pub rows: i32,
    pub unit_minutes: i32,
    pub total_tiles: i32,
    #[ts(type = "number")]
    pub participant_count: i64,
    #[ts(type = "number")]
    pub total_minutes: i64,
    pub revealed_count: i32,
    pub revealed_tiles: Vec<i32>,
    pub completed: bool,
    pub archived: bool,
    /// Whether the caller may administer this screen, so the client can show
    /// or hide controls without a second request.
    pub viewer_is_admin: bool,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct UpdateScreenRequest {
    pub title: Option<String>,
    pub cols: Option<i32>,
    pub rows: Option<i32>,
    pub unit_minutes: Option<i32>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CloneScreenRequest {
    pub title: String,
    #[ts(type = "string")]
    pub picture_id: Uuid,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct DeleteScreenRequest {
    /// Must match the screen's title exactly.
    pub confirm_title: String,
}

// ---------------------------------------------------------------------------
// Membership (administrator only)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct MemberSummary {
    #[ts(type = "string")]
    pub user_id: Uuid,
    pub username: String,
    #[ts(type = "string")]
    pub joined_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct AddMemberRequest {
    #[ts(type = "string")]
    pub user_id: Uuid,
}

/// What deleting a departing member's entries would cost the board, shown in
/// the removal dialog before the administrator commits.
#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct RemovalImpact {
    #[ts(type = "number")]
    pub entry_count: i64,
    #[ts(type = "number")]
    pub minutes: i64,
    pub revealed_tiles_lost: i32,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct RemoveMemberRequest {
    /// Explicit choice: keep their logged time in the screen total, or remove
    /// it and let the board rewind.
    pub delete_entries: bool,
}

// ---------------------------------------------------------------------------
// Entries
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateEntryRequest {
    pub hours: i32,
    pub minutes: i32,
    /// Defaults to today when absent. Never in the future.
    #[ts(type = "string | null")]
    pub occurred_at: Option<NaiveDate>,
    pub note: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct UpdateEntryRequest {
    pub hours: i32,
    pub minutes: i32,
    #[ts(type = "string | null")]
    pub occurred_at: Option<NaiveDate>,
    pub note: Option<String>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct EntrySummary {
    #[ts(type = "string")]
    pub id: Uuid,
    pub minutes: i32,
    #[ts(type = "string")]
    pub occurred_at: NaiveDate,
    pub note: Option<String>,
    #[ts(type = "string")]
    pub created_at: DateTime<Utc>,
    #[ts(type = "string | null")]
    pub edited_at: Option<DateTime<Utc>>,
}

/// The result of submitting progress.
///
/// `your_tiles` is what the client animates. Tiles revealed by other members
/// since the client last rendered are the difference between its previous
/// `revealed_count` and this one; those are snapped into place without
/// animation so the animation always means "this is what you just did".
#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct SubmitEntryResponse {
    pub entry: EntrySummary,
    #[ts(type = "number")]
    pub total_minutes: i64,
    pub revealed_count: i32,
    pub your_tiles: Vec<i32>,
    /// Every revealed tile, so a client whose view had drifted can resynchronise
    /// without a second request.
    pub revealed_tiles: Vec<i32>,
    pub completed: bool,
}

// ---------------------------------------------------------------------------
// Goals
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateGoalRequest {
    pub text: String,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct GoalSummary {
    #[ts(type = "string")]
    pub id: Uuid,
    pub text: String,
    #[ts(type = "string")]
    pub created_at: DateTime<Utc>,
    /// Lets the author see a delete control. This is the only signal of one's
    /// own authorship a non-administrator receives.
    pub is_mine: bool,
    /// Populated for administrators only; `None` for everyone else.
    pub author_username: Option<String>,
}
