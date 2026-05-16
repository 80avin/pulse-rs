use scraper::Html;

/// Strip HTML tags from a string and decode HTML entities, returning plain text.
pub fn strip_html(html: &str) -> String {
    let fragment = Html::parse_fragment(html);
    // Collect all text nodes
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

    // Truncate to 2000 chars for rule engine (model uses 512 tokens, handled separately)
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
}
