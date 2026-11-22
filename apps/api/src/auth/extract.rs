//! Request extractors that resolve and authorise the caller, and that settle
//! what day it is for them.

use std::convert::Infallible;

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum_extra::extract::CookieJar;
use chrono::{NaiveDate, Utc};

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

/// Header carrying the caller's own calendar day, as `YYYY-MM-DD`.
///
/// Lower-case because that is how it must be spelled in the CORS allow-list,
/// and one spelling shared between the layer and the extractor cannot drift.
pub const CLIENT_DATE_HEADER: &str = "x-client-date";

/// How far the caller's day may sit from the server's, in days.
///
/// Real offsets run from UTC-12 to UTC+14 — twenty-six hours — so an honest
/// client is never more than one calendar day away. The bound is what keeps a
/// wrong device clock, or a deliberate one, from moving a screen's deadline.
const MAX_SKEW_DAYS: i64 = 1;

/// The calendar day a request happens on, according to whoever sent it.
///
/// Every day-granularity decision belongs to the caller's day rather than the
/// server's: a member logging at 00:30 local time, with the server still on
/// yesterday's UTC date, is not logging in the future, and a screen whose
/// period opens that day has opened for them.
///
/// Advisory within the bound, ignored outside it, and absent means the server's
/// own date — which is exactly how the API behaved before this existed.
#[derive(Debug, Clone, Copy)]
pub struct ClientToday(NaiveDate);

impl ClientToday {
    pub fn date(self) -> NaiveDate {
        self.0
    }

    /// The resolution rules, kept free of the request so they can be tested
    /// against a fixed server date rather than the real clock.
    fn resolve(supplied: Option<&str>, server_today: NaiveDate) -> NaiveDate {
        let Some(raw) = supplied else {
            return server_today;
        };

        let Ok(date) = NaiveDate::parse_from_str(raw.trim(), "%Y-%m-%d") else {
            tracing::debug!(value = raw, "ignoring unparseable client date");
            return server_today;
        };

        if (date - server_today).num_days().abs() > MAX_SKEW_DAYS {
            tracing::debug!(
                %date,
                %server_today,
                "ignoring client date beyond the permitted skew"
            );
            return server_today;
        }

        date
    }
}

/// Infallible by design. A malformed or far-off date degrades to the server's
/// own, because a stale tab or a wrong clock should not make the application
/// unusable, and a 400 on a header the member never knowingly set would be
/// close to impossible for them to diagnose.
impl<S: Send + Sync> FromRequestParts<S> for ClientToday {
    type Rejection = Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let supplied = parts
            .headers
            .get(CLIENT_DATE_HEADER)
            .and_then(|value| value.to_str().ok());

        Ok(ClientToday(Self::resolve(supplied, Utc::now().date_naive())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    const SERVER: &str = "2026-11-19";

    #[test]
    fn absent_header_falls_back_to_the_server_date() {
        assert_eq!(ClientToday::resolve(None, date(SERVER)), date(SERVER));
    }

    #[test]
    fn the_callers_own_day_is_honoured() {
        assert_eq!(
            ClientToday::resolve(Some("2026-11-19"), date(SERVER)),
            date("2026-11-19")
        );
    }

    #[test]
    fn a_day_either_side_is_within_the_bound() {
        assert_eq!(
            ClientToday::resolve(Some("2026-11-18"), date(SERVER)),
            date("2026-11-18")
        );
        assert_eq!(
            ClientToday::resolve(Some("2026-11-20"), date(SERVER)),
            date("2026-11-20")
        );
    }

    #[test]
    fn two_days_out_is_discarded() {
        assert_eq!(
            ClientToday::resolve(Some("2026-11-17"), date(SERVER)),
            date(SERVER)
        );
        assert_eq!(
            ClientToday::resolve(Some("2026-11-21"), date(SERVER)),
            date(SERVER)
        );
    }

    #[test]
    fn a_closed_period_cannot_be_reopened_by_claiming_an_old_date() {
        assert_eq!(
            ClientToday::resolve(Some("2026-10-01"), date(SERVER)),
            date(SERVER)
        );
    }

    #[test]
    fn malformed_values_are_discarded() {
        for raw in ["", "today", "19-11-2026", "2026-13-01", "2026-11-19T10:00:00Z"] {
            assert_eq!(ClientToday::resolve(Some(raw), date(SERVER)), date(SERVER));
        }
    }

    #[test]
    fn surrounding_whitespace_is_tolerated() {
        assert_eq!(
            ClientToday::resolve(Some("  2026-11-20  "), date(SERVER)),
            date("2026-11-20")
        );
    }
}
