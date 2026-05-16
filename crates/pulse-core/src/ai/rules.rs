use regex::Regex;
use crate::types::{FeedItem, FeedType, TagResult, TaggerSource};

/// Scope of text to match against
#[derive(Debug, Clone)]
pub enum RuleScope {
    /// Match against title + body
    All,
    /// Match only against title
    TitleOnly,
    /// Match only against body_text
    BodyOnly,
}

/// A single pattern within a rule
#[derive(Debug, Clone)]
pub enum RulePattern {
    /// Case-insensitive substring match
    Keyword(String),
    /// Precompiled regex
    Regex(Regex),
    /// Matches if the item URL domain contains this string
    DomainMatch(String),
    /// Item score >= min (Reddit/HN)
    HasScore { min: i64 },
    /// Item comment_count >= min
    HasComments { min: i64 },
    /// Only matches items from a specific feed type
    FeedType(FeedType),
}

impl RulePattern {
    fn matches(&self, item: &FeedItem, text: &str, feed_type: &FeedType) -> bool {
        match self {
            RulePattern::Keyword(kw) => {
                text.to_lowercase().contains(kw.as_str())
            }
            RulePattern::Regex(re) => re.is_match(text),
            RulePattern::DomainMatch(domain) => {
                item.url.as_deref()
                    .map(|u| u.to_lowercase().contains(domain.as_str()))
                    .unwrap_or(false)
            }
            RulePattern::HasScore { min } => {
                item.score.map(|s| s >= *min).unwrap_or(false)
            }
            RulePattern::HasComments { min } => {
                item.comment_count.map(|c| c >= *min).unwrap_or(false)
            }
            RulePattern::FeedType(ft) => feed_type == ft,
        }
    }

    fn matched_text<'a>(&'a self, text: &'a str) -> Option<&'a str> {
        match self {
            RulePattern::Keyword(kw) => {
                // Check if keyword appears in the text (case-insensitive)
                if text.to_lowercase().contains(kw.as_str()) {
                    Some(kw.as_str())
                } else {
                    None
                }
            }
            RulePattern::Regex(re) => {
                re.find(text).map(|m| m.as_str())
            }
            _ => None,
        }
    }
}

/// A tag rule
#[derive(Debug, Clone)]
pub struct TagRule {
    pub id: String,
    pub tag: String,
    pub confidence: f32,
    pub explanation_template: String,
    pub patterns: Vec<RulePattern>,
    pub scope: RuleScope,
    /// If true, ALL patterns must match (AND). If false, ANY pattern matches (OR).
    pub require_all: bool,
    pub enabled: bool,
}

impl TagRule {
    /// Evaluate this rule against a feed item. Returns a TagResult if it matches.
    pub fn evaluate(&self, item: &FeedItem, feed_type: &FeedType) -> Option<TagResult> {
        if !self.enabled {
            return None;
        }

        // Build text to match against based on scope
        let title = item.title.as_str();
        let body = item.body_text.as_deref().unwrap_or("");

        let text_for_matching = match self.scope {
            RuleScope::All => {
                if body.is_empty() {
                    title.to_string()
                } else {
                    format!("{} {}", title, body)
                }
            }
            RuleScope::TitleOnly => title.to_string(),
            RuleScope::BodyOnly => body.to_string(),
        };

        let matched_pattern = if self.require_all {
            // AND: all patterns must match
            let all_match = self.patterns.iter().all(|p| p.matches(item, &text_for_matching, feed_type));
            if all_match {
                self.patterns.first().and_then(|p| p.matched_text(&text_for_matching))
                    .unwrap_or("all conditions")
                    .to_string()
            } else {
                return None;
            }
        } else {
            // OR: find first matching pattern
            let first_match = self.patterns.iter().find_map(|p| {
                if p.matches(item, &text_for_matching, feed_type) {
                    Some(p.matched_text(&text_for_matching)
                        .unwrap_or("condition")
                        .to_string())
                } else {
                    None
                }
            });

            match first_match {
                Some(m) => m,
                None => return None,
            }
        };

        let explanation = self.explanation_template
            .replace("{matched_text}", &matched_pattern)
            .replace("{tag}", &self.tag);

        Some(TagResult {
            tag: self.tag.clone(),
            confidence: self.confidence,
            explanation,
            source: TaggerSource::Rule,
            rule_id: Some(self.id.clone()),
        })
    }
}

