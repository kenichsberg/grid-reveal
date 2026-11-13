//! First-boot administrator seeding.

use sqlx::PgPool;

use crate::auth::password;
use crate::config::Config;
use crate::error::AppResult;

/// Creates the configured administrator when no administrator exists.
///
/// Deliberately a no-op once any administrator is present: the environment
/// variables are a bootstrap, not a source of truth, so editing them later
/// must not silently rewrite a live account.
pub async fn ensure_admin(pool: &PgPool, config: &Config) -> AppResult<()> {
    let (admin_count,): (i64,) =
        sqlx::query_as("SELECT COUNT(*)::bigint FROM users WHERE is_admin")
            .fetch_one(pool)
            .await?;

    if admin_count > 0 {
        tracing::info!(admin_count, "administrator already exists; skipping bootstrap");
        return Ok(());
    }

    let hash = password::hash_password(&config.admin_password)?;

    sqlx::query(
        "INSERT INTO users (username, password_hash, is_admin) VALUES ($1, $2, TRUE)
         ON CONFLICT DO NOTHING",
    )
    .bind(&config.admin_username)
    .bind(&hash)
    .execute(pool)
    .await?;

    tracing::info!(
        username = %config.admin_username,
        "bootstrap administrator created"
    );
    Ok(())
}
