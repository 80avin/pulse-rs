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

    if !applied.contains(&1) {
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

    Ok(())
}

async fn apply_sql(pool: &SqlitePool, sql: &str, name: &str) -> Result<(), StorageError> {
    let mut conn = pool.acquire().await.map_err(StorageError::Sqlite)?;
    sqlx::raw_sql(sql)
        .execute(&mut *conn)
        .await
        .map_err(|e| StorageError::Migration(format!("{name} failed: {e}")))?;
    Ok(())
}
