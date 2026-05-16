use sqlx::{sqlite::{SqliteConnectOptions, SqlitePoolOptions}, SqlitePool, SqliteConnection};
use crate::error::StorageError;
use crate::config::PulseConfig;
use std::path::Path;
use std::str::FromStr;

/// Open a SQLite connection pool for writes (single connection to serialize writes)
pub async fn open_writer_pool(path: &Path, _config: &PulseConfig) -> Result<SqlitePool, StorageError> {
    let path_str = path.to_string_lossy();
    let opts = SqliteConnectOptions::from_str(&format!("sqlite://{}?mode=rwc", path_str))
        .map_err(StorageError::Sqlite)?
        .create_if_missing(true);

    SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(opts)
        .await
        .map_err(StorageError::Sqlite)
}

/// Open a SQLite connection pool for reads (multiple concurrent readers via WAL)
pub async fn open_reader_pool(path: &Path, _config: &PulseConfig) -> Result<SqlitePool, StorageError> {
    let path_str = path.to_string_lossy();
    let opts = SqliteConnectOptions::from_str(&format!("sqlite://{}?mode=ro", path_str))
        .map_err(StorageError::Sqlite)?;

    SqlitePoolOptions::new()
        .max_connections(4)
        .connect_with(opts)
        .await
        .map_err(StorageError::Sqlite)
}

/// Apply required PRAGMAs to a connection
pub async fn apply_pragmas(
    conn: &mut SqliteConnection,
    config: &PulseConfig,
) -> Result<(), StorageError> {
    use sqlx::Executor;

    conn.execute("PRAGMA journal_mode = WAL;").await
        .map_err(StorageError::Sqlite)?;

    if config.is_android {
        conn.execute("PRAGMA synchronous = FULL;").await
            .map_err(StorageError::Sqlite)?;
    } else {
        conn.execute("PRAGMA synchronous = NORMAL;").await
            .map_err(StorageError::Sqlite)?;
    }

    conn.execute("PRAGMA foreign_keys = ON;").await
        .map_err(StorageError::Sqlite)?;
    conn.execute("PRAGMA busy_timeout = 5000;").await
        .map_err(StorageError::Sqlite)?;
    conn.execute("PRAGMA cache_size = -32768;").await
        .map_err(StorageError::Sqlite)?;
    conn.execute("PRAGMA temp_store = MEMORY;").await
        .map_err(StorageError::Sqlite)?;

    if config.is_android {
        conn.execute("PRAGMA mmap_size = 0;").await
            .map_err(StorageError::Sqlite)?;
    } else {
        conn.execute("PRAGMA mmap_size = 268435456;").await
            .map_err(StorageError::Sqlite)?;
    }

    Ok(())
}
