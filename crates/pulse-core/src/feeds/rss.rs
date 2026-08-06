use crate::error::FeedError;
use crate::feeds::normalize::{collapse_whitespace, count_words, strip_html};
use crate::types::{Feed, FeedItem};
use reqwest::Client;
use uuid::Uuid;

const USER_AGENT: &str = "Pulse/0.1 (+https://github.com/80avin/pulse-rs; feed-reader)";

pub struct RssFetchResult {
    pub items: Vec<FeedItem>,
    pub feed_title: Option<String>,
    pub feed_description: Option<String>,
    pub feed_site_url: Option<String>,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub was_cached: bool,
}

/// Fetch + parse an RSS/Atom feed, respecting ETag/If-Modified-Since.
pub async fn fetch_rss(client: &Client, feed: &Feed) -> Result<RssFetchResult, FeedError> {
    let fetched_at = chrono::Utc::now().timestamp();
    let url = feed.url.clone();

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

    // Extract caching headers before consuming the response body
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

    let bytes = crate::feeds::read_body_capped(response, 20 * 1024 * 1024).await?;

    let parsed = feed_rs::parser::parse(bytes.as_slice()).map_err(|e| FeedError::Parse {
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
    // source_guid: prefer entry.id, else a hash of the link URL, else a hash of
    // the item's own content — so distinct id-less, link-less entries never collapse.
    let source_guid = if !entry.id.is_empty() {
        entry.id.clone()
    } else if let Some(link) = entry.links.first() {
        // Keep the legacy scheme so existing item IDs (and read/saved state)
        // survive upgrades; "sha256:" is a misnomer for SipHash.
        format!("sha256:{:x}", md5_hash(&link.href))
    } else {
        // No id and no link: previously every such entry hashed the empty string
        // and collapsed into one item. Hash the content so distinct entries differ.
        let title = entry
            .title
            .as_ref()
            .map(|t| collapse_whitespace(&strip_html(&t.content)))
            .unwrap_or_default();
        let published = entry
            .published
            .or(entry.updated)
            .map(|dt| dt.timestamp())
            .unwrap_or(fetched_at);
        format!("sha256:{:x}:{}", md5_hash(&format!("{title}:{published}")), published)
    };

    let item_id = Uuid::new_v5(&ns_uuid, source_guid.as_bytes()).to_string();

    let title = entry
        .title
        .as_ref()
        .map(|t| collapse_whitespace(&strip_html(&t.content)))
        .unwrap_or_else(|| "(no title)".to_string());

    let url = entry.links.first().map(|l| l.href.clone());

    let author = entry.authors.first().and_then(|a| {
        // feed-rs parses RSS <author>text</author> as name="author" + email=text
        // (its handle_contact sets name to the element role). Prefer the email
        // content so the real author name/address isn't replaced by "author".
        if a.name == "author" {
            a.email.clone().or_else(|| Some(a.name.clone()))
        } else {
            Some(a.name.clone())
        }
    });

    let published_at = entry
        .published
        .or(entry.updated)
        .map(|dt| dt.timestamp())
        .unwrap_or(fetched_at);

    let body_html = entry
        .content
        .as_ref()
        .and_then(|c| c.body.as_ref())
        .cloned()
        .or_else(|| entry.summary.as_ref().map(|s| s.content.clone()));

    let body_text = body_html.as_deref().map(strip_html);

    let word_count = body_text.as_deref().map(|t| count_words(t) as i64);

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

/// SipHash-based fallback GUID hash. Legacy label says "md5"/"sha256"; the
/// algorithm is SipHash via `DefaultHasher` (stable in practice, kept to
/// preserve existing item IDs across upgrades).
fn md5_hash(s: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}


#[cfg(test)]
mod tests {
    use super::*;
    use feed_rs::model::{Entry, Link, Text};
    use uuid::Uuid;

    fn entry() -> Entry {
        Entry::default()
    }

    fn text(s: &str) -> Text {
        Text {
            content_type: "text/plain".parse().unwrap(),
            src: None,
            content: s.into(),
        }
    }

    fn link(href: &str) -> Link {
        Link { href: href.into(), rel: None, media_type: None, href_lang: None, title: None, length: None }
    }

    fn guid(e: &Entry) -> String {
        normalize_rss_entry(e.clone(), "feed", "https://x", Uuid::new_v4(), 1_700_000_000).source_guid
    }

    #[test]
    fn empty_id_and_empty_link_entries_do_not_collapse() {
        // Two distinct entries with neither id nor link previously hashed the
        // empty string and collapsed into ONE item. They must stay distinct.
        let mut a = entry();
        a.title = Some(text("First post"));
        let mut b = entry();
        b.title = Some(text("Second post"));
        assert_ne!(guid(&a), guid(&b), "distinct items collapsed to one guid");
    }

    #[test]
    fn linked_guid_is_stable_and_unique() {
        let mut a = entry();
        a.links = vec![link("https://example.com/1")];
        let mut b = entry();
        b.links = vec![link("https://example.com/1")];
        assert_eq!(guid(&a), guid(&b));
        let mut c = entry();
        c.links = vec![link("https://example.com/2")];
        assert_ne!(guid(&a), guid(&c));
    }

    #[test]
    fn real_entry_id_is_used_verbatim() {
        let mut a = entry();
        a.id = "post-1".into();
        assert_eq!(guid(&a), "post-1");
    }

    #[test]
    fn legacy_link_guid_scheme_is_preserved() {
        // The old scheme hashed only the href; changing it would re-ingest every
        // id-less linked item as a new row on upgrade. Verify it's unchanged.
        let mut a = entry();
        a.links = vec![link("https://example.com/stable")];
        assert_eq!(guid(&a), format!("sha256:{:x}", md5_hash("https://example.com/stable")));
    }
}
