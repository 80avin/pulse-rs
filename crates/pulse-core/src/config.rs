use std::path::PathBuf;

/// Application configuration
#[derive(Debug, Clone)]
pub struct PulseConfig {
    /// Path to the SQLite database file
    pub db_path: PathBuf,
    /// Path to the data directory (exports, etc.)
    pub data_dir: PathBuf,
    /// Maximum number of sync tasks that can run concurrently
    pub max_concurrent_syncs: usize,
    /// Maximum failure streak before a feed is disabled
    pub max_failure_streak: u32,
    /// Whether we're running on Android (affects some pragmas and sync behavior)
    pub is_android: bool,
    /// Reddit OAuth2 script-app client ID (from reddit.com/prefs/apps)
    pub reddit_client_id: Option<String>,
    /// Reddit OAuth2 script-app client secret
    pub reddit_client_secret: Option<String>,
}

impl PulseConfig {
    pub fn default_config() -> Self {
        let data_dir = platform_data_dir();
        let db_path = data_dir.join("pulse.db");

        Self {
            db_path,
            data_dir,
            max_concurrent_syncs: 10,
            max_failure_streak: 10,
            is_android: cfg!(target_os = "android"),
            reddit_client_id: None,
            reddit_client_secret: None,
        }
    }

    pub fn with_db_path(mut self, db_path: PathBuf) -> Self {
        self.db_path = db_path;
        self
    }

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
        let appdata = std::env::var("APPDATA").map_or_else(|_| PathBuf::from("."), PathBuf::from);
        appdata.join("pulse")
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
