//! Request and response types.
//!
//! These derive `TS`, so `scripts/gen-types.sh` emits matching TypeScript into
//! `apps/web/src/lib/api/types/`. The contract is therefore generated from the
//! Rust definitions rather than maintained twice.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use crate::models::ScreenPhase;

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
    /// What a tile is worth in the solved grid. Raised above the request when
    /// the target would otherwise need more tiles than the solver allows.
    pub unit_minutes: i32,
    /// What the administrator asked a tile to be worth, so the form can say
    /// plainly that it changed.
    pub requested_unit_minutes: i32,
    /// 1.0 means exactly square cells; larger is more oblong.
    pub cell_squareness: f64,
}

// ---------------------------------------------------------------------------
// Screens
// ---------------------------------------------------------------------------

/// Where a screen sits in its own lifetime, as the client sees it.
///
/// The wire mirror of [`crate::models::ScreenPhase`]. It is derived on the
/// server from the stored window and the current date, so the client must
/// render this verbatim and never recompute it from the dates: the two could
/// only ever disagree, and the client would lose.
#[derive(Debug, Clone, Copy, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Upcoming,
    Active,
    Ended,
    Archived,
}

impl From<ScreenPhase> for Phase {
    fn from(phase: ScreenPhase) -> Self {
        match phase {
            ScreenPhase::Upcoming => Phase::Upcoming,
            ScreenPhase::Active => Phase::Active,
            ScreenPhase::Ended => Phase::Ended,
            ScreenPhase::Archived => Phase::Archived,
        }
    }
}

