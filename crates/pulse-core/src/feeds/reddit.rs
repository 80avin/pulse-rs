use reqwest::Client;
use serde::Deserialize;
use uuid::Uuid;
use crate::error::FeedError;
use crate::types::{Feed, FeedItem};
use crate::feeds::normalize::{strip_html, count_words, collapse_whitespace, decode_html_entities};
use crate::feeds::reddit_auth::RedditAuth;

const USER_AGENT: &str = "Pulse/0.1 (+https://github.com/avinthakur080/pulse-rs; feed-reader)";
const REDDIT_BASE: &str = "https://www.reddit.com";
const REDDIT_OAUTH_BASE: &str = "https://oauth.reddit.com";

/// Result of a Reddit fetch
pub struct RedditFetchResult {
    pub items: Vec<FeedItem>,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub was_cached: bool,
}

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
    // post_hint: "self" | "link" | "image" | "rich:video" | "hosted:video"
    post_hint: Option<String>,
    // preview images
    preview: Option<RedditPreview>,
    // crosspost chain
    crosspost_parent_list: Option<Vec<CrosspostParent>>,
}

#[derive(Debug, Deserialize)]
struct RedditPreview {
    images: Option<Vec<RedditPreviewImage>>,
}

#[derive(Debug, Deserialize)]
struct RedditPreviewImage {
    source: Option<RedditImageSource>,
}

#[derive(Debug, Deserialize)]
struct RedditImageSource {
    url: Option<String>,
}

/// Condensed crosspost parent — only the fields we care about
#[derive(Debug, Deserialize)]
struct CrosspostParent {
    title: Option<String>,
    selftext: Option<String>,
    selftext_html: Option<String>,
    url: Option<String>,
    subreddit: Option<String>,
    author: Option<String>,
    score: Option<i64>,
    permalink: Option<String>,
    post_hint: Option<String>,
}

