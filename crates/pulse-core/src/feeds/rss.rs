use crate::error::FeedError;
use crate::feeds::normalize::{collapse_whitespace, count_words, strip_html};
use crate::types::{Feed, FeedItem};
use reqwest::Client;
use uuid::Uuid;

const USER_AGENT: &str = "Pulse/0.1 (+https://github.com/avinthakur080/pulse-rs; feed-reader)";

/// Result of an RSS/Atom fetch
pub struct RssFetchResult {
    pub items: Vec<FeedItem>,
    pub feed_title: Option<String>,
    pub feed_description: Option<String>,
    pub feed_site_url: Option<String>,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub was_cached: bool,
}

/// Fetch and parse an RSS/Atom feed, returning normalized FeedItems.
/// Respects conditional HTTP headers (ETag / If-Modified-Since).
pub async fn fetch_rss(client: &Client, feed: &Feed) -> Result<RssFetchResult, FeedError> {
    let fetched_at = chrono::Utc::now().timestamp();
    let url = feed.url.clone();

    // Build request with conditional headers
    let mut req = client.get(&url).header("User-Agent", USER_AGENT);

    if let Some(ref etag) = feed.etag {
        req = req.header("If-None-Match", etag);
    }
    if let Some(ref lm) = feed.last_modified {
        req = req.header("If-Modified-Since", lm);
    }

    let _start = std::time::Instant::now();
    let response = req.send().await.map_err(|e| FeedError::Network {
        url: url.clone(),
        source: e,
    })?;

    let status = response.status();

    // 304 Not Modified — content unchanged
    if status.as_u16() == 304 {
        return Ok(RssFetchResult {
            items: Vec::new(),
            feed_title: None,
            feed_description: None,
            feed_site_url: None,
            etag: feed.etag.clone(),
            last_modified: feed.last_modified.clone(),
            was_cached: true,
        });
    }

    if !status.is_success() {
        return Err(FeedError::Http {
            url: url.clone(),
            status: status.as_u16(),
            message: status.canonical_reason().unwrap_or("Unknown").to_string(),
        });
    }

    // Extract caching headers before consuming response
    let new_etag = response
        .headers()
        .get(reqwest::header::ETAG)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let new_last_modified = response
        .headers()
        .get(reqwest::header::LAST_MODIFIED)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let bytes = response.bytes().await.map_err(|e| FeedError::Network {
        url: url.clone(),
        source: e,
    })?;

    // Parse with feed-rs
    let parsed = feed_rs::parser::parse(bytes.as_ref()).map_err(|e| FeedError::Parse {
        url: url.clone(),
        source: Box::new(e),
    })?;

    let feed_title = parsed
        .title
        .as_ref()
        .map(|t| collapse_whitespace(&t.content));
    let feed_description = parsed
        .description
        .as_ref()
        .map(|d| collapse_whitespace(&d.content));
    let feed_site_url = parsed.links.first().map(|l| l.href.clone());

    // Compute the namespace UUID for this feed (UUIDv5 of feed URL)
    let ns_uuid = Uuid::new_v5(&Uuid::NAMESPACE_URL, url.as_bytes());

    let items = parsed
        .entries
        .into_iter()
        .map(|entry| normalize_rss_entry(entry, &feed.id, &url, ns_uuid, fetched_at))
        .collect();

    Ok(RssFetchResult {
        items,
        feed_title,
        feed_description,
        feed_site_url,
        etag: new_etag.or(feed.etag.clone()),
        last_modified: new_last_modified.or(feed.last_modified.clone()),
        was_cached: false,
    })
}

fn normalize_rss_entry(
    entry: feed_rs::model::Entry,
    feed_id: &str,
    _feed_url: &str,
    ns_uuid: Uuid,
    fetched_at: i64,
) -> FeedItem {
    // source_guid: use entry.id, or hash the link URL
    let source_guid = if entry.id.is_empty() {
        entry
            .links
            .first()
            .map(|l| format!("sha256:{:x}", md5_hash(&l.href)))
            .unwrap_or_else(|| format!("sha256:{:x}", md5_hash(&entry.id)))
    } else {
        entry.id.clone()
    };

    let item_id = Uuid::new_v5(&ns_uuid, source_guid.as_bytes()).to_string();

    let title = entry
        .title
        .as_ref()
        .map(|t| collapse_whitespace(&strip_html(&t.content)))
        .unwrap_or_else(|| "(no title)".to_string());

    let url = entry.links.first().map(|l| l.href.clone());

    let author = entry.authors.first().map(|a| a.name.clone());

    // published_at: use published, then updated, then fetched_at
    let published_at = entry
        .published
        .or(entry.updated)
        .map(|dt| dt.timestamp())
        .unwrap_or(fetched_at);

    // body_html: prefer content, fallback to summary
    let body_html = entry
        .content
        .as_ref()
        .and_then(|c| c.body.as_ref())
        .map(|s| s.clone())
        .or_else(|| entry.summary.as_ref().map(|s| s.content.clone()));

    // body_text: strip HTML from body_html or summary
    let body_text = body_html.as_deref().map(|h| strip_html(h));

    let word_count = body_text.as_deref().map(|t| count_words(t) as i64);

    // Collect categories
    let categories: Vec<String> = entry.categories.iter().map(|c| c.term.clone()).collect();
    let source_meta = serde_json::json!({ "categories": categories });

    FeedItem {
        id: item_id,
        feed_id: feed_id.to_string(),
        source_guid,
        title,
        url,
        author,
        published_at,
        fetched_at,
        body_text,
        body_html,
        word_count,
        score: None,
        comment_count: None,
        comment_url: None,
        source_meta,
    }
}

/// Simple hash for fallback GUID generation
fn md5_hash(s: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}
