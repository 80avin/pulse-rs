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
        // ── Core content-type tags ─────────────────────────────────────────────

        TagRule {
            id: "technical".to_string(),
            tag: "technical".to_string(),
            confidence: 0.80,
            explanation_template: "matched keyword '{matched_text}' in content".to_string(),
            patterns: vec![
                // languages
                RulePattern::Keyword("rust".to_string()),
                RulePattern::Keyword("python".to_string()),
                RulePattern::Keyword("typescript".to_string()),
                RulePattern::Keyword("javascript".to_string()),
                RulePattern::Keyword("golang".to_string()),
                RulePattern::Keyword("erlang".to_string()),
                RulePattern::Keyword("elixir".to_string()),
                RulePattern::Keyword("haskell".to_string()),
                RulePattern::Keyword("zig ".to_string()),
                RulePattern::Keyword("c++".to_string()),
                RulePattern::Keyword("java ".to_string()),
                RulePattern::Keyword("swift ".to_string()),
                RulePattern::Keyword("kotlin".to_string()),
                RulePattern::Keyword("ocaml".to_string()),
                // infrastructure / tooling
                RulePattern::Keyword("docker".to_string()),
                RulePattern::Keyword("kubernetes".to_string()),
                RulePattern::Keyword("linux".to_string()),
                RulePattern::Keyword("git ".to_string()),
                RulePattern::Keyword("llvm".to_string()),
                RulePattern::Keyword("wasm".to_string()),
                RulePattern::Keyword("webassembly".to_string()),
                RulePattern::Keyword("compiler".to_string()),
                RulePattern::Keyword("interpreter".to_string()),
                RulePattern::Keyword("runtime".to_string()),
                RulePattern::Keyword("cpu ".to_string()),
                RulePattern::Keyword("gpu ".to_string()),
                // software engineering concepts
                RulePattern::Keyword(" api ".to_string()),
                RulePattern::Keyword("framework".to_string()),
                RulePattern::Keyword("library".to_string()),
                RulePattern::Keyword("algorithm".to_string()),
                RulePattern::Keyword("performance".to_string()),
                RulePattern::Keyword("sql".to_string()),
                RulePattern::Keyword("database".to_string()),
                RulePattern::Keyword("open source".to_string()),
                RulePattern::Keyword("concurrency".to_string()),
                RulePattern::Keyword("package manager".to_string()),
                RulePattern::Keyword("terminal".to_string()),
                RulePattern::Keyword("debugging".to_string()),
                RulePattern::Keyword("refactor".to_string()),
                RulePattern::Keyword("memory safety".to_string()),
                RulePattern::Keyword("type system".to_string()),
                // Rust-specific / systems programming
                RulePattern::Keyword("struct".to_string()),
                RulePattern::Keyword(" trait".to_string()),
                RulePattern::Keyword(" macro".to_string()),
                RulePattern::Keyword(" crate".to_string()),
                RulePattern::Keyword(" async".to_string()),
                RulePattern::Keyword("orm".to_string()),
                RulePattern::Keyword("ownership".to_string()),
                RulePattern::Keyword("borrow".to_string()),
                RulePattern::Keyword("-rs".to_string()),   // catches image-rs, tokio-rs, etc.
                RulePattern::Keyword("mpsc".to_string()),
                RulePattern::Keyword("tokio".to_string()),
                RulePattern::Keyword("serde".to_string()),
                RulePattern::Keyword("axum".to_string()),
                RulePattern::Keyword("actix".to_string()),
                RulePattern::Keyword("rayon".to_string()),
                // graphics / game dev
                RulePattern::Keyword("shader".to_string()),
                RulePattern::Keyword("rendering".to_string()),
                RulePattern::Keyword("opengl".to_string()),
                RulePattern::Keyword("vulkan".to_string()),
                RulePattern::Keyword("webgl".to_string()),
                RulePattern::Keyword("blending".to_string()),
                // registries / hosting
                RulePattern::Keyword("github.com".to_string()),
                RulePattern::Keyword("crates.io".to_string()),
                RulePattern::Keyword("npm".to_string()),
                RulePattern::DomainMatch("github.com".to_string()),
                RulePattern::DomainMatch("crates.io".to_string()),
                RulePattern::DomainMatch("gitlab.com".to_string()),
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
                RulePattern::Keyword("how i built".to_string()),
                RulePattern::Keyword("how i wrote".to_string()),
                RulePattern::Keyword("from scratch".to_string()),
                RulePattern::Keyword("deep dive".to_string()),
                RulePattern::Keyword("by example".to_string()),
                RulePattern::Keyword("in practice".to_string()),
                RulePattern::Keyword("made simple".to_string()),
                RulePattern::Keyword("under the hood".to_string()),
                RulePattern::Keyword("explained".to_string()),
                RulePattern::Keyword("understanding".to_string()),
                RulePattern::Regex(Regex::new(r"(?i)write your own ").unwrap()),
                RulePattern::Regex(Regex::new(r"(?i)build (a|an|your) ").unwrap()),
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
                RulePattern::Keyword("peer review".to_string()),
                RulePattern::Keyword("experiment".to_string()),
                RulePattern::Keyword("hypothesis".to_string()),
                RulePattern::Keyword("analysis".to_string()),
                RulePattern::DomainMatch("arxiv.org".to_string()),
                RulePattern::DomainMatch("semanticscholar.org".to_string()),
                RulePattern::DomainMatch("scholar.google.com".to_string()),
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
                RulePattern::Keyword("acquisition".to_string()),
                RulePattern::Keyword("raises".to_string()),
                RulePattern::Keyword("funding".to_string()),
                RulePattern::Keyword("partnership".to_string()),
                RulePattern::Keyword("breach".to_string()),
                RulePattern::Keyword("outage".to_string()),
                RulePattern::Keyword("shuts down".to_string()),
                RulePattern::Keyword("taken offline".to_string()),
                RulePattern::Keyword("has been hacked".to_string()),
                RulePattern::Keyword("data center".to_string()),
                RulePattern::Keyword("layoffs".to_string()),
                RulePattern::Keyword("laid off".to_string()),
                RulePattern::Keyword("fired".to_string()),
                RulePattern::Keyword("ipo".to_string()),
                RulePattern::Keyword("merger".to_string()),
                RulePattern::Keyword("update".to_string()),
                RulePattern::Regex(Regex::new(r"(?i)\$\d+[bm]\b").unwrap()),
                RulePattern::Regex(Regex::new(r"(?i)\d+[bm] (in |deal|funding|valuation)").unwrap()),
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
                RulePattern::Keyword("what's your take".to_string()),
                RulePattern::Keyword("opinion".to_string()),
                RulePattern::Keyword("i believe".to_string()),
                RulePattern::Keyword("has anyone".to_string()),
                RulePattern::Keyword("should we".to_string()),
                RulePattern::Keyword("is it worth".to_string()),
                RulePattern::Keyword("pros and cons".to_string()),
                RulePattern::Keyword("unpopular opinion".to_string()),
                RulePattern::Keyword("change my mind".to_string()),
            ],
            scope: RuleScope::TitleOnly,
            require_all: false,
            enabled: true,
        },

        // ── Topic tags ─────────────────────────────────────────────────────────

        TagRule {
            id: "security".to_string(),
            tag: "security".to_string(),
            confidence: 0.85,
            explanation_template: "matched security keyword '{matched_text}'".to_string(),
            patterns: vec![
                RulePattern::Keyword("vulnerability".to_string()),
                RulePattern::Keyword("exploit".to_string()),
                RulePattern::Keyword("cve-".to_string()),
                RulePattern::Keyword("zero-day".to_string()),
                RulePattern::Keyword("ransomware".to_string()),
                RulePattern::Keyword("malware".to_string()),
                RulePattern::Keyword("phishing".to_string()),
                RulePattern::Keyword("supply chain attack".to_string()),
                RulePattern::Keyword("penetration test".to_string()),
                RulePattern::Keyword("cryptography".to_string()),
                RulePattern::Keyword("encryption".to_string()),
                RulePattern::Keyword("cybersecurity".to_string()),
                RulePattern::Keyword("infosec".to_string()),
                RulePattern::Keyword("threat actor".to_string()),
                RulePattern::Keyword("data breach".to_string()),
                RulePattern::Keyword("authentication".to_string()),
                RulePattern::Keyword("authorization".to_string()),
                RulePattern::Keyword("injection attack".to_string()),
                RulePattern::Keyword("xss".to_string()),
                RulePattern::Keyword("csrf".to_string()),
                RulePattern::Regex(Regex::new(r"(?i)\bcve\b").unwrap()),
            ],
            scope: RuleScope::All,
            require_all: false,
            enabled: true,
        },

        TagRule {
            id: "ai-ml".to_string(),
            tag: "ai-ml".to_string(),
            confidence: 0.85,
            explanation_template: "matched AI/ML keyword '{matched_text}'".to_string(),
            patterns: vec![
                RulePattern::Keyword("machine learning".to_string()),
                RulePattern::Keyword("deep learning".to_string()),
                RulePattern::Keyword("neural network".to_string()),
                RulePattern::Keyword("large language model".to_string()),
                RulePattern::Keyword("llm".to_string()),
                RulePattern::Keyword("gpt".to_string()),
                RulePattern::Keyword("openai".to_string()),
                RulePattern::Keyword("anthropic".to_string()),
                RulePattern::Keyword("gemini".to_string()),
                RulePattern::Keyword("claude".to_string()),
                RulePattern::Keyword("transformer".to_string()),
                RulePattern::Keyword("diffusion model".to_string()),
                RulePattern::Keyword("stable diffusion".to_string()),
                RulePattern::Keyword("midjourney".to_string()),
                RulePattern::Keyword("reinforcement learning".to_string()),
                RulePattern::Keyword("fine-tuning".to_string()),
                RulePattern::Keyword("prompt engineering".to_string()),
                RulePattern::Keyword("inference".to_string()),
                RulePattern::Keyword("embedding".to_string()),
                RulePattern::Keyword("generative ai".to_string()),
                RulePattern::Keyword("chatbot".to_string()),
                RulePattern::Keyword("hugging face".to_string()),
                RulePattern::Keyword("sigmoid".to_string()),
                RulePattern::Keyword("attention mechanism".to_string()),
                RulePattern::Keyword("backpropagation".to_string()),
                RulePattern::Keyword("tokenization".to_string()),
                RulePattern::Regex(Regex::new(r"(?i)\bai\b").unwrap()),
                RulePattern::DomainMatch("huggingface.co".to_string()),
            ],
            scope: RuleScope::All,
            require_all: false,
            enabled: true,
        },

        TagRule {
            id: "privacy".to_string(),
            tag: "privacy".to_string(),
            confidence: 0.85,
            explanation_template: "matched privacy keyword '{matched_text}'".to_string(),
            patterns: vec![
                RulePattern::Keyword("surveillance".to_string()),
                RulePattern::Keyword("facial recognition".to_string()),
                RulePattern::Keyword("user tracking".to_string()),
                RulePattern::Keyword("cross-site tracking".to_string()),
                RulePattern::Keyword("gdpr".to_string()),
                RulePattern::Keyword("ccpa".to_string()),
                RulePattern::Keyword("biometric".to_string()),
                RulePattern::Keyword("data collection".to_string()),
                RulePattern::Keyword("mass surveillance".to_string()),
                RulePattern::Keyword("palantir".to_string()),
                RulePattern::Keyword("location data".to_string()),
                RulePattern::Keyword("browser fingerprint".to_string()),
                RulePattern::Keyword("private browsing".to_string()),
                RulePattern::Keyword("vpn".to_string()),
                RulePattern::Keyword("tor browser".to_string()),
                RulePattern::Keyword("end-to-end encryption".to_string()),
                RulePattern::Keyword("anonymity".to_string()),
            ],
            scope: RuleScope::All,
            require_all: false,
            enabled: true,
        },

        TagRule {
            id: "policy".to_string(),
            tag: "policy".to_string(),
            confidence: 0.80,
            explanation_template: "matched policy keyword '{matched_text}'".to_string(),
            patterns: vec![
                RulePattern::Keyword("legislation".to_string()),
                RulePattern::Keyword("regulation".to_string()),
                RulePattern::Keyword("antitrust".to_string()),
                RulePattern::Keyword("ftc".to_string()),
                RulePattern::Keyword("doj ".to_string()),
                RulePattern::Keyword("congress".to_string()),
                RulePattern::Keyword("senate".to_string()),
                RulePattern::Keyword("eu commission".to_string()),
                RulePattern::Keyword("digital markets act".to_string()),
                RulePattern::Keyword("gdpr".to_string()),
                RulePattern::Keyword("net neutrality".to_string()),
                RulePattern::Keyword("copyright".to_string()),
                RulePattern::Keyword("patent".to_string()),
                RulePattern::Keyword("open source license".to_string()),
                RulePattern::Keyword("ban on".to_string()),
                RulePattern::Regex(Regex::new(r"(?i)\bbill\b.{0,30}(pass|sign|veto|introduc)").unwrap()),
            ],
            scope: RuleScope::All,
            require_all: false,
            enabled: true,
        },

        TagRule {
            id: "science".to_string(),
            tag: "science".to_string(),
            confidence: 0.80,
            explanation_template: "matched science keyword '{matched_text}'".to_string(),
            patterns: vec![
                RulePattern::Keyword("quantum".to_string()),
                RulePattern::Keyword("physics".to_string()),
                RulePattern::Keyword("chemistry".to_string()),
                RulePattern::Keyword("biology".to_string()),
                RulePattern::Keyword("astronomy".to_string()),
                RulePattern::Keyword("neuroscience".to_string()),
                RulePattern::Keyword("genomics".to_string()),
                RulePattern::Keyword("crispr".to_string()),
                RulePattern::Keyword("space exploration".to_string()),
                RulePattern::Keyword("climate change".to_string()),
                RulePattern::Keyword("climate model".to_string()),
                RulePattern::Keyword("particle physics".to_string()),
                RulePattern::Keyword("superconductor".to_string()),
                RulePattern::Keyword("semiconductor".to_string()),
                RulePattern::Keyword("cern".to_string()),
                RulePattern::Keyword("nasa".to_string()),
                RulePattern::Keyword("esa ".to_string()),
                RulePattern::Keyword("quasicrystal".to_string()),
                RulePattern::Keyword("crystallography".to_string()),
                RulePattern::Keyword("materials science".to_string()),
                RulePattern::DomainMatch("nature.com".to_string()),
                RulePattern::DomainMatch("science.org".to_string()),
                RulePattern::DomainMatch("newscientist.com".to_string()),
            ],
            scope: RuleScope::All,
            require_all: false,
            enabled: true,
        },

        // ── Quality / signal tags ──────────────────────────────────────────────

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

        // ── Platform/format tags ───────────────────────────────────────────────

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
                RulePattern::Keyword("job board".to_string()),
                RulePattern::Regex(Regex::new(r"(?i)\bis hiring\b").unwrap()),
                RulePattern::Keyword("freelance".to_string()),
            ],
            scope: RuleScope::All,
            require_all: false,
            enabled: true,
        },

        TagRule {
            id: "show-hn".to_string(),
            tag: "show-hn".to_string(),
            confidence: 0.99,
            explanation_template: "Show HN post detected".to_string(),
            patterns: vec![
                RulePattern::Keyword("show hn:".to_string()),
            ],
            scope: RuleScope::TitleOnly,
            require_all: false,
            enabled: true,
        },

        TagRule {
            id: "ask-hn".to_string(),
            tag: "ask-hn".to_string(),
            confidence: 0.99,
            explanation_template: "Ask HN post detected".to_string(),
            patterns: vec![
                RulePattern::Keyword("ask hn:".to_string()),
            ],
            scope: RuleScope::TitleOnly,
            require_all: false,
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
                RulePattern::DomainMatch("fortune.com".to_string()),
                RulePattern::DomainMatch("thenational.scot".to_string()),
                RulePattern::DomainMatch("telegraph.co.uk".to_string()),
                RulePattern::DomainMatch("washingtonpost.com".to_string()),
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
                // Title signals when post links to an external page about a video
                RulePattern::Keyword("on youtube".to_string()),
                RulePattern::Keyword("on twitch".to_string()),
                RulePattern::Keyword("on vimeo".to_string()),
                RulePattern::Keyword("live on youtube".to_string()),
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

    #[test]
    fn test_show_hn_title_only_no_feedtype_required() {
        let rules = default_rules();
        let engine = RuleEngine::new(rules);
        // Works for any feed type
        let item = make_item("Show HN: My new project", None, None);
        let results = engine.evaluate(&item, &FeedType::Rss);
        assert!(results.iter().any(|r| r.tag == "show-hn"));
        let results_hn = engine.evaluate(&item, &FeedType::Hn);
        assert!(results_hn.iter().any(|r| r.tag == "show-hn"));
    }

    #[test]
    fn test_ai_ml_rule() {
        let rules = default_rules();
        let engine = RuleEngine::new(rules);
        let item = make_item("OpenAI releases new LLM with better reasoning", None, None);
        let results = engine.evaluate(&item, &FeedType::Rss);
        assert!(results.iter().any(|r| r.tag == "ai-ml"));
    }

    #[test]
    fn test_security_rule() {
        let rules = default_rules();
        let engine = RuleEngine::new(rules);
        let item = make_item("Critical vulnerability in OpenSSL allows remote code execution", None, None);
        let results = engine.evaluate(&item, &FeedType::Rss);
        assert!(results.iter().any(|r| r.tag == "security"));
    }
}
