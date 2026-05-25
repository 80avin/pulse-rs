use crate::error::FeedError;
use reqwest::Client;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedCandidate {
    pub feed_url: String,
    pub kind: String, // "reddit" | "hn" | "rss"
    pub name: String,
    pub is_direct_feed: bool,
    pub is_hn: bool,
    /// True when the URL was reachable but no feed source could be found.
    /// The caller should show a warning and let the user correct the URL manually.
    pub no_feed_found: bool,
    pub candidates: Vec<FeedLink>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedLink {
    pub url: String,
    pub title: Option<String>,
}

pub async fn detect_feed_url(client: &Client, raw_url: &str) -> Result<FeedCandidate, FeedError> {
    let url = if raw_url.starts_with("http://") || raw_url.starts_with("https://") {
        raw_url.to_string()
    } else {
        format!("https://{raw_url}")
    };

    let parsed = reqwest::Url::parse(&url).map_err(|e| FeedError::Parse {
        url: url.clone(),
        source: Box::new(e),
    })?;
    let host = parsed.host_str().unwrap_or("").to_lowercase();
    let path = parsed.path().to_lowercase();
    let query = parsed.query().unwrap_or("").to_string();

    tracing::debug!(url = %url, "detecting feed type");

    // Reddit pattern
    if host.contains("reddit.com") {
        let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
        let (name, feed_url) = if parts.len() >= 2 && parts[0] == "r" && !parts[1].is_empty() {
            let sub = parts[1];
            (
                format!("r/{sub}"),
                format!("https://www.reddit.com/r/{sub}.rss"),
            )
        } else {
            ("Reddit".into(), "https://www.reddit.com/.rss".into())
        };
        return Ok(FeedCandidate {
            feed_url,
            kind: "reddit".into(),
            name,
            is_direct_feed: false,
            is_hn: false,
            no_feed_found: false,
            candidates: vec![],
        });
    }

    // HN pattern
    if host.contains("ycombinator.com") || host.contains("hacker-news") {
        return Ok(FeedCandidate {
            feed_url: "https://news.ycombinator.com/rss".into(),
            kind: "hn".into(),
            name: "Hacker News".into(),
            is_direct_feed: false,
            is_hn: true,
            no_feed_found: false,
            candidates: vec![],
        });
    }

    // Well-known sites: derive feed URL + title without a network request
    if let Some(known) = detect_well_known(&host, &path, &query) {
        return Ok(known);
    }

    tracing::debug!(url = %url, "fetching URL for feed discovery");

    // Fetch the URL
    let response = client
        .get(&url)
        .header("User-Agent", "Pulse/1.0 feed-detector")
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| FeedError::Network {
            url: url.clone(),
            source: e,
        })?;

    let status = response.status().as_u16();
    if status >= 400 {
        return Err(FeedError::Http {
            url: url.clone(),
            status,
            message: "request failed".into(),
        });
    }

    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_lowercase();

    let is_feed_ct = content_type.contains("application/rss")
        || content_type.contains("application/atom")
        || content_type.contains("application/feed")
        || (content_type.contains("xml") && !content_type.contains("html"));

    let bytes = response.bytes().await.map_err(|e| FeedError::Network {
        url: url.clone(),
        source: e,
    })?;

    if is_feed_ct {
        let name = feed_rs::parser::parse(bytes.as_ref())
            .ok()
            .and_then(|f| f.title)
            .map(|t| t.content)
            .unwrap_or_else(|| domain_name(&url));
        tracing::debug!(url = %url, name = %name, "direct feed detected");
        return Ok(FeedCandidate {
            feed_url: url,
            kind: "rss".into(),
            name,
            is_direct_feed: true,
            is_hn: false,
            no_feed_found: false,
            candidates: vec![],
        });
    }

    // HTML: discover <link rel="alternate"> tags
    if content_type.contains("text/html") || content_type.contains("application/xhtml") {
        let html_str = String::from_utf8_lossy(&bytes[..bytes.len().min(204_800)]);
        let candidates = find_alternate_links(&html_str, &url);
        if let Some(first) = candidates.first() {
            tracing::debug!(url = %url, feed_url = %first.url, candidates = candidates.len(), "found feed links in HTML");
            return Ok(FeedCandidate {
                feed_url: first.url.clone(),
                kind: "rss".into(),
                name: first.title.clone().unwrap_or_else(|| domain_name(&url)),
                is_direct_feed: false,
                is_hn: false,
                no_feed_found: false,
                candidates,
            });
        }
        // HTML but no RSS links found
        tracing::debug!(url = %url, "no feed links found in HTML");
        return Ok(FeedCandidate {
            feed_url: url.clone(),
            kind: "rss".into(),
            name: domain_name(&url),
            is_direct_feed: false,
            is_hn: false,
            no_feed_found: true,
            candidates: vec![],
        });
    }

    // Non-HTML, non-feed content (JSON API, image, video, etc.)
    Ok(FeedCandidate {
        feed_url: url.clone(),
        kind: "rss".into(),
        name: domain_name(&url),
        is_direct_feed: false,
        is_hn: false,
        no_feed_found: true,
        candidates: vec![],
    })
}

