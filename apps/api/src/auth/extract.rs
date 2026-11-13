//! Request extractors that resolve and authorise the caller.

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum_extra::extract::CookieJar;

use crate::auth::session;
use crate::error::AppError;
use crate::models::User;
use crate::state::AppState;

/// An authenticated caller. Rejects missing, unknown, and expired tokens with
/// 401.
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user: User,
    /// The presenting token, so a password change can revoke every *other*
    /// session without logging this one out.
    pub token: String,
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let jar = CookieJar::from_headers(&parts.headers);
        let token = jar
            .get(session::COOKIE_NAME)
            .map(|c| c.value().to_string())
            .ok_or(AppError::Unauthorized)?;

        let user = session::lookup(&state.db, &token)
            .await?
            .ok_or(AppError::Unauthorized)?;

        Ok(AuthUser { user, token })
    }
}

/// An authenticated administrator. Non-administrators get 403, which is safe
/// here because these endpoints are not screen-scoped and so disclose nothing
/// about which screens exist.
#[derive(Debug, Clone)]
pub struct AdminUser {
    pub user: User,
    pub token: String,
}

impl FromRequestParts<AppState> for AdminUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let AuthUser { user, token } = AuthUser::from_request_parts(parts, state).await?;
        if !user.is_admin {
            return Err(AppError::Forbidden);
        }
        Ok(AdminUser { user, token })
    }
}
