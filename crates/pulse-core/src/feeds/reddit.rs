use reqwest::Client;
use serde::Deserialize;
use uuid::Uuid;
use crate::error::FeedError;
use crate::types::{Feed, FeedItem};
use crate::feeds::normalize::{strip_html, count_words};

const USER_AGENT: &str = "Pulse/0.1 (+https://github.com/avinthakur080/pulse-rs; feed-reader)";
const REDDIT_BASE: &str = "https://reddit.com";

/// Result of a Reddit fetch
pub struct RedditFetchResult {
    pub items: Vec<FeedItem>,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub was_cached: bool,
}

/// Top-level Reddit listing response
#[derive(Debug, Deserialize)]
struct RedditListing {
    data: RedditListingData,
}

#[derive(Debug, Deserialize)]
struct RedditListingData {
    children: Vec<RedditChild>,
}

#[derive(Debug, Deserialize)]
struct RedditChild {
    data: RedditPost,
}

#[derive(Debug, Deserialize)]
struct RedditPost {
    id: String,
    title: String,
    url: Option<String>,
    permalink: String,
    author: Option<String>,
    created_utc: f64,
    selftext: Option<String>,
    selftext_html: Option<String>,
    score: Option<i64>,
    num_comments: Option<i64>,
    subreddit: Option<String>,
    link_flair_text: Option<String>,
    is_self: Option<bool>,
    thumbnail: Option<String>,
}

/// Fetch posts from a Reddit subreddit using the JSON API.
pub async fn fetch_reddit(
    client: &Client,
    feed: &Feed,
) -> Result<RedditFetchResult, FeedError> {
    let fetched_at = chrono::Utc::now().timestamp();

    let subreddit = feed.source_config
        .get("subreddit")
        .and_then(|v| v.as_str())
        .unwrap_or("rust");

    let sort = feed.source_config
        .get("sort")
        .and_then(|v| v.as_str())
        .unwrap_or("hot");

    let limit = 100u32;

    let fetch_url = format!("{}/r/{}/{}.json?limit={}", REDDIT_BASE, subreddit, sort, limit);

    let mut req = client.get(&fetch_url)
        .header("User-Agent", USER_AGENT);

    // Conditional headers
    if let Some(ref etag) = feed.etag {
        req = req.header("If-None-Match", etag);
    }
    if let Some(ref lm) = feed.last_modified {
        req = req.header("If-Modified-Since", lm);
    }

    let response = req.send().await.map_err(|e| FeedError::Network {
        url: fetch_url.clone(),
        source: e,
    })?;

    let status = response.status();

    if status.as_u16() == 304 {
        return Ok(RedditFetchResult {
            items: Vec::new(),
            etag: feed.etag.clone(),
            last_modified: feed.last_modified.clone(),
            was_cached: true,
        });
    }

    if !status.is_success() {
        return Err(FeedError::Http {
            url: fetch_url.clone(),
            status: status.as_u16(),
            message: status.canonical_reason().unwrap_or("Unknown").to_string(),
        });
    }

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

    let listing: RedditListing = response.json().await.map_err(|e| FeedError::Network {
        url: fetch_url.clone(),
        source: e,
    })?;

    let ns_uuid = Uuid::new_v5(&Uuid::NAMESPACE_URL, feed.url.as_bytes());

    let items = listing.data.children.into_iter()
        .map(|child| normalize_reddit_post(child.data, &feed.id, ns_uuid, fetched_at))
        .collect();

    Ok(RedditFetchResult {
        items,
        etag: new_etag.or(feed.etag.clone()),
        last_modified: new_last_modified.or(feed.last_modified.clone()),
        was_cached: false,
    })
}

fn normalize_reddit_post(
    post: RedditPost,
    feed_id: &str,
    ns_uuid: Uuid,
    fetched_at: i64,
) -> FeedItem {
    let source_guid = post.id.clone();
    let item_id = Uuid::new_v5(&ns_uuid, source_guid.as_bytes()).to_string();

    let comment_url = format!("{}{}", REDDIT_BASE, post.permalink);
    let is_self = post.is_self.unwrap_or(false);

    // For self posts, URL is the reddit post; for link posts, URL is external
    let url = if is_self {
        Some(comment_url.clone())
    } else {
        post.url
    };

    let published_at = post.created_utc as i64;

    // body_html: prefer selftext_html for self posts
    let body_html = post.selftext_html
        .filter(|s| !s.is_empty() && s != "null");

    // body_text: use selftext for self posts, or strip body_html
    let body_text = post.selftext
        .filter(|s| !s.is_empty() && s != "null")
        .or_else(|| body_html.as_deref().map(strip_html));

    let word_count = body_text.as_deref().map(|t| count_words(t) as i64);

    let thumbnail_url = post.thumbnail.filter(|t| {
        !t.is_empty() && t != "self" && t != "default" && t != "nsfw" && t.starts_with("http")
    });

    let source_meta = serde_json::json!({
        "subreddit": post.subreddit,
        "flair": post.link_flair_text,
        "is_self": is_self,
        "thumbnail_url": thumbnail_url,
    });

    FeedItem {
        id: item_id,
        feed_id: feed_id.to_string(),
        source_guid,
        title: post.title,
        url,
        author: post.author,
        published_at,
        fetched_at,
        body_text,
        body_html,
        word_count,
        score: post.score,
        comment_count: post.num_comments,
        comment_url: Some(comment_url),
        source_meta,
    }
}
