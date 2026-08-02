pub mod detect;
pub mod enrich;
pub mod hackernews;
pub mod normalize;
pub mod reddit;
pub mod reddit_auth;
pub mod rss;

pub use detect::{FeedCandidate, FeedLink, detect_feed_url};
pub use enrich::{EnrichmentResult, fetch_enrichment, is_image_url, should_enrich};
pub use hackernews::fetch_hn;
pub use reddit::fetch_reddit;
pub use reddit_auth::RedditAuth;
pub use rss::fetch_rss;

use crate::error::FeedError;

/// Read a response body fully but abort once `max_bytes` is exceeded, so a
/// malicious or broken source cannot exhaust the process's memory. Also honors
/// a `Content-Length` header when present.
pub(crate) async fn read_body_capped(
    response: reqwest::Response,
    max_bytes: usize,
) -> Result<Vec<u8>, FeedError> {
    use futures::StreamExt;
    let url = response.url().to_string();
    if let Some(len) = response.content_length()
        && len > max_bytes as u64
    {
        return Err(FeedError::BodyTooLarge { url, limit: max_bytes });
    }
    let mut body = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|source| FeedError::Network { url: url.clone(), source })?;
        if body.len().saturating_add(chunk.len()) > max_bytes {
            return Err(FeedError::BodyTooLarge { url, limit: max_bytes });
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}