/// The rule evaluation engine
pub struct RuleEngine {
    rules: Vec<TagRule>,
}

impl RuleEngine {
    pub fn new(rules: Vec<TagRule>) -> Self {
        Self { rules }
    }

    /// Evaluate all rules against an item. Returns all matching tag results.
    /// All rules are evaluated (no short-circuit) to allow multiple tags per item.
    pub fn evaluate(&self, item: &FeedItem, feed_type: &FeedType) -> Vec<TagResult> {
        self.rules
            .iter()
            .filter_map(|rule| rule.evaluate(item, feed_type))
            .collect()
    }
}

/// Build the default set of rules
pub fn default_rules() -> Vec<TagRule> {
    vec![
        TagRule {
            id: "technical".to_string(),
            tag: "technical".to_string(),
            confidence: 0.80,
            explanation_template: "matched keyword '{matched_text}' in content".to_string(),
            patterns: vec![
                RulePattern::Keyword("github.com".to_string()),
                RulePattern::Keyword("crates.io".to_string()),
                RulePattern::Keyword("npm".to_string()),
                RulePattern::Keyword("docker".to_string()),
                RulePattern::Keyword("kubernetes".to_string()),
                RulePattern::Keyword(" api ".to_string()),
                RulePattern::Keyword("framework".to_string()),
                RulePattern::Keyword("library".to_string()),
                RulePattern::Keyword("algorithm".to_string()),
                RulePattern::Keyword("performance".to_string()),
                RulePattern::Keyword("rust".to_string()),
                RulePattern::Keyword("python".to_string()),
                RulePattern::Keyword("typescript".to_string()),
                RulePattern::Keyword("golang".to_string()),
                RulePattern::Keyword(" sql".to_string()),
                RulePattern::Keyword("linux".to_string()),
                RulePattern::DomainMatch("github.com".to_string()),
                RulePattern::DomainMatch("crates.io".to_string()),
            ],
            scope: RuleScope::All,
            require_all: false,
            enabled: true,
        },

        TagRule {
            id: "tutorial".to_string(),
            tag: "tutorial".to_string(),
            confidence: 0.85,
            explanation_template: "matched tutorial keyword '{matched_text}' in title".to_string(),
            patterns: vec![
                RulePattern::Keyword("how to".to_string()),
                RulePattern::Keyword("tutorial".to_string()),
                RulePattern::Keyword("guide".to_string()),
                RulePattern::Keyword("step by step".to_string()),
                RulePattern::Keyword("getting started".to_string()),
                RulePattern::Keyword("introduction to".to_string()),
                RulePattern::Keyword("beginner".to_string()),
                RulePattern::Keyword("walkthrough".to_string()),
            ],
            scope: RuleScope::TitleOnly,
            require_all: false,
            enabled: true,
        },

        TagRule {
            id: "research".to_string(),
            tag: "research".to_string(),
            confidence: 0.80,
            explanation_template: "matched research keyword '{matched_text}'".to_string(),
            patterns: vec![
                RulePattern::Keyword("arxiv.org".to_string()),
                RulePattern::Keyword("paper".to_string()),
                RulePattern::Keyword("study".to_string()),
                RulePattern::Keyword("research".to_string()),
                RulePattern::Keyword("findings".to_string()),
                RulePattern::Keyword("dataset".to_string()),
                RulePattern::Keyword("benchmark".to_string()),
                RulePattern::Keyword("evaluation".to_string()),
                RulePattern::Keyword("methodology".to_string()),
                RulePattern::DomainMatch("arxiv.org".to_string()),
            ],
            scope: RuleScope::All,
            require_all: false,
            enabled: true,
        },

        TagRule {
            id: "news".to_string(),
            tag: "news".to_string(),
            confidence: 0.75,
            explanation_template: "matched news keyword '{matched_text}' in title".to_string(),
            patterns: vec![
                RulePattern::Keyword("announces".to_string()),
                RulePattern::Keyword("releases".to_string()),
                RulePattern::Keyword("launches".to_string()),
                RulePattern::Keyword("acquires".to_string()),
                RulePattern::Keyword("raises".to_string()),
                RulePattern::Keyword("funding".to_string()),
                RulePattern::Keyword("partnership".to_string()),
                RulePattern::Keyword("breach".to_string()),
                RulePattern::Keyword("outage".to_string()),
            ],
            scope: RuleScope::TitleOnly,
            require_all: false,
            enabled: true,
        },

        TagRule {
            id: "discussion".to_string(),
            tag: "discussion".to_string(),
            confidence: 0.90,
            explanation_template: "matched discussion keyword '{matched_text}' in title".to_string(),
            patterns: vec![
                RulePattern::Keyword("ask hn:".to_string()),
                RulePattern::Keyword("ask reddit:".to_string()),
                RulePattern::Keyword("discussion:".to_string()),
                RulePattern::Keyword("thoughts on".to_string()),
                RulePattern::Keyword("what do you think".to_string()),
                RulePattern::Keyword("opinion".to_string()),
            ],
            scope: RuleScope::TitleOnly,
            require_all: false,
            enabled: true,
        },

        TagRule {
            id: "clickbait".to_string(),
            tag: "clickbait".to_string(),
            confidence: 0.85,
            explanation_template: "matched clickbait pattern '{matched_text}' in title".to_string(),
            patterns: vec![
                RulePattern::Keyword("you won't believe".to_string()),
                RulePattern::Keyword("shocking".to_string()),
                RulePattern::Keyword("mind-blowing".to_string()),
                RulePattern::Keyword("game-changing".to_string()),
                RulePattern::Keyword("destroyed".to_string()),
                RulePattern::Keyword("obliterated".to_string()),
                RulePattern::Keyword("?!".to_string()),
                RulePattern::Keyword("!!!".to_string()),
                RulePattern::Regex(Regex::new(r"(?i)\d+ reasons why").unwrap()),
                RulePattern::Regex(Regex::new(r"(?i)the [a-z]+ that changed everything").unwrap()),
            ],
            scope: RuleScope::TitleOnly,
            require_all: false,
            enabled: true,
        },

        // low-effort is handled entirely by evaluate_low_effort() with compound logic;
        // this TagRule entry is intentionally disabled to avoid false positives.
        TagRule {
            id: "low-effort".to_string(),
            tag: "low-effort".to_string(),
            confidence: 0.70,
            explanation_template: "low score and short content on Reddit".to_string(),
            patterns: vec![
                RulePattern::FeedType(FeedType::Reddit),
            ],
            scope: RuleScope::All,
            require_all: false,
            enabled: false,
        },

        // ragebait (disabled by default — high false-positive risk)
        TagRule {
            id: "ragebait".to_string(),
            tag: "ragebait".to_string(),
            confidence: 0.50,
            explanation_template: "matched ragebait pattern '{matched_text}' in title".to_string(),
            patterns: vec![
                RulePattern::Keyword("they want to destroy".to_string()),
                RulePattern::Keyword("is destroying our".to_string()),
                RulePattern::Keyword("BREAKING ALERT:".to_string()),
            ],
            scope: RuleScope::TitleOnly,
            require_all: false,
            enabled: false, // disabled by default
        },

        TagRule {
            id: "job-posting".to_string(),
            tag: "job-posting".to_string(),
            confidence: 0.90,
            explanation_template: "matched job posting keyword '{matched_text}'".to_string(),
            patterns: vec![
                RulePattern::Keyword("who is hiring".to_string()),
                RulePattern::Keyword("job opening".to_string()),
                RulePattern::Keyword("we're looking for".to_string()),
                RulePattern::Keyword("join our team".to_string()),
                RulePattern::Keyword("we are hiring".to_string()),
            ],
            scope: RuleScope::All,
            require_all: false,
            enabled: true,
        },

        TagRule {
            id: "show-hn".to_string(),
            tag: "show-hn".to_string(),
            confidence: 0.99,
            explanation_template: "HN Show HN post detected".to_string(),
            patterns: vec![
                RulePattern::Keyword("show hn:".to_string()),
                RulePattern::FeedType(FeedType::Hn),
            ],
            scope: RuleScope::TitleOnly,
            require_all: true, // title must contain "Show HN:" AND feed must be HN
            enabled: true,
        },

        TagRule {
            id: "ask-hn".to_string(),
            tag: "ask-hn".to_string(),
            confidence: 0.99,
            explanation_template: "HN Ask HN post detected".to_string(),
            patterns: vec![
                RulePattern::Keyword("ask hn:".to_string()),
                RulePattern::FeedType(FeedType::Hn),
            ],
            scope: RuleScope::TitleOnly,
            require_all: true, // title must contain "Ask HN:" AND feed must be HN
            enabled: true,
        },

        TagRule {
            id: "paywall".to_string(),
            tag: "paywall".to_string(),
            confidence: 0.95,
            explanation_template: "URL matches known paywall domain".to_string(),
            patterns: vec![
                RulePattern::DomainMatch("nytimes.com".to_string()),
                RulePattern::DomainMatch("wsj.com".to_string()),
                RulePattern::DomainMatch("ft.com".to_string()),
                RulePattern::DomainMatch("theatlantic.com".to_string()),
                RulePattern::DomainMatch("wired.com".to_string()),
                RulePattern::DomainMatch("technologyreview.com".to_string()),
                RulePattern::DomainMatch("economist.com".to_string()),
                RulePattern::DomainMatch("bloomberg.com/news".to_string()),
            ],
            scope: RuleScope::All,
            require_all: false,
            enabled: true,
        },

        TagRule {
            id: "video".to_string(),
            tag: "video".to_string(),
            confidence: 0.99,
            explanation_template: "URL matches video platform domain".to_string(),
            patterns: vec![
                RulePattern::DomainMatch("youtube.com".to_string()),
                RulePattern::DomainMatch("youtu.be".to_string()),
                RulePattern::DomainMatch("vimeo.com".to_string()),
                RulePattern::DomainMatch("twitch.tv".to_string()),
            ],
            scope: RuleScope::All,
            require_all: false,
            enabled: true,
        },
    ]
}

