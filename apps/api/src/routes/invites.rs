//! One-time invite links.
//!
//! Two of these endpoints are public, which is unusual here: everything else
//! is behind a session, and a screen answers 404 to anyone who is not a member.
//! The exception is deliberate and narrow — the holder of a 120-bit token gets
//! to see the title and picture of the one screen they were invited to, which
//! is the minimum needed to know what they are agreeing to join, and is
//! exactly what the administrator meant to share when they sent the link.

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use axum_extra::extract::CookieJar;
use rand::Rng;
use sqlx::PgPool;
use uuid::Uuid;

use crate::access;
use crate::auth::extract::{AdminUser, ClientToday};
use crate::auth::{password, session};
use crate::dto::{
    AcceptInviteRequest, AcceptInviteResponse, CreateInviteRequest, CurrentUser, InvitePeek,
    InviteSummary,
};
use crate::error::{on_unique_violation, AppError, AppResult};
use crate::models::{Screen, ScreenInvite, ScreenPhase, User};
use crate::routes::screens::SLUG_ALPHABET;
use crate::routes::users::validate_username;
use crate::state::AppState;

/// 24 characters of the slug alphabet: 120 bits, and no character that is
/// easy to misread when the link is copied by hand.
const TOKEN_LEN: usize = 24;

pub const MAX_LABEL_LEN: usize = 80;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/screens/{slug}/invites", get(list).post(create))
        // Scoped to the screen rather than `/invites/{id}`: it puts the
        // administrator's access check on the screen, where every other one in
        // this codebase lives, and leaves `/invites/{token}` free for the
        // public endpoints.
        .route(
            "/screens/{slug}/invites/{id}",
            axum::routing::delete(revoke),
        )
        .route("/invites/{token}", get(peek))
        .route("/invites/{token}/accept", post(accept))
}

/// The invite row plus the username that accepted it, if any still exists.
#[derive(sqlx::FromRow)]
struct InviteRow {
    #[sqlx(flatten)]
    invite: ScreenInvite,
    accepted_username: Option<String>,
}

fn generate_token() -> String {
    let mut rng = rand::thread_rng();
    (0..TOKEN_LEN)
        .map(|_| SLUG_ALPHABET[rng.gen_range(0..SLUG_ALPHABET.len())] as char)
        .collect()
}

fn validate_label(value: Option<&str>) -> AppResult<Option<String>> {
    match value.map(str::trim).filter(|s| !s.is_empty()) {
        None => Ok(None),
        Some(text) if text.chars().count() > MAX_LABEL_LEN => Err(AppError::bad_request(format!(
            "the label must be {MAX_LABEL_LEN} characters or fewer"
        ))),
        Some(text) => Ok(Some(text.to_string())),
    }
}

fn to_summary(row: InviteRow, phase: ScreenPhase) -> InviteSummary {
    InviteSummary {
        open: row.invite.is_open(phase),
        id: row.invite.id,
        token: row.invite.token,
        label: row.invite.label,
        created_at: row.invite.created_at,
        accepted_at: row.invite.accepted_at,
        accepted_by: row.accepted_username,
        revoked_at: row.invite.revoked_at,
    }
}

async fn rows_for_screen(pool: &PgPool, screen_id: Uuid) -> AppResult<Vec<InviteRow>> {
    Ok(sqlx::query_as::<_, InviteRow>(
        "SELECT i.*, u.username AS accepted_username
         FROM screen_invites i
         LEFT JOIN users u ON u.id = i.accepted_by
         WHERE i.screen_id = $1
         ORDER BY i.created_at DESC",
    )
    .bind(screen_id)
    .fetch_all(pool)
    .await?)
}

/// Every invite for a screen, open or spent. The spent ones are the only
/// record of how each member got in, so they are not hidden.
async fn list(
    State(state): State<AppState>,
    admin: AdminUser,
    client_today: ClientToday,
    Path(slug): Path<String>,
) -> AppResult<Json<Vec<InviteSummary>>> {
    let screen = access::screen_for_admin(&state.db, &slug, &admin.user).await?;
    let phase = screen.phase(client_today.date());

    let rows = rows_for_screen(&state.db, screen.id).await?;
    Ok(Json(
        rows.into_iter().map(|row| to_summary(row, phase)).collect(),
    ))
}

