use crate::error::FeedError;
use reqwest::Client;
use scraper::{Html, Selector};

const ENRICH_TIMEOUT_SECS: u64 = 8;

static IMAGE_EXTENSIONS: &[&str] = &[
    ".jpg", ".jpeg", ".png", ".gif", ".webp", ".avif", ".svg", ".bmp",
];
static IMAGE_DOMAINS: &[&str] = &[
    "i.redd.it",
    "i.imgur.com",
    "pbs.twimg.com",
    "media.giphy.com",
];
// Internal URLs we already have full content for — skip HTTP fetch
static SKIP_DOMAINS: &[&str] = &[
    "news.ycombinator.com",
    "reddit.com/r/",
    "reddit.com/comments/",
];

#[derive(Debug, Default)]
pub struct EnrichmentResult {
    pub og_title: Option<String>,
    pub og_description: Option<String>,
    pub og_image: Option<String>,
    pub og_site_name: Option<String>,
    pub og_type: Option<String>,
    pub canonical_url: Option<String>,
    pub is_image: bool,
    pub skipped: bool,
}

/// Detect image-only URLs (no body to extract)
pub fn is_image_url(url: &str) -> bool {
    let u = url.split('?').next().unwrap_or(url).to_lowercase();
    IMAGE_EXTENSIONS.iter().any(|ext| u.ends_with(ext))
        || IMAGE_DOMAINS.iter().any(|d| url.contains(d))
}

/// Returns false for URLs we shouldn't bother fetching
pub fn should_enrich(url: &str) -> bool {
    if is_image_url(url) {
        return false;
    }
    if SKIP_DOMAINS.iter().any(|d| url.contains(d)) {
        return false;
    }
    true
}

/// Fetch Open Graph / Twitter Card / meta description for a URL.
/// Returns `skipped=true` when the URL type doesn't benefit from fetching.
pub async fn fetch_enrichment(client: &Client, url: &str) -> Result<EnrichmentResult, FeedError> {
    if is_image_url(url) {
        return Ok(EnrichmentResult {
            is_image: true,
            ..Default::default()
        });
    }
    if !should_enrich(url) {
        return Ok(EnrichmentResult {
            skipped: true,
            ..Default::default()
        });
    }

    let response = client
        .get(url)
        .timeout(std::time::Duration::from_secs(ENRICH_TIMEOUT_SECS))
        // Signal we only want metadata — some servers return lighter pages
        .header("Accept", "text/html,application/xhtml+xml")
        .send()
        .await
        .map_err(|e| FeedError::Network {
            url: url.to_string(),
            source: e,
        })?;

    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_lowercase();

    if content_type.starts_with("image/") {
        return Ok(EnrichmentResult {
            is_image: true,
            ..Default::default()
        });
    }
    if !content_type.contains("text/html") && !content_type.contains("application/xhtml") {
        return Ok(EnrichmentResult {
            skipped: true,
            ..Default::default()
        });
    }

    // Read at most 200 KB — enough for <head> on any page
    let bytes = response.bytes().await.map_err(|e| FeedError::Network {
        url: url.to_string(),
        source: e,
    })?;
    let html = String::from_utf8_lossy(&bytes[..bytes.len().min(204_800)]);

    Ok(parse_meta(&html))
}

fn parse_meta(html: &str) -> EnrichmentResult {
    let doc = Html::parse_document(html);
    let mut r = EnrichmentResult::default();

    // <meta property="og:*" content="...">
    if let Ok(sel) = Selector::parse("meta[property]") {
        for el in doc.select(&sel) {
            let prop = el.value().attr("property").unwrap_or("");
            let content = el.value().attr("content").unwrap_or("").trim();
            if content.is_empty() {
                continue;
            }
            match prop {
                "og:title" => r.og_title = Some(content.to_string()),
                "og:description" => r.og_description = Some(content.to_string()),
                "og:image" => {
                    if r.og_image.is_none() {
                        r.og_image = Some(content.to_string());
                    }
                }
                "og:image:url" => r.og_image = Some(content.to_string()), // overrides og:image if more specific
                "og:site_name" => r.og_site_name = Some(content.to_string()),
                "og:type" => r.og_type = Some(content.to_string()),
                "og:url" => r.canonical_url = Some(content.to_string()),
                _ => {}
            }
        }
    }

    // <meta name="twitter:*"> and <meta name="description"> — fill gaps only
    if let Ok(sel) = Selector::parse("meta[name]") {
        for el in doc.select(&sel) {
            let name = el.value().attr("name").unwrap_or("");
            let content = el.value().attr("content").unwrap_or("").trim();
            if content.is_empty() {
                continue;
            }
            match name {
                "twitter:title" => {
                    if r.og_title.is_none() {
                        r.og_title = Some(content.to_string());
                    }
                }
                "twitter:description" => {
                    if r.og_description.is_none() {
                        r.og_description = Some(content.to_string());
                    }
                }
                "twitter:image" | "twitter:image:src" => {
                    if r.og_image.is_none() {
                        r.og_image = Some(content.to_string());
                    }
                }
                "description" => {
                    if r.og_description.is_none() {
                        r.og_description = Some(content.to_string());
                    }
                }
                _ => {}
            }
        }
    }

    // <link rel="canonical" href="...">
    if r.canonical_url.is_none() {
        if let Ok(sel) = Selector::parse(r#"link[rel="canonical"]"#) {
            if let Some(el) = doc.select(&sel).next() {
                if let Some(href) = el.value().attr("href") {
                    let href = href.trim();
                    if !href.is_empty() {
                        r.canonical_url = Some(href.to_string());
                    }
                }
            }
        }
    }

    r
}
