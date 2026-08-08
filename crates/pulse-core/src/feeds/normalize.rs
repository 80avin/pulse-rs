use regex::Regex;
use scraper::Html;
use std::sync::OnceLock;

/// Strip HTML tags from a string and decode HTML entities, returning plain text.
pub fn strip_html(html: &str) -> String {
    let fragment = Html::parse_fragment(html);
    let mut text = String::new();
    for node in fragment.tree.nodes() {
        if let scraper::node::Node::Text(t) = node.value() {
            text.push_str(t);
        }
    }
    text
}

/// Normalize text for AI processing:
/// - Strip HTML
/// - Collapse whitespace
/// - Truncate to ~2000 chars
pub fn normalize_text(title: &str, body: Option<&str>) -> String {
    let title_clean = collapse_whitespace(title);

    let body_clean = body
        .map(|b| collapse_whitespace(&strip_html(b)))
        .unwrap_or_default();

    let combined = if body_clean.is_empty() {
        title_clean
    } else {
        format!("{} {}", title_clean, body_clean)
    };

    // Truncate for the rule engine (2000 chars)
    if combined.len() > 2000 {
        combined[..2000].to_string()
    } else {
        combined
    }
}

/// Decode HTML entities in a plain-text string (e.g., Reddit API returns `&amp;` in titles).
/// Reuses the HTML parser because entity decoding is a side effect of fragment parsing.
pub fn decode_html_entities(s: &str) -> String {
    let fragment = Html::parse_fragment(s);
    let mut text = String::with_capacity(s.len());
    for node in fragment.tree.nodes() {
        if let scraper::node::Node::Text(t) = node.value() {
            text.push_str(t);
        }
    }
    text
}

/// Collapse multiple whitespace characters (including newlines) into a single space
pub fn collapse_whitespace(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut last_was_space = true; // start true to trim leading space

    for ch in s.chars() {
        if ch.is_whitespace() {
            if !last_was_space {
                result.push(' ');
                last_was_space = true;
            }
        } else {
            result.push(ch);
            last_was_space = false;
        }
    }

    // Trim trailing space
    if result.ends_with(' ') {
        result.pop();
    }

    result
}

static URL_ATTR_RE: OnceLock<Regex> = OnceLock::new();

const SKIP_URL_PREFIXES: [&str; 7] = ["data:", "http://", "https://", "mailto:", "tel:", "#", "//"];

/// Rewrite relative `src`/`href` attribute values in an HTML string to absolute
/// URLs resolved against `base_url`. Values that already carry a scheme, a
/// protocol-relative prefix (`//`), a fragment, or an empty/whitespace value are
/// left untouched. Only attribute values are rewritten — the rest of the HTML is
/// byte-identical. If `base_url` doesn't parse, the html is returned unchanged.
pub fn resolve_relative_urls(html: &str, base_url: &str) -> String {
    let Ok(base) = reqwest::Url::parse(base_url) else {
        return html.to_string();
    };
    let re = URL_ATTR_RE
        .get_or_init(|| Regex::new(r#"(?i)(src|href)\s*=\s*(['"])([^'"]+)(['"])"#).unwrap());

    let mut out = String::with_capacity(html.len());
    let mut last = 0;
    for caps in re.captures_iter(html) {
        let m = caps.get(0).unwrap();
        out.push_str(&html[last..m.start()]);
        let value = caps.get(3).unwrap();
        let trimmed = value.as_str().trim();
        if trimmed.is_empty() || SKIP_URL_PREFIXES.iter().any(|p| trimmed.starts_with(p)) {
            out.push_str(m.as_str());
        } else {
            match base.join(trimmed) {
                Ok(joined) => {
                    let mstr = m.as_str();
                    let start = value.start() - m.start();
                    let end = value.end() - m.start();
                    out.push_str(&mstr[..start]);
                    out.push_str(joined.as_str());
                    out.push_str(&mstr[end..]);
                }
                Err(_) => out.push_str(m.as_str()),
            }
        }
        last = m.end();
    }
    out.push_str(&html[last..]);
    out
}

/// Count words in a string (approximate)
pub fn count_words(text: &str) -> usize {
    text.split_whitespace().count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_html() {
        let html = "<p>Hello <b>world</b>!</p>";
        assert_eq!(strip_html(html), "Hello world!");
    }

    #[test]
    fn test_collapse_whitespace() {
        assert_eq!(collapse_whitespace("  hello   world  "), "hello world");
        assert_eq!(collapse_whitespace("foo\n\nbar"), "foo bar");
    }

    #[test]
    fn test_normalize_text_truncates() {
        let long_body = "a ".repeat(2000);
        let result = normalize_text("title", Some(&long_body));
        assert!(result.len() <= 2000);
    }

    #[test]
    fn resolve_relative_urls_rewrites_relative_src() {
        let html = r#"<img src="/blog/images/2025/flamescopes1.png" width="700">"#;
        let out = resolve_relative_urls(html, "https://example.com/blog/post");
        assert_eq!(
            out,
            r#"<img src="https://example.com/blog/images/2025/flamescopes1.png" width="700">"#
        );
    }

    #[test]
    fn resolve_relative_urls_rewrites_relative_href() {
        let html = r#"<a href="../next">next</a>"#;
        let out = resolve_relative_urls(html, "https://example.com/blog/2025/post");
        assert_eq!(out, r#"<a href="https://example.com/blog/next">next</a>"#);
    }

    #[test]
    fn resolve_relative_urls_matches_case_insensitively_and_with_spaces() {
        let html = r#"<IMG SRC = "/logo.png">"#;
        let out = resolve_relative_urls(html, "https://example.com");
        assert_eq!(out, r#"<IMG SRC = "https://example.com/logo.png">"#);
    }

    #[test]
    fn resolve_relative_urls_leaves_absolute_and_other_values_unchanged() {
        let html = r#"<img src="https://cdn.example.com/x.png"><a href="http://example.com">x</a>"#;
        assert_eq!(resolve_relative_urls(html, "https://example.com"), html);
    }

    #[test]
    fn resolve_relative_urls_leaves_data_uri_unchanged() {
        let html = r#"<img src="data:image/png;base64,AAA">"#;
        assert_eq!(resolve_relative_urls(html, "https://example.com"), html);
    }

    #[test]
    fn resolve_relative_urls_leaves_protocol_relative_unchanged() {
        let html = r#"<img src="//cdn.example.com/x.png">"#;
        assert_eq!(resolve_relative_urls(html, "https://example.com"), html);
    }

    #[test]
    fn resolve_relative_urls_leaves_anchor_unchanged() {
        let html = r##"<a href="#section">x</a>"##;
        assert_eq!(resolve_relative_urls(html, "https://example.com"), html);
    }

    #[test]
    fn resolve_relative_urls_leaves_mailto_and_tel_unchanged() {
        let html = r#"<a href="mailto:hi@example.com">mail</a><a href="tel:+1234">call</a>"#;
        assert_eq!(resolve_relative_urls(html, "https://example.com"), html);
    }

    #[test]
    fn resolve_relative_urls_empty_base_returns_html_unchanged() {
        let html = r#"<img src="/x.png">"#;
        assert_eq!(resolve_relative_urls(html, ""), html);
    }
}