async fn create(
    State(state): State<AppState>,
    admin: AdminUser,
    client_today: ClientToday,
    Path(slug): Path<String>,
    Json(body): Json<CreateInviteRequest>,
) -> AppResult<Json<InviteSummary>> {
    let screen = access::screen_for_admin(&state.db, &slug, &admin.user).await?;
    let label = validate_label(body.label.as_deref())?;

    // Retried rather than assumed unique, the way `unique_slug` is: a
    // collision is vanishingly unlikely and trivially recoverable.
    let mut last: Option<AppError> = None;
    for _ in 0..10 {
        let result = sqlx::query_as::<_, ScreenInvite>(
            "INSERT INTO screen_invites (screen_id, token, label, created_by)
             VALUES ($1, $2, $3, $4) RETURNING *",
        )
        .bind(screen.id)
        .bind(generate_token())
        .bind(label.as_deref())
        .bind(admin.user.id)
        .fetch_one(&state.db)
        .await;

        match result {
            Ok(invite) => {
                let phase = screen.phase(client_today.date());
                return Ok(Json(to_summary(
                    InviteRow {
                        invite,
                        accepted_username: None,
                    },
                    phase,
                )));
            }
            Err(e) => last = Some(on_unique_violation(e, "could not allocate an invite token")),
        }
    }

    Err(last.unwrap_or_else(|| AppError::internal("could not allocate an invite token")))
}

/// Withdraws an invite that has not been accepted.
///
/// An accepted one is left exactly as it is: the member who used it keeps
/// their account and their membership, and the record of how they joined
/// stays true.
async fn revoke(
    State(state): State<AppState>,
    admin: AdminUser,
    Path((slug, id)): Path<(String, Uuid)>,
) -> AppResult<Json<serde_json::Value>> {
    let screen = access::screen_for_admin(&state.db, &slug, &admin.user).await?;

    let revoked = sqlx::query(
        "UPDATE screen_invites SET revoked_at = now()
         WHERE id = $1 AND screen_id = $2 AND accepted_at IS NULL",
    )
    .bind(id)
    .bind(screen.id)
    .execute(&state.db)
    .await?;

    // Nothing updated means either no such invite on this screen, or one that
    // was already accepted. Neither is an error: revoking a spent invite is a
    // no-op by design, and the caller re-reads the list either way.
    tracing::debug!(%id, rows = revoked.rows_affected(), "invite revoked");

    Ok(Json(serde_json::json!({ "ok": true })))
}

/// Loads an invite and its screen by token.
async fn by_token(pool: &PgPool, token: &str) -> AppResult<(ScreenInvite, Screen)> {
    let invite = sqlx::query_as::<_, ScreenInvite>("SELECT * FROM screen_invites WHERE token = $1")
        .bind(token)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::NotFound)?;

    let screen = sqlx::query_as::<_, Screen>("SELECT * FROM screens WHERE id = $1")
        .bind(invite.screen_id)
        .fetch_one(pool)
        .await?;

    Ok((invite, screen))
}

/// Why this invite will not admit anyone, said plainly.
fn closed_reason(invite: &ScreenInvite, phase: ScreenPhase) -> Option<String> {
    if invite.accepted_at.is_some() {
        return Some("This invite has already been used.".into());
    }
    if invite.revoked_at.is_some() {
        return Some("This invite was withdrawn.".into());
    }
    match phase {
        ScreenPhase::Upcoming | ScreenPhase::Active => None,
        ScreenPhase::Ended => Some("This screen's period has ended.".into()),
        ScreenPhase::Archived => Some("This screen has been archived.".into()),
    }
}

/// Resolves a session if one is present, without rejecting when it is not.
async fn signed_in(state: &AppState, jar: &CookieJar) -> AppResult<Option<User>> {
    let Some(cookie) = jar.get(session::COOKIE_NAME) else {
        return Ok(None);
    };
    session::lookup(&state.db, cookie.value()).await
}

/// Public. Writes nothing, so a prefetch, a chat client's link preview, or a
/// reloaded page cannot spend the invite.
async fn peek(
    State(state): State<AppState>,
    jar: CookieJar,
    client_today: ClientToday,
    Path(token): Path<String>,
) -> AppResult<Json<InvitePeek>> {
    let (invite, screen) = by_token(&state.db, &token).await?;
    let phase = screen.phase(client_today.date());

    let picture = sqlx::query_as::<_, crate::models::Picture>(
        "SELECT * FROM pictures WHERE id = $1",
    )
    .bind(screen.picture_id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(InvitePeek {
        screen_title: screen.title,
        picture_url: state.storage.public_url(&picture.storage_key),
        open: invite.is_open(phase),
        closed_reason: closed_reason(&invite, phase),
        viewer_is_signed_in: signed_in(&state, &jar).await?.is_some(),
    }))
}

