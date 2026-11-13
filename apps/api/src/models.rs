//! Database row types.

use chrono::{DateTime, NaiveDate, Utc};
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
}

impl Screen {
    pub fn total_tiles(&self) -> i32 {
        self.cols * self.rows
    }

    pub fn is_archived(&self) -> bool {
        self.archived_at.is_some()
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
