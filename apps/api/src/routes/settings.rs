//! Instance-wide settings.
//!
//! One endpoint here is public, which is unusual in this codebase: the login
//! and invite pages print the application's name before anyone has signed in,
//! so the name cannot ride on an authenticated response. A handler is public
//! precisely by naming no auth extractor, as the invite peek is; writing it
//! still takes `AdminUser`.
//!
//! The settings are a single row, seeded by the migration that created the
//! table, so a read always finds one.

use axum::extract::State;
use axum::http::header;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};

use crate::auth::extract::AdminUser;
use crate::dto::{SiteSettings, UpdateSettingsRequest};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// Long enough for a sentence-ish name, short enough to sit in a header bar.
/// The same limit the per-screen labels use, and the database repeats it.
const MAX_APP_NAME_LEN: usize = 60;

pub fn router() -> Router<AppState> {
    Router::new().route("/settings", get(read).patch(update))
}

fn validate_app_name(name: &str) -> AppResult<String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(AppError::bad_request("a name is required"));
    }
    if trimmed.chars().count() > MAX_APP_NAME_LEN {
        return Err(AppError::bad_request(format!(
            "the name must be {MAX_APP_NAME_LEN} characters or fewer"
        )));
    }
    Ok(trimmed.to_string())
}

async fn load(state: &AppState) -> AppResult<SiteSettings> {
    let (app_name,): (String,) = sqlx::query_as("SELECT app_name FROM site_settings")
        .fetch_one(&state.db)
        .await?;
    Ok(SiteSettings { app_name })
}

/// Public: the name is on the login page, before there is a session.
async fn read(State(state): State<AppState>) -> AppResult<Response> {
    let settings = load(&state).await?;

    // A rename has to be visible on the next load. This is the first endpoint
    // here a shared cache could hold, so it is told not to.
    Ok((
        [(header::CACHE_CONTROL, "no-store")],
        Json(settings),
    )
        .into_response())
}

async fn update(
    State(state): State<AppState>,
    _admin: AdminUser,
    Json(body): Json<UpdateSettingsRequest>,
) -> AppResult<Json<SiteSettings>> {
    let app_name = validate_app_name(&body.app_name)?;

    sqlx::query("UPDATE site_settings SET app_name = $1, updated_at = now()")
        .bind(&app_name)
        .execute(&state.db)
        .await?;

    Ok(Json(SiteSettings { app_name }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_is_trimmed_and_bounded() {
        assert!(validate_app_name("").is_err());
        assert!(validate_app_name("   ").is_err());
        assert_eq!(validate_app_name("  Daimoku  ").unwrap(), "Daimoku");
        assert_eq!(
            validate_app_name(&"a".repeat(MAX_APP_NAME_LEN)).unwrap().len(),
            MAX_APP_NAME_LEN
        );
        assert!(validate_app_name(&"a".repeat(MAX_APP_NAME_LEN + 1)).is_err());
    }
}
