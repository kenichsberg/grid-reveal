use std::sync::Arc;

use grid_reveal_api::{bootstrap, config::Config, db, routes, state::AppState, storage::Storage};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "grid_reveal_api=info,tower_http=info".into()),
        )
        .init();

    if let Err(error) = run().await {
        // Configuration and startup failures are reported plainly rather than
        // as a panic backtrace: they are nearly always a missing variable.
        eprintln!("{error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_env()?;

    let db = db::connect(&config.database_url).await?;
    db::migrate(&db).await?;
    bootstrap::ensure_admin(&db, &config).await?;

    let storage = Storage::from_config(&config.storage)?;

    let bind_addr = config.bind_addr.clone();
    let state = AppState {
        db,
        config: Arc::new(config),
        storage: Arc::new(storage),
    };

    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    tracing::info!(address = %bind_addr, "listening");

    axum::serve(listener, routes::app(state)).await?;
    Ok(())
}
