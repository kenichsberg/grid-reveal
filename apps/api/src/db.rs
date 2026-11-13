//! Database pool and migrations.

use std::time::Duration;

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

use crate::error::{AppError, AppResult};

/// Opens the pool. Sized for a single small VM talking to a free-tier database,
/// not for throughput.
pub async fn connect(database_url: &str) -> AppResult<PgPool> {
    PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(10))
        .connect(database_url)
        .await
        .map_err(|e| AppError::internal(format!("could not connect to the database: {e}")))
}

/// Applies pending migrations at startup, so a deploy never serves against an
/// older schema than the binary expects.
pub async fn migrate(pool: &PgPool) -> AppResult<()> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .map_err(|e| AppError::internal(format!("migrations failed: {e}")))?;
    tracing::info!("migrations applied");
    Ok(())
}
