use crate::config::PulseConfig;
use crate::error::StorageError;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::{SqliteConnection, SqlitePool};
use std::path::Path;
use std::time::Duration;

/// Open a SQLite connection pool for writes (single connection to serialize writes).
/// Applies WAL mode and all performance pragmas at connect time.
pub async fn open_writer_pool(
    path: &Path,
    config: &PulseConfig,
) -> Result<SqlitePool, StorageError> {
    let sync = if config.is_android {
        SqliteSynchronous::Full
    } else {
        SqliteSynchronous::Normal
    };
    let mmap = if config.is_android { "0" } else { "268435456" };

    let opts = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .read_only(false)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(sync)
        .busy_timeout(Duration::from_secs(5))
        .foreign_keys(true)
        .pragma("cache_size", "-32768")
        .pragma("temp_store", "memory")
        .pragma("mmap_size", mmap);

    SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(opts)
        .await
        .map_err(StorageError::Sqlite)
}

/// Open a SQLite connection pool for reads (up to 4 concurrent readers via WAL).
/// WAL mode is NOT set here — the writer pool sets it; readers inherit it.
pub async fn open_reader_pool(
    path: &Path,
    config: &PulseConfig,
) -> Result<SqlitePool, StorageError> {
    let mmap = if config.is_android { "0" } else { "268435456" };

    let opts = SqliteConnectOptions::new()
        .filename(path)
        .read_only(true)
        .busy_timeout(Duration::from_secs(5))
        .foreign_keys(true)
        .pragma("cache_size", "-32768")
        .pragma("temp_store", "memory")
        .pragma("mmap_size", mmap);

    SqlitePoolOptions::new()
        .max_connections(4)
        .connect_with(opts)
        .await
        .map_err(StorageError::Sqlite)
}

/// Apply required PRAGMAs to a raw connection (for callers that acquire connections directly).
pub async fn apply_pragmas(
    conn: &mut SqliteConnection,
    config: &PulseConfig,
) -> Result<(), StorageError> {
    use sqlx::Executor;
    conn.execute("PRAGMA journal_mode = WAL;")
        .await
        .map_err(StorageError::Sqlite)?;
    let sync = if config.is_android {
        "PRAGMA synchronous = FULL;"
    } else {
        "PRAGMA synchronous = NORMAL;"
    };
    conn.execute(sync).await.map_err(StorageError::Sqlite)?;
    conn.execute("PRAGMA foreign_keys = ON;")
        .await
        .map_err(StorageError::Sqlite)?;
    conn.execute("PRAGMA busy_timeout = 5000;")
        .await
        .map_err(StorageError::Sqlite)?;
    conn.execute("PRAGMA cache_size = -32768;")
        .await
        .map_err(StorageError::Sqlite)?;
    conn.execute("PRAGMA temp_store = MEMORY;")
        .await
        .map_err(StorageError::Sqlite)?;
    let mmap = if config.is_android {
        "PRAGMA mmap_size = 0;"
    } else {
        "PRAGMA mmap_size = 268435456;"
    };
    conn.execute(mmap).await.map_err(StorageError::Sqlite)?;
    Ok(())
}
