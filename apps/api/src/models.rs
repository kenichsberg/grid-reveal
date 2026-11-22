//! Database row types.

use chrono::{DateTime, Duration, NaiveDate, Utc};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: Option<String>,
    pub password_hash: String,
    pub is_admin: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct Picture {
    pub id: Uuid,
    pub storage_key: String,
    pub width: i32,
    pub height: i32,
    pub uploaded_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

impl Picture {
    pub fn aspect(&self) -> f64 {
        if self.height <= 0 {
            1.0
        } else {
            f64::from(self.width) / f64::from(self.height)
        }
    }
}

/// Where a screen sits in its own lifetime.
///
/// Derived on every read from the stored window and today's date rather than
/// stored, for the same reason the board itself is derived: there is no
/// scheduler in this service, and a stored state would need one to stay true.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenPhase {
    /// The window has not opened yet. Visible, but nothing may be written.
    Upcoming,
    /// Open for logging: the caller's day falls inside the screen's period.
    Active,
    /// The window has closed. Read-only, but still listed and still shown with
    /// its board and completion celebration intact.
    Ended,
    /// Archived by an administrator, or automatically once the retention term
    /// after `ends_on` has passed. Moves to the archived section of the list.
    Archived,
}

#[derive(Debug, Clone, FromRow)]
pub struct Screen {
    pub id: Uuid,
    pub slug: String,
    pub title: String,
    pub picture_id: Uuid,
    pub cols: i32,
    pub rows: i32,
    pub unit_minutes: i32,
    pub permutation: Vec<i32>,
    pub archived_at: Option<DateTime<Utc>>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    /// Inclusive first day of the active period.
    pub starts_on: NaiveDate,
    /// Inclusive last day of the active period.
    pub ends_on: NaiveDate,
    /// Days after `ends_on` before the screen archives itself; `None` disables
    /// automatic archiving.
    pub auto_archive_days: Option<i32>,
    /// Heading above the board's progress gauge. `None` leaves the client to
    /// its own neutral default, which is how a screen reads until an
    /// administrator gives it wording of its own.
    pub progress_title: Option<String>,
    /// What one logged session is called on this screen, e.g. "Daimoku". The
    /// client builds "Add …" and "Edit …" from it, so this is a bare noun and
    /// never a whole button label. `None` falls back to "entry".
    pub entry_noun: Option<String>,
}

impl Screen {
    pub fn total_tiles(&self) -> i32 {
        self.cols * self.rows
    }

    /// Whether an administrator archived this screen by hand.
    ///
    /// Distinct from [`Screen::is_archived_on`], which also accounts for the
    /// automatic archive. Only the archive and unarchive handlers care about
    /// this difference; everything that renders state wants the other one.
    pub fn is_archived(&self) -> bool {
        self.archived_at.is_some()
    }

    /// The day the screen archives itself, if it ever does.
    /// Still an `Option`: the period is mandatory, but the retention term that
    /// counts from its end is not.
    pub fn auto_archive_on(&self) -> Option<NaiveDate> {
        let days = self.auto_archive_days?;
        self.ends_on
            .checked_add_signed(Duration::days(i64::from(days)))
    }

    pub fn is_archived_on(&self, today: NaiveDate) -> bool {
        if self.is_archived() {
            return true;
        }
        match self.auto_archive_on() {
            Some(archive_on) => today > archive_on,
            None => false,
        }
    }

    pub fn phase(&self, today: NaiveDate) -> ScreenPhase {
        if self.is_archived_on(today) {
            return ScreenPhase::Archived;
        }
        if today < self.starts_on {
            return ScreenPhase::Upcoming;
        }
        if today > self.ends_on {
            return ScreenPhase::Ended;
        }
        ScreenPhase::Active
    }

    /// Whether entries and goals may be written today.
    pub fn accepts_writes(&self, today: NaiveDate) -> bool {
        self.phase(today) == ScreenPhase::Active
    }

    /// Whether a calendar date falls inside the active period.
    ///
    /// Used to bound `occurred_at`, so that backdating cannot reach outside the
    /// window the administrator set.
    pub fn covers(&self, date: NaiveDate) -> bool {
        date >= self.starts_on && date <= self.ends_on
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct Entry {
    pub id: Uuid,
    pub screen_id: Uuid,
    pub user_id: Uuid,
    pub minutes: i32,
    pub occurred_at: NaiveDate,
    pub note: Option<String>,
    pub created_at: DateTime<Utc>,
    pub edited_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, FromRow)]
pub struct Goal {
    pub id: Uuid,
    pub screen_id: Uuid,
    pub text: String,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub position: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(iso: &str) -> NaiveDate {
        iso.parse().expect("test date")
    }

    /// Every screen has a period, so the helper takes one.
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
    fn phase_walks_the_window() {
        let screen = screen("2026-01-10", "2026-01-20", None);
        assert_eq!(screen.phase(date("2026-01-09")), ScreenPhase::Upcoming);
        assert_eq!(screen.phase(date("2026-01-10")), ScreenPhase::Active);
        assert_eq!(screen.phase(date("2026-01-15")), ScreenPhase::Active);
        assert_eq!(screen.phase(date("2026-01-20")), ScreenPhase::Active);
        assert_eq!(screen.phase(date("2026-01-21")), ScreenPhase::Ended);
    }

    #[test]
    fn both_window_bounds_are_inclusive() {
        let screen = screen("2026-01-10", "2026-01-20", None);
        assert!(!screen.covers(date("2026-01-09")));
        assert!(screen.covers(date("2026-01-10")));
        assert!(screen.covers(date("2026-01-20")));
        assert!(!screen.covers(date("2026-01-21")));
    }

    #[test]
    fn an_ended_screen_stays_ended_without_an_auto_archive_term() {
        let screen = screen("2026-01-10", "2026-01-20", None);
        assert_eq!(screen.auto_archive_on(), None);
        assert_eq!(screen.phase(date("2030-01-01")), ScreenPhase::Ended);
    }

    #[test]
    fn auto_archive_runs_the_day_after_the_term_expires() {
        let screen = screen("2026-01-10", "2026-01-20", Some(15));
        assert_eq!(screen.auto_archive_on(), Some(date("2026-02-04")));
        assert_eq!(screen.phase(date("2026-02-04")), ScreenPhase::Ended);
        assert_eq!(screen.phase(date("2026-02-05")), ScreenPhase::Archived);
    }

    #[test]
    fn a_zero_day_term_archives_the_day_after_the_period_ends() {
        let screen = screen("2026-01-10", "2026-01-20", Some(0));
        assert_eq!(screen.phase(date("2026-01-20")), ScreenPhase::Active);
        assert_eq!(screen.phase(date("2026-01-21")), ScreenPhase::Archived);
    }

    #[test]
    fn a_manual_archive_outranks_the_window() {
        let screen = Screen {
            archived_at: Some(Utc::now()),
            ..screen("2026-01-10", "2026-01-20", None)
        };
        // Archived even mid-window, and even before it opens.
        assert_eq!(screen.phase(date("2026-01-15")), ScreenPhase::Archived);
        assert_eq!(screen.phase(date("2026-01-01")), ScreenPhase::Archived);
    }

    #[test]
    fn only_an_active_screen_accepts_writes() {
        let screen = screen("2026-01-10", "2026-01-20", Some(15));
        assert!(!screen.accepts_writes(date("2026-01-09")));
        assert!(screen.accepts_writes(date("2026-01-15")));
        assert!(!screen.accepts_writes(date("2026-01-21")));
        assert!(!screen.accepts_writes(date("2026-02-05")));
    }
}
