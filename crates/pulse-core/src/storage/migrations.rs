use crate::error::StorageError;
use sqlx::{Row, SqlitePool};

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

    tracing::info!(applied = ?applied, "Starting database migrations");

    if !applied.contains(&1) {
        apply(
            pool,
            1,
            "M0001_initial",
            include_str!("../../migrations/M0001_initial.sql"),
        )
        .await?;
    }

    if !applied.contains(&2) {
        apply(
            pool,
            2,
            "M0002_fts_update_trigger",
            include_str!("../../migrations/M0002_fts_update_trigger.sql"),
        )
        .await?;
    }

    if !applied.contains(&3) {
        // M0003 is a bare ALTER TABLE ... ADD COLUMN. If a legacy DB already has
        // the column (crash between the old ALTER commit and the version insert),
        // running it again would brick startup with "duplicate column name".
        if !column_exists(pool, "item_states", "note").await? {
            apply(pool, 3, "M0003_add_note", include_str!("../../migrations/M0003_add_note.sql"))
                .await?;
        } else {
            record_version(pool, 3).await?;
        }
    }

    if !applied.contains(&4) {
        if !column_exists(pool, "feeds", "hue").await? {
            apply(pool, 4, "M0004_add_feed_hue", include_str!("../../migrations/M0004_add_feed_hue.sql"))
                .await?;
        } else {
            record_version(pool, 4).await?;
        }
    }

    if !applied.contains(&5) {
        apply(
            pool,
            5,
            "M0005_fts_content_fix",
            include_str!("../../migrations/M0005_fts_content_fix.sql"),
        )
        .await?;
    }

    if !applied.contains(&6) {
        // Data migration: rewrite stored body_html so relative src/href/srcset
        // become absolute (and http media -> https). Runs once; existing items
        // that predate the ingestion-time resolver get fixed here.
        let fixed = crate::storage::actor::fix_relative_urls(pool).await?;
        record_version(pool, 6).await?;
        tracing::info!(fixed, "M0006_fix_relative_urls rewrote relative URLs");
    }

    tracing::info!("Database migrations complete");
    Ok(())
}

/// Run a migration and record its version in a single transaction, so a crash
/// between the SQL and the version insert cannot leave the DB half-migrated.
async fn apply(
    pool: &SqlitePool,
    version: i64,
    name: &str,
    sql: &str,
) -> Result<(), StorageError> {
    tracing::info!(migration = name, "Applying migration");
    let mut tx = pool.begin().await.map_err(StorageError::Sqlite)?;
    sqlx::raw_sql(sql).execute(&mut *tx).await.map_err(|e| {
        tracing::error!(migration = name, error = %e, "Migration failed");
        StorageError::Migration(format!("{name} failed: {e}"))
    })?;
    sqlx::query("INSERT INTO schema_migrations (version, applied_at) VALUES (?, unixepoch())")
        .bind(version)
        .execute(&mut *tx)
        .await
        .map_err(StorageError::Sqlite)?;
    tx.commit().await.map_err(StorageError::Sqlite)?;
    tracing::info!(migration = name, "Applied migration");
    Ok(())
}

async fn record_version(pool: &SqlitePool, version: i64) -> Result<(), StorageError> {
    sqlx::query(
        "INSERT OR IGNORE INTO schema_migrations (version, applied_at) VALUES (?, unixepoch())",
    )
    .bind(version)
    .execute(pool)
    .await
    .map_err(StorageError::Sqlite)?;
    Ok(())
}

async fn column_exists(pool: &SqlitePool, table: &str, column: &str) -> Result<bool, StorageError> {
    // table/column are compile-time constants in this crate; not user input.
    let rows = sqlx::query(&format!("PRAGMA table_info({table})"))
        .fetch_all(pool)
        .await
        .map_err(StorageError::Sqlite)?;
    Ok(rows
        .iter()
        .any(|r| r.get::<String, _>("name").eq_ignore_ascii_case(column)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::PulseConfig;
    use crate::storage::connection::open_writer_pool;
    use uuid::Uuid;

    async fn test_pool() -> SqlitePool {
        let dir = std::env::temp_dir().join(format!("pulse-core-mig-test-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let config = PulseConfig::default().with_data_dir(dir);
        open_writer_pool(&config.db_path, &config).await.unwrap()
    }

    #[tokio::test]
    async fn migrations_are_idempotent() {
        let pool = test_pool().await;
        run_migrations(&pool).await.unwrap();
        run_migrations(&pool).await.unwrap();
    }

    #[tokio::test]
    async fn missing_version_row_with_existing_column_does_not_brick() {
        let pool = test_pool().await;
        run_migrations(&pool).await.unwrap();
        // Simulate a legacy partial state: the note column exists but the
        // M0003 version row is missing. Startup must not fail.
        sqlx::query("DELETE FROM schema_migrations WHERE version = 3")
            .execute(&pool)
            .await
            .unwrap();
        run_migrations(&pool).await.unwrap();
        let note: Option<String> =
            sqlx::query_scalar("SELECT name FROM pragma_table_info('item_states') WHERE name = 'note'")
                .fetch_optional(&pool)
                .await
                .unwrap();
        assert_eq!(note.as_deref(), Some("note"));
    }
}
