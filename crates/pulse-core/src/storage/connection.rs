use crate::config::PulseConfig;
use crate::error::StorageError;
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
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
        .pragma("cache_size", "-8192")
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
        .pragma("cache_size", "-16384")
        .pragma("temp_store", "memory")
        .pragma("mmap_size", mmap);

    SqlitePoolOptions::new()
        .max_connections(3)
        .connect_with(opts)
        .await
        .map_err(StorageError::Sqlite)
}
