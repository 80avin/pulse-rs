use crate::error::StorageError;
use sqlx::SqlitePool;

/// Apply all pending migrations to the database.
/// Uses a simple manual migration table.
pub async fn run_migrations(pool: &SqlitePool) -> Result<(), StorageError> {
    // Create schema_migrations table if it doesn't exist
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            applied_at INTEGER NOT NULL
        )",
    )
    .execute(pool)
    .await
    .map_err(StorageError::Sqlite)?;

    // Check which migrations have been applied
    let applied: Vec<i64> =
        sqlx::query_scalar("SELECT version FROM schema_migrations ORDER BY version")
            .fetch_all(pool)
            .await
            .map_err(StorageError::Sqlite)?;

    tracing::info!(
        applied = ?applied,
        "Starting database migrations"
    );

    if !applied.contains(&1) {
        tracing::info!("Applying migration M0001_initial");
        apply_sql(
            pool,
            include_str!("../../migrations/M0001_initial.sql"),
            "M0001",
        )
        .await?;
        sqlx::query("INSERT INTO schema_migrations (version, applied_at) VALUES (1, unixepoch())")
            .execute(pool)
            .await
            .map_err(StorageError::Sqlite)?;
        tracing::info!("Applied migration M0001_initial");
    }

    if !applied.contains(&2) {
        tracing::info!("Applying migration M0002_fts_update_trigger");
        apply_sql(
            pool,
            include_str!("../../migrations/M0002_fts_update_trigger.sql"),
            "M0002",
        )
        .await?;
        sqlx::query("INSERT INTO schema_migrations (version, applied_at) VALUES (2, unixepoch())")
            .execute(pool)
            .await
            .map_err(StorageError::Sqlite)?;
        tracing::info!("Applied migration M0002_fts_update_trigger");
    }

    if !applied.contains(&3) {
        tracing::info!("Applying migration M0003_add_note");
        apply_sql(
            pool,
            include_str!("../../migrations/M0003_add_note.sql"),
            "M0003",
        )
        .await?;
        sqlx::query("INSERT INTO schema_migrations (version, applied_at) VALUES (3, unixepoch())")
            .execute(pool)
            .await
            .map_err(StorageError::Sqlite)?;
        tracing::info!("Applied migration M0003_add_note");
    }

    tracing::info!("Database migrations complete");
    Ok(())
}

async fn apply_sql(pool: &SqlitePool, sql: &str, name: &str) -> Result<(), StorageError> {
    tracing::debug!(migration = name, "Executing migration SQL");
    let mut conn = pool.acquire().await.map_err(StorageError::Sqlite)?;
    sqlx::raw_sql(sql).execute(&mut *conn).await.map_err(|e| {
        tracing::error!(migration = name, error = %e, "Migration failed");
        StorageError::Migration(format!("{name} failed: {e}"))
    })?;
    tracing::debug!(migration = name, "Migration SQL executed successfully");
    Ok(())
}
