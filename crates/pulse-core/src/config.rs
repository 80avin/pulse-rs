use std::path::PathBuf;

/// Which text classification backend is active
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum TextBackend {
    /// DeBERTa NLI cross-encoder (legacy)
    Nli,
    /// FastText supervised classifier (2-10MB, <1ms/item)
    FastText,
    /// MiniLM ONNX embedding + MLP classifier head (24MB)
    MiniMl,
    /// FastText primary; MiniLM for semantic categories (default)
    #[default]
    HybridFastTextMiniMl,
}

/// Application configuration
#[derive(Debug, Clone)]
pub struct PulseConfig {
    /// Path to the SQLite database file
    pub db_path: PathBuf,
    /// Path to the data directory (models, exports, etc.)
    pub data_dir: PathBuf,
    /// Maximum number of sync tasks that can run concurrently
    pub max_concurrent_syncs: usize,
    /// Maximum failure streak before a feed is disabled
    pub max_failure_streak: u32,
    /// Maximum backoff interval in seconds (4 hours)
    pub max_backoff_secs: u64,
    /// Whether we're running on Android (affects some pragmas and sync behavior)
    pub is_android: bool,
    /// Reddit OAuth2 script-app client ID (from reddit.com/prefs/apps)
    pub reddit_client_id: Option<String>,
    /// Reddit OAuth2 script-app client secret
    pub reddit_client_secret: Option<String>,
    /// Which text classification backend to use for tagging
    pub text_backend: TextBackend,
    /// Whether to supplement model tags with rule engine (always false in new stack)
    pub use_rules: bool,
}

impl PulseConfig {
    /// Create a config with default settings using the platform data directory
    pub fn default_config() -> Self {
        let data_dir = platform_data_dir();
        let db_path = data_dir.join("pulse.db");

        Self {
            db_path,
            data_dir,
            max_concurrent_syncs: 10,
            max_failure_streak: 10,
            max_backoff_secs: 14400, // 4 hours
            is_android: cfg!(target_os = "android"),
            reddit_client_id: None,
            reddit_client_secret: None,
            text_backend: TextBackend::HybridFastTextMiniMl,
            use_rules: false,
        }
    }

    /// Create a config pointing at a specific database path
    pub fn with_db_path(mut self, db_path: PathBuf) -> Self {
        self.db_path = db_path;
        self
    }

    /// Create a config pointing at a specific data dir
    pub fn with_data_dir(mut self, data_dir: PathBuf) -> Self {
        self.data_dir = data_dir.clone();
        self.db_path = data_dir.join("pulse.db");
        self
    }

    /// Set Reddit OAuth2 credentials (client-credentials / script-app flow).
    pub fn with_reddit_auth(mut self, client_id: String, client_secret: String) -> Self {
        self.reddit_client_id = Some(client_id);
        self.reddit_client_secret = Some(client_secret);
        self
    }

    /// Path to the models directory
    pub fn models_dir(&self) -> PathBuf {
        self.data_dir.join("models")
    }

    /// Path to the training data directory (labels, exports)
    pub fn training_dir(&self) -> PathBuf {
        self.data_dir.join("training")
    }
}

impl Default for PulseConfig {
    fn default() -> Self {
        Self::default_config()
    }
}

/// Returns the platform-appropriate data directory for Pulse.
///
/// - Linux/macOS: `$XDG_DATA_HOME/pulse` (fallback: `~/.local/share/pulse`)
/// - Android: `/data/data/com.avinthakur080.pulse_rs/files`
/// - Windows: `%APPDATA%\pulse`
pub fn platform_data_dir() -> PathBuf {
    #[cfg(target_os = "android")]
    {
        PathBuf::from("/data/data/com.avinthakur080.pulse_rs/files")
    }

    #[cfg(target_os = "windows")]
    {
        let appdata = std::env::var("APPDATA").unwrap_or_else(|_| {
            dirs_home()
                .unwrap_or_else(|| PathBuf::from("."))
                .to_string_lossy()
                .into_owned()
        });
        PathBuf::from(appdata).join("pulse")
    }

    #[cfg(not(any(target_os = "android", target_os = "windows")))]
    {
        // Linux / macOS: XDG_DATA_HOME or ~/.local/share
        if let Ok(xdg) = std::env::var("XDG_DATA_HOME")
            && !xdg.is_empty()
        {
            return PathBuf::from(xdg).join("pulse");
        }

        let home = std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."));

        home.join(".local").join("share").join("pulse")
    }
}
