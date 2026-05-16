use reqwest::Client;
use serde::Deserialize;
use uuid::Uuid;
use futures::stream::{self, StreamExt};
use crate::error::FeedError;
use crate::types::{Feed, FeedItem};
use crate::feeds::normalize::{strip_html, count_words};

const USER_AGENT: &str = "Pulse/0.1 (+https://github.com/avinthakur080/pulse-rs; feed-reader)";
const HN_API_BASE: &str = "https://hacker-news.firebaseio.com/v0";
const CONCURRENT_ITEM_FETCHES: usize = 10;
const FIRST_SYNC_LIMIT: usize = 30;

#[derive(Debug, Deserialize)]
struct HnItem {
    id: u64,
    #[serde(rename = "type")]
    item_type: Option<String>,
    title: Option<String>,
    url: Option<String>,
    by: Option<String>,
    time: Option<i64>,
    text: Option<String>,
    score: Option<i64>,
    descendants: Option<i64>,
    kids: Option<Vec<u64>>,
    deleted: Option<bool>,
    dead: Option<bool>,
}

/// Result of an HN fetch
pub struct HnFetchResult {
    pub items: Vec<FeedItem>,
    pub last_seen_id: Option<u64>,
    pub was_cached: bool,
}

/// Fetch HN items (top stories, new stories, etc.)
///
/// - First sync: fetches top `initial_limit` items (default 30)
/// - Subsequent syncs: only fetches items with ID > `last_seen_id`
pub async fn fetch_hn(
    client: &Client,
    feed: &Feed,
) -> Result<HnFetchResult, FeedError> {
    let fetched_at = chrono::Utc::now().timestamp();

    // Parse source_config
    let section = feed.source_config
        .get("section")
        .and_then(|v| v.as_str())
        .unwrap_or("topstories");

    let initial_limit = feed.source_config
        .get("initial_limit")
        .and_then(|v| v.as_u64())
        .unwrap_or(FIRST_SYNC_LIMIT as u64) as usize;

    let last_seen_id: Option<u64> = feed.source_config
        .get("last_seen_id")
        .and_then(|v| v.as_u64());

    // Fetch the IDs list
    let list_url = format!("{}/{}.json", HN_API_BASE, section);
    let all_ids: Vec<u64> = client.get(&list_url)
        .header("User-Agent", USER_AGENT)
        .send()
        .await
        .map_err(|e| FeedError::Network { url: list_url.clone(), source: e })?
        .json()
        .await
        .map_err(|e| FeedError::Network { url: list_url.clone(), source: e })?;

    if all_ids.is_empty() {
        return Ok(HnFetchResult {
            items: Vec::new(),
            last_seen_id,
            was_cached: true,
        });
    }

    let max_id = all_ids.iter().copied().max();

    // Determine which IDs to fetch
    let ids_to_fetch: Vec<u64> = if let Some(last_id) = last_seen_id {
        // Incremental: only fetch items we haven't seen yet
        all_ids.into_iter().filter(|&id| id > last_id).collect()
    } else {
        // First sync: fetch only top N
        all_ids.into_iter().take(initial_limit).collect()
    };

    if ids_to_fetch.is_empty() {
        return Ok(HnFetchResult {
            items: Vec::new(),
            last_seen_id: max_id.or(last_seen_id),
            was_cached: true,
        });
    }

    // Compute namespace UUID for this feed
    let ns_uuid = Uuid::new_v5(&Uuid::NAMESPACE_URL, feed.url.as_bytes());

    // Fetch items concurrently (up to CONCURRENT_ITEM_FETCHES at a time)
    let items: Vec<FeedItem> = stream::iter(ids_to_fetch)
        .map(|id| {
            let client = client.clone();
            let feed_id = feed.id.clone();
            async move {
                fetch_hn_item(&client, id, &feed_id, ns_uuid, fetched_at).await
            }
        })
        .buffer_unordered(CONCURRENT_ITEM_FETCHES)
        .filter_map(|r| async move {
            match r {
                Ok(Some(item)) => Some(item),
                Ok(None) => None, // deleted/dead items
                Err(e) => {
                    tracing::warn!("Failed to fetch HN item: {}", e);
                    None
                }
            }
        })
        .collect()
        .await;

    Ok(HnFetchResult {
        items,
        last_seen_id: max_id.or(last_seen_id),
        was_cached: false,
    })
}

async fn fetch_hn_item(
    client: &Client,
    id: u64,
    feed_id: &str,
    ns_uuid: Uuid,
    fetched_at: i64,
) -> Result<Option<FeedItem>, FeedError> {
    let url = format!("{}/item/{}.json", HN_API_BASE, id);
    let hn_item: HnItem = client.get(&url)
        .header("User-Agent", USER_AGENT)
        .send()
        .await
        .map_err(|e| FeedError::Network { url: url.clone(), source: e })?
        .json()
        .await
        .map_err(|e| FeedError::Network { url: url.clone(), source: e })?;

    // Skip deleted or dead items
    if hn_item.deleted.unwrap_or(false) || hn_item.dead.unwrap_or(false) {
        return Ok(None);
    }

    // Skip items without a title (not a story)
    let title = match hn_item.title {
        Some(t) if !t.is_empty() => t,
        _ => return Ok(None),
    };

    let source_guid = hn_item.id.to_string();
    let item_id = Uuid::new_v5(&ns_uuid, source_guid.as_bytes()).to_string();

    let item_url = hn_item.url
        .filter(|u| !u.is_empty())
        .or_else(|| Some(format!("https://news.ycombinator.com/item?id={}", hn_item.id)));

    let comment_url = format!("https://news.ycombinator.com/item?id={}", hn_item.id);

    // HN `time` is always present for real items; safe to use as published_at
    let published_at = hn_item.time.unwrap_or(fetched_at);

    let body_html = hn_item.text.as_deref().map(|t| t.to_string());
    let body_text = body_html.as_deref().map(|h| strip_html(h));
    let word_count = body_text.as_deref().map(|t| count_words(t) as i64);

    let item_type = hn_item.item_type.unwrap_or_else(|| "story".to_string());
    let kids = hn_item.kids.unwrap_or_default();

    let source_meta = serde_json::json!({
        "type": item_type,
        "kids": kids,
    });

    Ok(Some(FeedItem {
        id: item_id,
        feed_id: feed_id.to_string(),
        source_guid,
        title,
        url: item_url,
        author: hn_item.by,
        published_at,
        fetched_at,
        body_text,
        body_html,
        word_count,
        score: hn_item.score,
        comment_count: hn_item.descendants,
        comment_url: Some(comment_url),
        source_meta,
    }))
}