/// Special evaluation for low-effort rule (requires score check + body length check)
pub fn evaluate_low_effort(item: &FeedItem, feed_type: &FeedType) -> Option<TagResult> {
    if feed_type != &FeedType::Reddit {
        return None;
    }

    let has_low_score = item.score.map(|s| s <= -5).unwrap_or(false);
    let has_short_body = item.body_text.as_deref()
        .map(|b| b.len() < 50)
        .unwrap_or(true); // no body = short

    if has_low_score && has_short_body {
        Some(TagResult {
            tag: "low-effort".to_string(),
            confidence: 0.70,
            explanation: format!(
                "Reddit post with score {} and short body",
                item.score.unwrap_or(0)
            ),
            source: TaggerSource::Rule,
            rule_id: Some("low-effort".to_string()),
        })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_item(title: &str, url: Option<&str>, body: Option<&str>) -> FeedItem {
        FeedItem {
            id: "test-id".to_string(),
            feed_id: "feed-id".to_string(),
            source_guid: "guid".to_string(),
            title: title.to_string(),
            url: url.map(|s| s.to_string()),
            author: None,
            published_at: 0,
            fetched_at: 0,
            body_text: body.map(|s| s.to_string()),
            body_html: None,
            word_count: None,
            score: None,
            comment_count: None,
            comment_url: None,
            source_meta: serde_json::Value::Object(serde_json::Map::new()),
        }
    }

    #[test]
    fn test_technical_rule_matches_github() {
        let rules = default_rules();
        let engine = RuleEngine::new(rules);
        let item = make_item("New library on github.com", None, None);
        let results = engine.evaluate(&item, &FeedType::Rss);
        assert!(results.iter().any(|r| r.tag == "technical"));
    }

    #[test]
    fn test_paywall_rule_matches_nytimes() {
        let rules = default_rules();
        let engine = RuleEngine::new(rules);
        let item = make_item("Article", Some("https://nytimes.com/article"), None);
        let results = engine.evaluate(&item, &FeedType::Rss);
        assert!(results.iter().any(|r| r.tag == "paywall"));
    }

    #[test]
    fn test_clickbait_regex() {
        let rules = default_rules();
        let engine = RuleEngine::new(rules);
        let item = make_item("5 reasons why Rust is amazing", None, None);
        let results = engine.evaluate(&item, &FeedType::Rss);
        assert!(results.iter().any(|r| r.tag == "clickbait"));
    }
}
