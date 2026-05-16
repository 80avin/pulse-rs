use thiserror::Error;

pub type Result<T> = std::result::Result<T, PulseError>;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] sqlx::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Database actor disconnected")]
    ActorDisconnected,

    #[error("Item not found: {id}")]
    NotFound { id: String },

    #[error("Database I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Migration error: {0}")]
    Migration(String),
}

#[derive(Debug, Error)]
pub enum FeedError {
    #[error("HTTP {status} fetching {url}: {message}")]
    Http {
        url: String,
        status: u16,
        message: String,
    },

    #[error("Feed parse error for {url}: {source}")]
    Parse {
        url: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("Feed not found: {id}")]
    NotFound { id: String },

    #[error("Network error fetching {url}: {source}")]
    Network {
        url: String,
        #[source]
        source: reqwest::Error,
    },

    #[error("JSON deserialize error for {url}: {source}")]
    Json {
        url: String,
        #[source]
        source: serde_json::Error,
    },
}

#[derive(Debug, Error)]
pub enum SyncError {
    #[error("Feed error: {0}")]
    Feed(#[from] FeedError),

    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),

    #[error("Sync task for feed {feed_id} encountered a permanent error: {message}")]
    Permanent { feed_id: String, message: String },

    #[error("Sync was cancelled")]
    Cancelled,
}

#[derive(Debug, Error)]
pub enum TaggingError {
    #[error("AI model not loaded")]
    ModelNotLoaded,

    #[error("Storage error during tagging: {0}")]
    Storage(#[from] StorageError),

    #[error("Rule evaluation error: {0}")]
    Rule(String),

    #[error("ONNX inference error: {0}")]
    Onnx(String),

    #[error("Tokenizer error: {0}")]
    Tokenizer(String),

    #[error("Image decode error: {0}")]
    ImageDecode(String),

    #[error("Image network error: {0}")]
    ImageNetwork(String),
}

#[derive(Debug, Error)]
pub enum PulseError {
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),

    #[error("Feed error: {0}")]
    Feed(#[from] FeedError),

    #[error("Sync error: {0}")]
    Sync(#[from] SyncError),

    #[error("Tagging error: {0}")]
    Tagging(#[from] TaggingError),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Not found: {0}")]
    NotFound(String),
}
