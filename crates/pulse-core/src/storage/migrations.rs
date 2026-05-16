use sqlx::SqlitePool;
use crate::error::StorageError;

/// Apply all pending migrations to the database.
/// Uses a simple manual migration table.
pub async fn run_migrations(pool: &SqlitePool) -> Result<(), StorageError> {
    // Create schema_migrations table if it doesn't exist
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            applied_at INTEGER NOT NULL
        )"
    )
    .execute(pool)
    .await
    .map_err(StorageError::Sqlite)?;

    // Check which migrations have been applied
    let applied: Vec<i64> = sqlx::query_scalar("SELECT version FROM schema_migrations ORDER BY version")
        .fetch_all(pool)
        .await
        .map_err(StorageError::Sqlite)?;

    // Apply M0001 if not applied
    if !applied.contains(&1) {
        apply_m0001(pool).await?;
        sqlx::query("INSERT INTO schema_migrations (version, applied_at) VALUES (1, unixepoch())")
            .execute(pool)
            .await
            .map_err(StorageError::Sqlite)?;
        tracing::info!("Applied migration M0001_initial");
    }

    Ok(())
}

async fn apply_m0001(pool: &SqlitePool) -> Result<(), StorageError> {
    let sql = include_str!("../../migrations/M0001_initial.sql");

    // Execute the migration SQL statement by statement
    // We need to split on semicolons carefully (avoiding those in strings/comments)
    let mut conn = pool.acquire().await.map_err(StorageError::Sqlite)?;

    // Use sqlx's raw execution for multi-statement SQL
    sqlx::raw_sql(sql)
        .execute(&mut *conn)
        .await
        .map_err(|e| StorageError::Migration(format!("M0001 failed: {e}")))?;

    Ok(())
}
