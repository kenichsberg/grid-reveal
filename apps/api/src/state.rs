//! Shared application state.

use std::sync::Arc;

use sqlx::PgPool;

use crate::config::Config;
use crate::storage::Storage;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub config: Arc<Config>,
    pub storage: Arc<Storage>,
}