pub async fn fetch_reddit(client: &Client, feed: &Feed, auth: Option<&RedditAuth>) -> Result<RedditFetchResult, FeedError> {
    let fetched_at = chrono::Utc::now().timestamp();

    // Prefer explicit source_config; fall back to parsing from the feed URL.
    // Feed URL format: https://www.reddit.com/r/{subreddit}/{sort}.json
    let subreddit = feed.source_config
        .get("subreddit").and_then(|v| v.as_str())
        .or_else(|| subreddit_from_url(&feed.url))
        .unwrap_or("rust");
    let sort = feed.source_config
        .get("sort").and_then(|v| v.as_str())
        .or_else(|| sort_from_url(&feed.url))
        .unwrap_or("hot");
    let limit = 100u32;

    // Authenticated requests use oauth.reddit.com (higher rate limits, no TLS fingerprint issues).
    let base = if auth.is_some() { REDDIT_OAUTH_BASE } else { REDDIT_BASE };
    let fetch_url = format!("{}/r/{}/{}.json?limit={}", base, subreddit, sort, limit);

    let mut req = client.get(&fetch_url).header("User-Agent", USER_AGENT);
    if let Some(reddit_auth) = auth {
        let token = reddit_auth.token(client).await?;
        req = req.header("Authorization", format!("Bearer {}", token));
    }
    if let Some(ref etag) = feed.etag { req = req.header("If-None-Match", etag); }
    if let Some(ref lm) = feed.last_modified { req = req.header("If-Modified-Since", lm); }

    let response = req.send().await.map_err(|e| FeedError::Network { url: fetch_url.clone(), source: e })?;
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

    let new_etag = response.headers()
        .get(reqwest::header::ETAG)
        .and_then(|v| v.to_str().ok()).map(|s| s.to_string());
    let new_last_modified = response.headers()
        .get(reqwest::header::LAST_MODIFIED)
        .and_then(|v| v.to_str().ok()).map(|s| s.to_string());

    let listing: RedditListing = response.json().await.map_err(|e| FeedError::Network {
        url: fetch_url.clone(), source: e,
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

fn normalize_reddit_post(post: RedditPost, feed_id: &str, ns_uuid: Uuid, fetched_at: i64) -> FeedItem {
    let source_guid = post.id.clone();
    let item_id = Uuid::new_v5(&ns_uuid, source_guid.as_bytes()).to_string();
    let comment_url = format!("{}{}", REDDIT_BASE, post.permalink);
    let is_self = post.is_self.unwrap_or(false);
    let post_hint = post.post_hint.as_deref().unwrap_or(if is_self { "self" } else { "link" });

    // Always use the Reddit permalink as the primary URL so the Open button
    // navigates to the Reddit post page. For link posts, the external target
    // is stored in source_meta["external_url"] and shown as a secondary link.
    let external_url = if !is_self { post.url.clone() } else { None };
    let url = Some(comment_url.clone());
    let published_at = post.created_utc as i64;

    // ── Body text assembly ────────────────────────────────────────────────────
    // Priority: self-text → crosspost self-text → (enrichment fills in link posts later)
    let own_body_html = post.selftext_html.as_deref()
        .filter(|s| !s.is_empty() && *s != "null")
        .map(|s| s.to_string());

    let own_body_text = post.selftext.as_deref()
        .filter(|s| !s.is_empty() && *s != "null")
        .map(|s| collapse_whitespace(s))
        .or_else(|| own_body_html.as_deref().map(strip_html));

    // Resolve crosspost chain (take first parent)
    let crosspost = post.crosspost_parent_list.as_deref()
        .and_then(|v| v.first());

    let (body_text, body_html, crosspost_meta) = if let Some(cp) = crosspost {
        let cp_body_html = cp.selftext_html.as_deref()
            .filter(|s| !s.is_empty() && *s != "null")
            .map(|s| s.to_string());
        let cp_body_text = cp.selftext.as_deref()
            .filter(|s| !s.is_empty() && *s != "null")
            .map(|s| collapse_whitespace(s))
            .or_else(|| cp_body_html.as_deref().map(strip_html));

        // Merge own + parent body
        let merged_text = match (own_body_text.as_deref(), cp_body_text.as_deref()) {
            (Some(own), Some(cp_t)) => Some(format!("{}\n\n[crosspost from r/{}]: {}", own, cp.subreddit.as_deref().unwrap_or("?"), cp_t)),
            (None, Some(cp_t)) => Some(format!("[crosspost from r/{}]: {}", cp.subreddit.as_deref().unwrap_or("?"), cp_t)),
            (Some(own), None) => Some(own.to_string()),
            (None, None) => None,
        };
        let merged_html = own_body_html.or(cp_body_html);

        let cp_meta = serde_json::json!({
            "parent_title": cp.title,
            "parent_subreddit": cp.subreddit,
            "parent_author": cp.author,
            "parent_url": cp.url,
            "parent_permalink": cp.permalink.as_deref().map(|p| format!("{}{}", REDDIT_BASE, p)),
            "parent_score": cp.score,
            "parent_post_hint": cp.post_hint,
        });
        (merged_text, merged_html, Some(cp_meta))
    } else {
        (own_body_text, own_body_html, None)
    };

    let word_count = body_text.as_deref().map(|t| count_words(t) as i64);

    // ── Best thumbnail / preview image ────────────────────────────────────────
    let preview_image_url = post.preview.as_ref()
        .and_then(|p| p.images.as_deref())
        .and_then(|imgs| imgs.first())
        .and_then(|img| img.source.as_ref())
        .and_then(|src| src.url.as_deref())
        // Reddit encodes preview URLs with HTML entities; decode &amp; → &
        .map(|u| u.replace("&amp;", "&"));

    let thumbnail_url = post.thumbnail.as_deref().filter(|t| {
        !t.is_empty() && *t != "self" && *t != "default" && *t != "nsfw"
            && *t != "image" && *t != "spoiler" && t.starts_with("http")
    }).map(|s| s.to_string());

    // Use full preview image over thumbnail if available
    let best_image = preview_image_url.or(thumbnail_url);

    // ── source_meta ───────────────────────────────────────────────────────────
    let mut meta = serde_json::json!({
        "subreddit": post.subreddit,
        "flair": post.link_flair_text,
        "is_self": is_self,
        "post_hint": post_hint,
        "og_image": best_image,
        "external_url": external_url,
    });

    if let Some(cp_meta) = crosspost_meta {
        meta.as_object_mut().unwrap().insert("crosspost".to_string(), cp_meta);
    }

    FeedItem {
        id: item_id,
        feed_id: feed_id.to_string(),
        source_guid,
        title: decode_html_entities(&post.title),
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
        source_meta: meta,
    }
}

/// Extract subreddit name from a Reddit URL.
/// e.g. `https://www.reddit.com/r/rust/hot.json` → `"rust"`
fn subreddit_from_url(url: &str) -> Option<&str> {
    let after_r = url.split("/r/").nth(1)?;
    let sub = after_r.split('/').next()?;
    if sub.is_empty() { None } else { Some(sub) }
}

/// Extract sort order from a Reddit URL.
/// e.g. `https://www.reddit.com/r/rust/hot.json` → `"hot"`
fn sort_from_url(url: &str) -> Option<&str> {
    let after_r = url.split("/r/").nth(1)?;
    let mut parts = after_r.splitn(3, '/');
    let _ = parts.next()?; // subreddit
    let sort_part = parts.next()?;
    let sort = sort_part.split('.').next()?;
    if sort.is_empty() { None } else { Some(sort) }
}