/// The active period an administrator sets on a screen.
///
/// Carried whole on update rather than as three loose fields, so that a
/// request can still distinguish "leave the period alone" (the field is
/// absent) from "replace it" (the field is present). What it can no longer
/// express is "clear this bound": both dates are required, so a window missing
/// one is a 400 that names it. Create and clone take the three fields inline.
#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct ScreenWindow {
    #[ts(type = "string")]
    pub starts_on: NaiveDate,
    #[ts(type = "string")]
    pub ends_on: NaiveDate,
    /// Days after `ends_on` before the screen archives itself; `null` never.
    pub auto_archive_days: Option<i32>,
}

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
    /// The active period. Both dates are required: a screen without one never
    /// ends, never archives itself, and never becomes read-only. The browser
    /// pre-fills the start with the administrator's own day.
    #[ts(type = "string")]
    pub starts_on: NaiveDate,
    #[ts(type = "string")]
    pub ends_on: NaiveDate,
    pub auto_archive_days: Option<i32>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct ScreenSummary {
    pub slug: String,
    pub title: String,
    /// `None` until every tile is revealed.
    ///
    /// The list is a lobby, not a board. A card carrying the picture before
    /// the last tile fell would disclose the very thing the group is
    /// uncovering, so the URL is withheld rather than merely left unrendered —
    /// the same reason `ScreenDetail` sends only the revealed prefix of the
    /// permutation.
    pub picture_url: Option<String>,
    pub archived: bool,
    pub phase: Phase,
    /// Carried so a list can say when an upcoming screen opens, not merely
    /// that it has not started.
    #[ts(type = "string")]
    pub starts_on: NaiveDate,
    #[ts(type = "string")]
    pub ends_on: NaiveDate,
    #[ts(type = "number")]
    pub total_minutes: i64,
    pub revealed_count: i32,
    pub total_tiles: i32,
    /// How many people are on the screen. The card has room for one fact
    /// beneath the title, and who is in it says more than a figure the
    /// percentage above it already gives.
    #[ts(type = "number")]
    pub participant_count: i64,
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
    pub phase: Phase,
    /// Carried so a list can say when an upcoming screen opens, not merely
    /// that it has not started.
    #[ts(type = "string")]
    pub starts_on: NaiveDate,
    #[ts(type = "string")]
    pub ends_on: NaiveDate,
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
    /// True for a screen an administrator archived AND for one the automatic
    /// archive has retired, so every existing consumer of this flag keeps
    /// working without knowing about the window.
    pub archived: bool,
    /// The authoritative lifecycle state. Prefer this over `archived`: it also
    /// distinguishes a screen whose period has not opened from one whose
    /// period has closed.
    pub phase: Phase,
    #[ts(type = "string")]
    pub starts_on: NaiveDate,
    #[ts(type = "string")]
    pub ends_on: NaiveDate,
    pub auto_archive_days: Option<i32>,
    /// Heading above the progress gauge; `null` means the client's default.
    pub progress_title: Option<String>,
    /// What one logged session is called here; `null` means the client's
    /// default. A bare noun — the client builds "Add …" / "Edit …" from it.
    pub entry_noun: Option<String>,
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
    /// Absent leaves the window untouched. Present replaces it wholesale, so
    /// clearing one bound means sending the window with that field null.
    pub window: Option<ScreenWindow>,
    /// Absent leaves the heading alone; an empty string clears it back to the
    /// client's default. A label is one string, so it can carry "clear"
    /// itself — the window needed a nested struct only because it is three.
    pub progress_title: Option<String>,
    /// Absent leaves it alone; an empty string clears it. See
    /// [`Self::progress_title`].
    pub entry_noun: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CloneScreenRequest {
    pub title: String,
    #[ts(type = "string")]
    pub picture_id: Uuid,
    /// A clone is a new period rather than a copy, so it gets its own window
    /// rather than inheriting the original's — and, being a screen, it must
    /// have one.
    #[ts(type = "string")]
    pub starts_on: NaiveDate,
    #[ts(type = "string")]
    pub ends_on: NaiveDate,
    pub auto_archive_days: Option<i32>,
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
    /// Administrator of the whole system, not of this screen: the role is
    /// global. Shown so members know who to ask about the screen itself.
    pub is_admin: bool,
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

// ---------------------------------------------------------------------------
// Invites
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateInviteRequest {
    /// Who the link is meant for, for the administrator's own benefit. Blank
    /// clears it, the same as every other label in this API.
    pub label: Option<String>,
}

/// An invite as its administrator sees it.
///
/// Carries the token, because an administrator who closed the tab needs to be
/// able to copy the link again. The invited person never sees this shape.
#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct InviteSummary {
    #[ts(type = "string")]
    pub id: Uuid,
    pub token: String,
    pub label: Option<String>,
    #[ts(type = "string")]
    pub created_at: DateTime<Utc>,
    /// Derived per request from the screen's phase, so it is the same answer
    /// the invited person would get if they opened the link right now.
    pub open: bool,
    #[ts(type = "string | null")]
    pub accepted_at: Option<DateTime<Utc>>,
    /// The account that accepted, if it still exists.
    pub accepted_by: Option<String>,
    #[ts(type = "string | null")]
    pub revoked_at: Option<DateTime<Utc>>,
}

/// What the holder of a token is told before they commit to anything.
///
/// Deliberately thin: the screen's name and picture are what someone needs to
/// know what they are joining, and the slug is not among them — that arrives
/// with membership.
#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct InvitePeek {
    pub screen_title: String,
    pub picture_url: String,
    pub open: bool,
    /// Why it is closed, phrased for the person holding the link. `None` while
    /// the invite is open.
    pub closed_reason: Option<String>,
    /// Whether the caller is already signed in, so the page can offer to join
    /// with that account instead of asking for new credentials.
    pub viewer_is_signed_in: bool,
}

/// Credentials are absent when the caller already has a session: an invite
/// then joins the account they are signed in as, rather than making a second.
#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct AcceptInviteRequest {
    pub username: Option<String>,
    pub password: Option<String>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct AcceptInviteResponse {
    pub user: CurrentUser,
    /// Where to go next, which the caller could not have known before.
    pub slug: String,
}