/// Derive feed URL + title for well-known sites without a network request.
fn detect_well_known(host: &str, path: &str, query: &str) -> Option<FeedCandidate> {
    let make = |feed_url: String, name: String, candidates: Vec<FeedLink>| FeedCandidate {
        feed_url,
        kind: "rss".into(),
        name,
        is_direct_feed: false,
        is_hn: false,
        no_feed_found: false,
        candidates,
    };

    // ── YouTube ────────────────────────────────────────────────────────────────
    if host == "youtube.com" || host == "www.youtube.com" {
        // /channel/UCxxxxxxxxxxxxxxxxxxxxxxxx
        if let Some(id) = path.strip_prefix("/channel/") {
            let id = id.trim_end_matches('/');
            if !id.is_empty() {
                return Some(make(
                    format!("https://www.youtube.com/feeds/videos.xml?channel_id={id}"),
                    format!("{id} (YouTube)"),
                    vec![],
                ));
            }
        }
        // /user/username (legacy URLs)
        if let Some(user) = path.strip_prefix("/user/") {
            let user = user.trim_end_matches('/').split('/').next().unwrap_or("");
            if !user.is_empty() {
                return Some(make(
                    format!("https://www.youtube.com/feeds/videos.xml?user={user}"),
                    format!("{user} (YouTube)"),
                    vec![],
                ));
            }
        }
        // /playlist?list=PLxxxxx
        if path.starts_with("/playlist") {
            let pl_id = query.split('&').find_map(|p| p.strip_prefix("list="));
            if let Some(pl_id) = pl_id.filter(|s| !s.is_empty()) {
                return Some(make(
                    format!("https://www.youtube.com/feeds/videos.xml?playlist_id={pl_id}"),
                    format!("YouTube Playlist ({pl_id})"),
                    vec![],
                ));
            }
        }
        // /@handle — YouTube embeds <link rel="alternate"> in page HTML so the
        // generic scraper picks it up. Extract a title hint from the handle.
        if let Some(handle) = path.strip_prefix("/@") {
            let handle = handle.trim_end_matches('/').split('/').next().unwrap_or("");
            if !handle.is_empty() {
                // Return None to fall through to the HTTP scraper,
                // but we can't pre-set the name without the network here.
                let _ = handle;
            }
        }
    }

    // ── GitHub ─────────────────────────────────────────────────────────────────
    if host == "github.com" || host == "www.github.com" {
        let parts: Vec<&str> = path
            .trim_start_matches('/')
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();
        match parts.as_slice() {
            [user, repo] => {
                return Some(FeedCandidate {
                    feed_url: format!("https://github.com/{user}/{repo}/releases.atom"),
                    kind: "rss".into(),
                    name: format!("{user}/{repo}"),
                    is_direct_feed: false,
                    is_hn: false,
                    no_feed_found: false,
                    candidates: vec![
                        FeedLink {
                            url: format!("https://github.com/{user}/{repo}/releases.atom"),
                            title: Some("Releases".into()),
                        },
                        FeedLink {
                            url: format!("https://github.com/{user}/{repo}/commits.atom"),
                            title: Some("Commits".into()),
                        },
                    ],
                });
            }
            [user] if !user.starts_with('.') => {
                return Some(make(
                    format!("https://github.com/{user}.atom"),
                    format!("{user} (GitHub)"),
                    vec![],
                ));
            }
            _ => {}
        }
    }

    // ── Substack ───────────────────────────────────────────────────────────────
    if let Some(subdomain) = host.strip_suffix(".substack.com")
        && !subdomain.is_empty()
    {
        return Some(make(
            format!("https://{host}/feed"),
            format!("{subdomain} (Substack)"),
            vec![],
        ));
    }

    // ── Medium ─────────────────────────────────────────────────────────────────
    if host == "medium.com" || host == "www.medium.com" {
        if let Some(user) = path.strip_prefix("/@") {
            let user = user.trim_end_matches('/').split('/').next().unwrap_or("");
            if !user.is_empty() {
                return Some(make(
                    format!("https://medium.com/feed/@{user}"),
                    format!("@{user} (Medium)"),
                    vec![],
                ));
            }
        }
        let parts: Vec<&str> = path
            .trim_start_matches('/')
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();
        if let [pub_name] = parts.as_slice()
            && !pub_name.starts_with('@')
        {
            return Some(make(
                format!("https://medium.com/feed/{pub_name}"),
                format!("{pub_name} (Medium)"),
                vec![],
            ));
        }
    }

    // ── Dev.to ─────────────────────────────────────────────────────────────────
    if host == "dev.to" || host == "www.dev.to" {
        let parts: Vec<&str> = path
            .trim_start_matches('/')
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();
        if let [user] = parts.as_slice() {
            return Some(make(
                format!("https://dev.to/feed/{user}"),
                format!("{user} (Dev.to)"),
                vec![],
            ));
        }
    }

    // ── Hashnode ───────────────────────────────────────────────────────────────
    if host.ends_with(".hashnode.dev") {
        let subdomain = host.strip_suffix(".hashnode.dev").unwrap_or("");
        if !subdomain.is_empty() {
            return Some(make(
                format!("https://{host}/rss.xml"),
                format!("{subdomain} (Hashnode)"),
                vec![],
            ));
        }
    }

    None
}

fn find_alternate_links(html: &str, base_url: &str) -> Vec<FeedLink> {
    let doc = Html::parse_document(html);
    let Ok(sel) = Selector::parse(
        r#"link[rel="alternate"][type="application/rss+xml"],link[rel="alternate"][type="application/atom+xml"]"#,
    ) else {
        return vec![];
    };
    let base = reqwest::Url::parse(base_url).ok();
    doc.select(&sel)
        .filter_map(|el| {
            let href = el.value().attr("href")?.trim();
            if href.is_empty() {
                return None;
            }
            let absolute = base
                .as_ref()
                .and_then(|b| b.join(href).ok())
                .map(|u| u.to_string())
                .unwrap_or_else(|| href.to_string());
            let title = el.value().attr("title").map(|t| t.trim().to_string());
            Some(FeedLink {
                url: absolute,
                title,
            })
        })
        .collect()
}

fn domain_name(url: &str) -> String {
    reqwest::Url::parse(url)
        .map(|u| u.host_str().unwrap_or("").replace("www.", "").to_string())
        .unwrap_or_default()
}