/// Public. The only endpoint that spends an invite.
///
/// Everything happens in one transaction, so a taken username costs the person
/// nothing: the invite rolls back to unused and they can try another.
async fn accept(
    State(state): State<AppState>,
    jar: CookieJar,
    client_today: ClientToday,
    Path(token): Path<String>,
    Json(body): Json<AcceptInviteRequest>,
) -> AppResult<(CookieJar, Json<AcceptInviteResponse>)> {
    let (invite, screen) = by_token(&state.db, &token).await?;
    let phase = screen.phase(client_today.date());

    if !invite.is_open(phase) {
        return Err(AppError::bad_request(
            closed_reason(&invite, phase)
                .unwrap_or_else(|| "This invite is no longer open.".into()),
        ));
    }

    let existing = signed_in(&state, &jar).await?;

    // Prepared before the transaction so the expensive hash is not held across
    // it, and so a malformed request fails before anything is written.
    let new_account = match &existing {
        Some(_) => None,
        None => {
            let username = validate_username(body.username.as_deref().unwrap_or(""))?;
            let supplied = body.password.as_deref().unwrap_or("");
            password::validate_password_len(supplied)?;
            Some((username, password::hash_password(supplied)?))
        }
    };

    let mut tx = state.db.begin().await?;

    // Single use, decided here rather than by the check above: two acceptances
    // racing both pass that check, and only one of them updates a row.
    let consumed: Option<(Uuid,)> = sqlx::query_as(
        "UPDATE screen_invites SET accepted_at = now()
         WHERE id = $1 AND accepted_at IS NULL AND revoked_at IS NULL
         RETURNING id",
    )
    .bind(invite.id)
    .fetch_optional(&mut *tx)
    .await?;

    if consumed.is_none() {
        return Err(AppError::bad_request("This invite has already been used."));
    }

    let user = match (existing, new_account) {
        (Some(user), _) => user,
        (None, Some((username, hash))) => sqlx::query_as::<_, User>(
            "INSERT INTO users (username, password_hash, is_admin)
             VALUES ($1, $2, FALSE) RETURNING *",
        )
        .bind(&username)
        .bind(&hash)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| on_unique_violation(e, "that username is already taken"))?,
        // Unreachable: `new_account` is `Some` whenever there is no session.
        (None, None) => return Err(AppError::bad_request("a username and password are required")),
    };

    sqlx::query("UPDATE screen_invites SET accepted_by = $1 WHERE id = $2")
        .bind(user.id)
        .bind(invite.id)
        .execute(&mut *tx)
        .await?;

    sqlx::query(
        "INSERT INTO screen_members (screen_id, user_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(screen.id)
    .bind(user.id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    let token = session::create(&state.db, user.id, state.config.session_ttl_days).await?;
    let jar = jar.add(session::build_cookie(&state.config, token));

    Ok((
        jar,
        Json(AcceptInviteResponse {
            user: CurrentUser {
                id: user.id,
                username: user.username,
                email: user.email,
                is_admin: user.is_admin,
            },
            slug: screen.slug,
        }),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_token_is_long_and_free_of_confusable_characters() {
        let token = generate_token();
        assert_eq!(token.chars().count(), TOKEN_LEN);
        assert!(token.chars().all(|c| SLUG_ALPHABET.contains(&(c as u8))));
        for confusable in ['l', 'o', '0', '1'] {
            assert!(!token.contains(confusable), "{token} contains {confusable}");
        }
    }

    #[test]
    fn two_tokens_differ() {
        assert_ne!(generate_token(), generate_token());
    }

    #[test]
    fn a_label_is_trimmed_bounded_and_optional() {
        assert_eq!(validate_label(None).unwrap(), None);
        assert_eq!(validate_label(Some("  ")).unwrap(), None);
        assert_eq!(validate_label(Some("  Mika  ")).unwrap(), Some("Mika".into()));
        assert!(validate_label(Some(&"x".repeat(MAX_LABEL_LEN + 1))).is_err());
    }

    fn invite() -> ScreenInvite {
        ScreenInvite {
            id: Uuid::nil(),
            screen_id: Uuid::nil(),
            token: "t".into(),
            label: None,
            created_by: Uuid::nil(),
            created_at: chrono::Utc::now(),
            accepted_at: None,
            accepted_by: None,
            revoked_at: None,
        }
    }

    #[test]
    fn an_open_invite_has_nothing_to_explain() {
        assert_eq!(closed_reason(&invite(), ScreenPhase::Active), None);
        assert_eq!(closed_reason(&invite(), ScreenPhase::Upcoming), None);
    }

    #[test]
    fn each_way_of_being_closed_says_so_differently() {
        let spent = ScreenInvite {
            accepted_at: Some(chrono::Utc::now()),
            ..invite()
        };
        assert!(closed_reason(&spent, ScreenPhase::Active)
            .unwrap()
            .contains("already been used"));

        let withdrawn = ScreenInvite {
            revoked_at: Some(chrono::Utc::now()),
            ..invite()
        };
        assert!(closed_reason(&withdrawn, ScreenPhase::Active)
            .unwrap()
            .contains("withdrawn"));

        assert!(closed_reason(&invite(), ScreenPhase::Ended)
            .unwrap()
            .contains("period has ended"));
        assert!(closed_reason(&invite(), ScreenPhase::Archived)
            .unwrap()
            .contains("archived"));
    }

    /// Spending it outranks the screen's state: someone who already used a
    /// link should be told that, not that the screen has since ended.
    #[test]
    fn a_spent_invite_explains_itself_before_the_screen_does() {
        let spent = ScreenInvite {
            accepted_at: Some(chrono::Utc::now()),
            ..invite()
        };
        assert!(closed_reason(&spent, ScreenPhase::Ended)
            .unwrap()
            .contains("already been used"));
    }
}
