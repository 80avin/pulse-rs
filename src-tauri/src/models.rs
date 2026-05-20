use serde::{Deserialize, Serialize};

/// Source/feed DTO sent to the frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceDto {
    pub id: String,
    pub name: String,
    pub url: String,
    pub kind: String,
    pub group: String,
    pub unread: i64,
    pub item_count: i64,
    pub avg_latency_ms: Option<f64>,
    pub last_sync: Option<String>,
    pub enabled: bool,
    pub failure_streak: i64,
}

/// Feed item DTO sent to the frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedItemDto {
    pub id: String,
    pub source_id: String,
    pub source_name: String,
    pub title: String,
    pub url: String,
    pub body: String,
    pub body_html: Option<String>,
    pub external_url: Option<String>,
    pub author: Option<String>,
    pub published_at: String,
    pub read: bool,
    pub saved: bool,
    pub hidden: bool,
    pub score: Option<i64>,
    pub n: i64,
    pub tags: Vec<String>,
    pub og_image: Option<String>,
    pub signal: f64,
}

/// Group DTO sent to the frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupDto {
    pub id: String,
    pub name: String,
    pub n: i64,
}

/// AI status DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiStatusDto {
    pub model_loaded: bool,
    pub vision_loaded: bool,
    pub fasttext_loaded: bool,
    pub miniml_loaded: bool,
    pub model_name: Option<String>,
    pub vision_model_name: Option<String>,
    pub fasttext_model_name: Option<String>,
    pub miniml_model_name: Option<String>,
    pub tagging_mode: String,
}

/// Known downloadable model info
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfoDto {
    pub id: String,
    pub name: String,
    pub description: String,
    pub size_mb: u32,
    pub downloaded: bool,
    pub active: bool,
    pub kind: String,
}

/// Download progress event payload
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgressEvent {
    pub model_id: String,
    pub file: String,
    pub bytes_done: u64,
    pub bytes_total: u64,
    pub done: bool,
}

/// Tagging progress event payload — emitted per-item during retag_all.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaggingProgressEvent {
    pub tagged: usize,
    pub total: usize,
    pub done: bool,
}

/// App settings DTO (round-tripped via tauri_settings.json)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettingsDto {
    pub density: String,
    pub mark_read_on: String,
    pub sync_interval_min: i64,
    pub wifi_only: bool,
    pub background_sync: bool,
    pub ai_tagging: bool,
    pub confidence_threshold: f64,
    pub notify_high_signal: bool,
    pub notify_saved: bool,
}

impl Default for AppSettingsDto {
    fn default() -> Self {
        Self {
            density: "normal".into(),
            mark_read_on: "open".into(),
            sync_interval_min: 15,
            wifi_only: false,
            background_sync: true,
            ai_tagging: true,
            confidence_threshold: 0.5,
            notify_high_signal: false,
            notify_saved: false,
        }
    }
}

/// DB stats DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbStatsDto {
    pub total_items: i64,
    pub unread_items: i64,
    pub saved_items: i64,
    pub total_sources: i64,
    pub db_size_kb: i64,
    pub tag_count: i64,
}

/// Share intent event payload — emitted from JNI bridge to frontend
#[cfg(target_os = "android")]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IncomingShareEvent {
    pub url: String,
}

/// Individual feed link candidate discovered from HTML scraping
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedLinkDto {
    pub url: String,
    pub title: Option<String>,
}

/// Result of detect_feed — either a direct feed or HTML-scraped candidates
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedCandidateDto {
    pub feed_url: String,
    pub kind: String,
    pub name: String,
    pub is_direct_feed: bool,
    pub is_hn: bool,
    pub no_feed_found: bool,
    pub candidates: Vec<FeedLinkDto>,
}

/// Sync result DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncResultDto {
    pub new_count: i64,
    pub error: Option<String>,
}

/// Pagination cursor for timeline pages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CursorDto {
    pub published_at: i64,
    pub item_id: String,
}

/// Paginated timeline response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemPageDto {
    pub items: Vec<FeedItemDto>,
    pub next_cursor: Option<CursorDto>,
}
