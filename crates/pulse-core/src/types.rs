use serde::{Deserialize, Serialize};

/// Opaque feed identifier (UUIDv4 string)
pub type FeedId = String;
/// Opaque feed group identifier (UUIDv4 string)
pub type GroupId = String;
/// Opaque feed item identifier (UUIDv5 string)
pub type ItemId = String;

/// Source type for a feed
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FeedType {
    Rss,
    Hn,
    Reddit,
}

impl FeedType {
    pub fn as_str(&self) -> &'static str {
        match self {
            FeedType::Rss => "rss",
            FeedType::Hn => "hn",
            FeedType::Reddit => "reddit",
        }
    }
}

impl std::fmt::Display for FeedType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for FeedType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "rss" => Ok(FeedType::Rss),
            "hn" => Ok(FeedType::Hn),
            "reddit" => Ok(FeedType::Reddit),
            other => Err(format!("unknown feed type: {other}")),
        }
    }
}

/// A feed source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Feed {
    pub id: FeedId,
    pub url: String,
    pub feed_type: FeedType,
    pub title: Option<String>,
    pub description: Option<String>,
    pub site_url: Option<String>,
    pub icon_url: Option<String>,
    pub group_id: Option<GroupId>,
    pub poll_interval_secs: i64,
    pub is_enabled: bool,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub last_fetched_at: Option<i64>,
    pub last_success_at: Option<i64>,
    pub last_item_at: Option<i64>,
    pub failure_streak: i64,
    pub total_fetches: i64,
    pub total_failures: i64,
    pub avg_latency_ms: Option<f64>,
    pub next_fetch_at: Option<i64>,
    pub source_config: serde_json::Value,
    pub language: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// A group of feeds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedGroup {
    pub id: GroupId,
    pub name: String,
    pub description: Option<String>,
    pub color: Option<String>,
    pub sort_order: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

/// A normalized feed item ready for DB storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedItem {
    pub id: ItemId,
    pub feed_id: FeedId,
    pub source_guid: String,
    pub title: String,
    pub url: Option<String>,
    pub author: Option<String>,
    /// Unix timestamp (seconds). Always present — falls back to fetched_at.
    pub published_at: i64,
    pub fetched_at: i64,
    pub body_text: Option<String>,
    pub body_html: Option<String>,
    pub word_count: Option<i64>,
    pub score: Option<i64>,
    pub comment_count: Option<i64>,
    pub comment_url: Option<String>,
    pub source_meta: serde_json::Value,
}

/// User-controlled state for a feed item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemState {
    pub item_id: ItemId,
    pub is_read: bool,
    pub is_saved: bool,
    pub is_hidden: bool,
    pub read_at: Option<i64>,
    pub saved_at: Option<i64>,
    pub hidden_at: Option<i64>,
    pub updated_at: i64,
}

/// Partial update to item state
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ItemStatePatch {
    pub is_read: Option<bool>,
    pub is_saved: Option<bool>,
    pub is_hidden: Option<bool>,
}

/// An AI-generated tag for a feed item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiTag {
    pub id: String,
    pub item_id: ItemId,
    pub tag: String,
    pub confidence: f32,
    pub tagger_source: TaggerSource,
    pub rule_id: Option<String>,
    pub model_name: Option<String>,
    pub model_version: Option<String>,
    pub explanation: String,
    pub created_at: i64,
}

/// Where a tag came from
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaggerSource {
    Rule,
    Model,
}

impl TaggerSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaggerSource::Rule => "rule",
            TaggerSource::Model => "model",
        }
    }
}

impl std::str::FromStr for TaggerSource {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "rule" => Ok(TaggerSource::Rule),
            "model" => Ok(TaggerSource::Model),
            other => Err(format!("unknown tagger source: {other}")),
        }
    }
}

/// Result from the tagging engine for a single tag
#[derive(Debug, Clone)]
pub struct TagResult {
    pub tag: String,
    pub confidence: f32,
    pub explanation: String,
    pub source: TaggerSource,
    pub rule_id: Option<String>,
}

/// A flattened view of a feed item for display (joined with feed + state + tags)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedItemView {
    // Item fields
    pub id: ItemId,
    pub title: String,
    pub url: Option<String>,
    pub author: Option<String>,
    pub published_at: i64,
    pub fetched_at: i64,
    pub word_count: Option<i64>,
    pub score: Option<i64>,
    pub comment_count: Option<i64>,
    pub comment_url: Option<String>,

    // Feed fields
    pub feed_id: FeedId,
    pub feed_title: Option<String>,
    pub feed_type: FeedType,
    pub feed_url: String,

    // Group fields
    pub group_id: Option<GroupId>,
    pub group_name: Option<String>,

    // State fields
    pub is_read: bool,
    pub is_saved: bool,
    pub is_hidden: bool,

    // AI tags (JSON array of tag names)
    pub ai_tags: Vec<String>,
}

/// Cursor for timeline pagination (published_at, id) tuple
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineCursor {
    pub published_at: i64,
    pub id: ItemId,
}

/// Filters for timeline queries
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TimelineFilter {
    pub group_id: Option<GroupId>,
    pub feed_id: Option<FeedId>,
    pub is_read: Option<bool>,
    pub is_saved: Option<bool>,
    pub tag: Option<String>,
}

/// A page of timeline results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelinePage {
    pub items: Vec<FeedItemView>,
    pub next_cursor: Option<TimelineCursor>,
    pub has_more: bool,
}

/// Result of a sync operation for a feed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    pub feed_id: FeedId,
    pub new_items: usize,
    pub skipped_items: usize,
    pub fetch_latency_ms: u64,
    pub was_cached: bool,
}

/// DB statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbStats {
    pub feed_count: i64,
    pub item_count: i64,
    pub unread_count: i64,
    pub saved_count: i64,
    pub tag_count: i64,
    pub db_size_bytes: i64,
}

/// Outcome of enriching a single item
#[derive(Debug, Clone)]
pub enum EnrichStatus {
    Ok,
    Image,
    Skipped,
    Error(String),
}

#[derive(Debug, Clone)]
pub struct EnrichItemResult {
    pub item_id: String,
    pub url: String,
    pub status: EnrichStatus,
    pub og_title: Option<String>,
    pub og_description: Option<String>,
    pub og_image: Option<String>,
}

/// Aggregate stats for an enrichment run
#[derive(Debug, Default, Clone)]
pub struct EnrichStats {
    pub enriched: usize,
    pub image_posts: usize,
    pub skipped: usize,
    pub errors: usize,
}
