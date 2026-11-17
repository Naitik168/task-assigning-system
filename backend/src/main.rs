use std::time::Duration;

use anyhow::Context;
use sqlx::postgres::PgPoolOptions;
use task_manager_api::{build_router, config::Config, services::cache::build_cache, AppState};
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("task_manager_api=info,tower_http=info")),
        )
        .init();

    let config = Config::from_env()?;
    let pool = connect_with_retry(&config.database_url).await?;

    if config.run_migrations {
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .context("failed to run database migrations")?;
        tracing::info!("database migrations applied");
    }

    let cache = build_cache(&config).await;
    let bind_addr = config.bind_addr.clone();
    let state = AppState::new(pool, config, cache);
    let app = build_router(state);

    let listener = TcpListener::bind(&bind_addr)
        .await
        .with_context(|| format!("failed to bind {bind_addr}"))?;
    tracing::info!("listening on http://{bind_addr} (Swagger UI at /docs)");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

/// Retries the initial connection so the API tolerates Postgres starting
/// slightly later (e.g. under Docker Compose).
async fn connect_with_retry(url: &str) -> anyhow::Result<sqlx::PgPool> {
    let mut attempt = 0;
    loop {
        attempt += 1;
        match PgPoolOptions::new()
            .max_connections(10)
            .acquire_timeout(Duration::from_secs(5))
            .connect(url)
            .await
        {
            Ok(pool) => return Ok(pool),
            Err(e) if attempt < 15 => {
                tracing::warn!(attempt, error = %e, "database not ready, retrying in 2s");
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
            Err(e) => return Err(e).context("could not connect to the database"),
        }
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c().await.ok();
    };
    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut sig) =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            sig.recv().await;
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    tracing::info!("shutdown signal received");
}
