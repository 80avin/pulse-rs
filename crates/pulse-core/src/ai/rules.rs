use crate::types::{FeedItem, FeedType, TagResult, TaggerSource};
use regex::Regex;

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
            RulePattern::Keyword(kw) => text.to_lowercase().contains(kw.as_str()),
            RulePattern::Regex(re) => re.is_match(text),
            RulePattern::DomainMatch(domain) => {
                item.url
                    .as_deref()
                    .map(|u| {
                        let lower = u.to_lowercase();
                        // Check ://domain (bare) or .domain (subdomain), avoiding substring false positives
                        lower.contains(&format!("://{}", domain))
                            || lower.contains(&format!(".{}", domain))
                    })
                    .unwrap_or(false)
            }
            RulePattern::HasScore { min } => item.score.map(|s| s >= *min).unwrap_or(false),
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
            RulePattern::Regex(re) => re.find(text).map(|m| m.as_str()),
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
            let all_match = self
                .patterns
                .iter()
                .all(|p| p.matches(item, &text_for_matching, feed_type));
            if all_match {
                self.patterns
                    .first()
                    .and_then(|p| p.matched_text(&text_for_matching))
                    .unwrap_or("all conditions")
                    .to_string()
            } else {
                return None;
            }
        } else {
            // OR: find first matching pattern
            let first_match = self.patterns.iter().find_map(|p| {
                if p.matches(item, &text_for_matching, feed_type) {
                    Some(
                        p.matched_text(&text_for_matching)
                            .unwrap_or("condition")
                            .to_string(),
                    )
                } else {
                    None
                }
            });

            match first_match {
                Some(m) => m,
                None => return None,
            }
        };

        let explanation = self
            .explanation_template
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
                RulePattern::Regex(Regex::new(r"(?i)\brust\b").unwrap()),
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
                RulePattern::Keyword("algorithm".to_string()),
                RulePattern::Keyword("sql".to_string()),
                RulePattern::Keyword("database".to_string()),
                RulePattern::Keyword("open source".to_string()),
                RulePattern::Keyword("concurrency".to_string()),
                RulePattern::Keyword("package manager".to_string()),
                RulePattern::Keyword("debugging".to_string()),
                RulePattern::Keyword("refactor".to_string()),
                RulePattern::Keyword("memory safety".to_string()),
                RulePattern::Keyword("type system".to_string()),
                // Rust-specific / systems programming
                RulePattern::Regex(Regex::new(r"(?i)\bstruct\b").unwrap()),
                RulePattern::Keyword(" trait".to_string()),
                RulePattern::Keyword(" macro".to_string()),
                RulePattern::Keyword(" crate".to_string()),
                RulePattern::Keyword(" async".to_string()),
                RulePattern::Regex(Regex::new(r"(?i)\borm\b").unwrap()),
                RulePattern::Keyword("ownership".to_string()),
                RulePattern::Regex(Regex::new(r"(?i)\bborrow\b").unwrap()),
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
                // Title-only: prevents body-text "benchmark"/"experiment" false positives
                // (software perf testing and casual experimentation both use these words)
                RulePattern::Keyword("arxiv.org".to_string()),
                RulePattern::Regex(Regex::new(r"(?i)\barxiv\b").unwrap()),
                RulePattern::Regex(Regex::new(r"(?i)\bstudy\b").unwrap()),
                RulePattern::Keyword("research".to_string()),
                RulePattern::Keyword("findings".to_string()),
                RulePattern::Keyword("dataset".to_string()),
                RulePattern::Keyword("methodology".to_string()),
                RulePattern::Keyword("peer review".to_string()),
                RulePattern::Keyword("hypothesis".to_string()),
                RulePattern::Keyword("benchmark".to_string()),
                RulePattern::Keyword("experiment".to_string()),
                RulePattern::DomainMatch("arxiv.org".to_string()),
                RulePattern::DomainMatch("semanticscholar.org".to_string()),
                RulePattern::DomainMatch("scholar.google.com".to_string()),
            ],
            scope: RuleScope::TitleOnly,
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
                // Local/regional news patterns common in Indian subreddits
                RulePattern::Keyword("inauguration".to_string()),
                RulePattern::Keyword("inaugurated".to_string()),
                RulePattern::Keyword("renamed".to_string()),
                RulePattern::Keyword("arrested".to_string()),
                RulePattern::Keyword("seized".to_string()),
                RulePattern::Keyword("warns".to_string()),
                RulePattern::Keyword("approved for".to_string()),
                RulePattern::Keyword("deployed".to_string()),
            ],
            scope: RuleScope::TitleOnly,
            require_all: false,
            enabled: true,
        },

        TagRule {
            id: "civic".to_string(),
            tag: "civic".to_string(),
            confidence: 0.85,
            explanation_template: "matched civic/infrastructure keyword '{matched_text}'".to_string(),
            patterns: vec![
                // Power/utility failures (English)
                RulePattern::Keyword("electricity".to_string()),
                RulePattern::Keyword("power cut".to_string()),
                RulePattern::Keyword("power outage".to_string()),
                RulePattern::Keyword("load shedding".to_string()),
                RulePattern::Keyword("jkpdd".to_string()),
                RulePattern::Keyword("jkpowerco".to_string()),
                RulePattern::Keyword("light nhi".to_string()),
                RulePattern::Keyword("light nahi".to_string()),
                // batti/bijli + complaint word — avoids "bijli mahadev trek" false positive
                RulePattern::Regex(Regex::new(r"(?i)(batti|bijli).{0,20}(nhi|nahi|kab|kyu)").unwrap()),
                RulePattern::Regex(Regex::new(r"(?i)no\s+electricity").unwrap()),
                // Municipal / civic bodies
                RulePattern::Keyword("municipal".to_string()),
                RulePattern::Keyword("jmc ".to_string()),
                RulePattern::Keyword("civic sense".to_string()),
                RulePattern::Keyword("civic body".to_string()),
                RulePattern::Keyword("smart city".to_string()),
                // Water, roads, sanitation
                RulePattern::Keyword("water supply".to_string()),
                RulePattern::Keyword("no water".to_string()),
                RulePattern::Keyword("pani nahi".to_string()),
                RulePattern::Keyword("sewage".to_string()),
                RulePattern::Keyword("drainage".to_string()),
                RulePattern::Keyword("pothole".to_string()),
                RulePattern::Keyword("road condition".to_string()),
                // Telecom
                RulePattern::Keyword("bsnl".to_string()),
                RulePattern::Keyword("jio fiber".to_string()),
            ],
            scope: RuleScope::TitleOnly,
            require_all: false,
            enabled: true,
        },

        TagRule {
            id: "local-rec".to_string(),
            tag: "local-rec".to_string(),
            confidence: 0.82,
            explanation_template: "local recommendation request matching '{matched_text}'".to_string(),
            patterns: vec![
                // Health — specific enough that service type alone implies recommendation context
                RulePattern::Keyword("therapist for".to_string()),
                RulePattern::Keyword("good therapist".to_string()),
                RulePattern::Keyword("good dermatologist".to_string()),
                RulePattern::Keyword("best dermatologist".to_string()),
                RulePattern::Keyword("good dentist".to_string()),
                RulePattern::Keyword("best dentist".to_string()),
                RulePattern::Keyword("good physician".to_string()),
                RulePattern::Keyword("best doctor".to_string()),
                RulePattern::Keyword("good doctor".to_string()),
                RulePattern::Keyword("urologist".to_string()),
                RulePattern::Keyword("neurologist".to_string()),
                RulePattern::Keyword("gynecologist".to_string()),
                RulePattern::Keyword("psychiatrist".to_string()),
                // Food & restaurants
                RulePattern::Keyword("best momos".to_string()),
                RulePattern::Keyword("good momos".to_string()),
                RulePattern::Keyword("best dhaba".to_string()),
                RulePattern::Keyword("best dhabha".to_string()),
                RulePattern::Keyword("suggest best dhabha".to_string()),
                RulePattern::Keyword("suggest best dhaba".to_string()),
                RulePattern::Keyword("best restaurant".to_string()),
                RulePattern::Keyword("good restaurant".to_string()),
                RulePattern::Keyword("cabin cafe".to_string()),
                RulePattern::Keyword("good cafe".to_string()),
                RulePattern::Keyword("best cafe".to_string()),
                RulePattern::Keyword("halal spot".to_string()),
                RulePattern::Keyword("veg spot".to_string()),
                // Fitness
                RulePattern::Keyword("good gym".to_string()),
                RulePattern::Keyword("best gym".to_string()),
                RulePattern::Keyword("suggest good gym".to_string()),
                RulePattern::Keyword("suggest good gyms".to_string()),
                RulePattern::Keyword("gym fees".to_string()),
                // Education (specific local codes and institutions, not generic "college")
                RulePattern::Keyword("gcet ".to_string()),
                RulePattern::Keyword("smvdu".to_string()),
                RulePattern::Keyword("mbs college".to_string()),
                RulePattern::Keyword("pmsss college".to_string()),
                RulePattern::Keyword("colleges in pmsss".to_string()),
                RulePattern::Keyword("college in pmsss".to_string()),
                RulePattern::Keyword("good colleges through pmsss".to_string()),
                RulePattern::Keyword("best coaching".to_string()),
                RulePattern::Keyword("good coaching".to_string()),
                RulePattern::Keyword("jkssb classes".to_string()),
                RulePattern::Keyword("jkssb coaching".to_string()),
                // Professionals and accommodation
                RulePattern::Keyword("good lawyer".to_string()),
                RulePattern::Keyword("good hotel".to_string()),
                RulePattern::Keyword("review hotel".to_string()),
                // Generic city-agnostic patterns: "[service] in [City]" and "best/good X in [City]"
                RulePattern::Regex(Regex::new(r"(?i)\b(best|good|any\s+good|recommend|suggest)\b.{0,40}\b(in|near)\s+[A-Za-z]{3,}\b").unwrap()),
                RulePattern::Regex(Regex::new(r"(?i)\b(therapist|lawyer|advocate|hostel|coaching|dentist|dermatologist)\b.{0,30}\bin\s+[A-Za-z]{3,}\b").unwrap()),
                RulePattern::Regex(Regex::new(r"(?i)\bhow\s+is\b.{0,30}\b(jkssb|gcet|smvdu|pmsss)\b").unwrap()),
                RulePattern::Regex(Regex::new(r"(?i)\bcentral university\b").unwrap()),
            ],
            scope: RuleScope::TitleOnly,
            require_all: false,
            enabled: true,
        },

        TagRule {
            id: "culture".to_string(),
            tag: "culture".to_string(),
            confidence: 0.85,
            explanation_template: "matched cultural/heritage keyword '{matched_text}'".to_string(),
            patterns: vec![
                // Language and ethnic identity
                RulePattern::Regex(Regex::new(r"(?i)\bdogr[ia]\b").unwrap()),
                RulePattern::Keyword("pahari culture".to_string()),
                RulePattern::Keyword("gojri".to_string()),
                // Heritage and traditions
                RulePattern::Keyword("folk tradition".to_string()),
                RulePattern::Keyword("folk culture".to_string()),
                RulePattern::Keyword("local heritage".to_string()),
                RulePattern::Keyword("dogra heritage".to_string()),
                RulePattern::Keyword("dogra dynasty".to_string()),
                RulePattern::Keyword("dogra kingdom".to_string()),
                RulePattern::Keyword("dogra sadar".to_string()),
                // Historical figures (Maharaja is distinctive enough in J&K context)
                RulePattern::Regex(Regex::new(r"(?i)\bmaharaj(a)?\b").unwrap()),
                // Local cultural landmarks and food traditions
                RulePattern::Keyword("bahu fort".to_string()),
                RulePattern::Keyword("anchali".to_string()),
                RulePattern::Keyword("rasonth".to_string()),
                RulePattern::Keyword("kalari".to_string()),
                RulePattern::Keyword("kaladi".to_string()),
                RulePattern::Keyword("dogri film".to_string()),
                RulePattern::Keyword("dogri poem".to_string()),
                RulePattern::Keyword("dogri song".to_string()),
                RulePattern::Keyword("dogri music".to_string()),
                RulePattern::Keyword("cultural identity".to_string()),
                RulePattern::Keyword("local tradition".to_string()),
            ],
            scope: RuleScope::TitleOnly,
            require_all: false,
            enabled: true,
        },

        TagRule {
            id: "marketplace".to_string(),
            tag: "marketplace".to_string(),
            confidence: 0.90,
            explanation_template: "matched marketplace keyword '{matched_text}'".to_string(),
            patterns: vec![
                // Selling
                RulePattern::Regex(Regex::new(r"(?i)\bsell(ing)?\b").unwrap()),
                RulePattern::Keyword("for sale".to_string()),
                RulePattern::Keyword(" wts ".to_string()),
                RulePattern::Keyword(" wtb ".to_string()),
                RulePattern::Keyword(" wtt ".to_string()),
                RulePattern::Keyword("looking to sell".to_string()),
                RulePattern::Keyword("anyone selling".to_string()),
                // Buying
                RulePattern::Keyword("looking to buy".to_string()),
                RulePattern::Keyword("anyone buying".to_string()),
                // Rentals
                RulePattern::Keyword("room for rent".to_string()),
                RulePattern::Keyword("flat for rent".to_string()),
                RulePattern::Keyword("for rent".to_string()),
                RulePattern::Keyword("rental".to_string()),
                RulePattern::Keyword("room available".to_string()),
                RulePattern::Keyword("pg available".to_string()),
                RulePattern::Keyword("accommodation available".to_string()),
                // Domestic staff listings
                RulePattern::Keyword("cook needed".to_string()),
                RulePattern::Keyword("maid needed".to_string()),
                RulePattern::Keyword("care giver".to_string()),
                RulePattern::Keyword("caregiver needed".to_string()),
                RulePattern::Keyword("driver needed".to_string()),
                RulePattern::Keyword("looking for cook".to_string()),
                RulePattern::Keyword("need a cook".to_string()),
                // Job listings (complements the `job-posting` rule for corporate/HN context)
                RulePattern::Keyword("vacancy".to_string()),
                RulePattern::Keyword("hiring for".to_string()),
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
                RulePattern::Keyword("data leak".to_string()),
                RulePattern::Keyword("authentication".to_string()),
                RulePattern::Keyword("authorization".to_string()),
                RulePattern::Keyword("injection attack".to_string()),
                RulePattern::Keyword("xss".to_string()),
                RulePattern::Keyword("csrf".to_string()),
                RulePattern::Keyword("backdoor".to_string()),
                RulePattern::Keyword("pwned".to_string()),
                RulePattern::Keyword("remote code execution".to_string()),
                RulePattern::Keyword("privilege escalation".to_string()),
                RulePattern::Keyword("buffer overflow".to_string()),
                RulePattern::Keyword("memory corruption".to_string()),
                RulePattern::Regex(Regex::new(r"(?i)\bcve\b").unwrap()),
                RulePattern::Regex(Regex::new(r"(?i)\brce\b").unwrap()),
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
                RulePattern::Keyword("mistral".to_string()),
                RulePattern::Keyword("copilot".to_string()),
                RulePattern::Keyword("chatgpt".to_string()),
                RulePattern::Keyword("transformer".to_string()),
                RulePattern::Keyword("diffusion model".to_string()),
                RulePattern::Keyword("stable diffusion".to_string()),
                RulePattern::Keyword("midjourney".to_string()),
                RulePattern::Keyword("reinforcement learning".to_string()),
                RulePattern::Keyword("fine-tuning".to_string()),
                RulePattern::Keyword("prompt engineering".to_string()),
                RulePattern::Keyword("embedding model".to_string()),
                RulePattern::Keyword("generative ai".to_string()),
                RulePattern::Keyword("chatbot".to_string()),
                RulePattern::Keyword("hugging face".to_string()),
                RulePattern::Keyword("onnx".to_string()),
                RulePattern::Keyword("sigmoid".to_string()),
                RulePattern::Keyword("attention mechanism".to_string()),
                RulePattern::Keyword("backpropagation".to_string()),
                RulePattern::Keyword("tokenization".to_string()),
                RulePattern::DomainMatch("huggingface.co".to_string()),
            ],
            scope: RuleScope::All,
            require_all: false,
            enabled: true,
        },

        // \bai\b alone is too noisy in body text; restrict to titles where "AI" signals topic intent
        TagRule {
            id: "ai-ml-title".to_string(),
            tag: "ai-ml".to_string(),
            confidence: 0.85,
            explanation_template: "matched AI keyword 'AI' in title".to_string(),
            patterns: vec![
                RulePattern::Regex(Regex::new(r"(?i)\bai\b").unwrap()),
            ],
            scope: RuleScope::TitleOnly,
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
                RulePattern::Keyword("mullvad".to_string()),
                RulePattern::Keyword("protonmail".to_string()),
                RulePattern::Keyword("proton vpn".to_string()),
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
                RulePattern::Regex(Regex::new(r"(?i)\bbill\b.{0,50}(pass|sign|veto|introduc|advanc|clear)").unwrap()),
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
                RulePattern::Regex(Regex::new(r"(?i)\bcern\b").unwrap()),
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

        // ── Quality tags ───────────────────────────────────────────────────────
        // These catch posts whose content is definitively identifiable as low-value.
        // Suppression logic in process_tag_request ensures `no-context` is removed
        // if a substantive topic tag (civic, local-rec, technical, etc.) also fires.

        TagRule {
            id: "no-context".to_string(),
            tag: "no-context".to_string(),
            confidence: 0.82,
            explanation_template: "matched vague help-seeking pattern '{matched_text}'".to_string(),
            patterns: vec![
                // Hindi / Hinglish vague question phrases (very distinctive)
                RulePattern::Keyword("kya karu".to_string()),
                RulePattern::Keyword("kya karun".to_string()),
                RulePattern::Keyword("kya karoon".to_string()),
                RulePattern::Keyword("kya karna chahiye".to_string()),
                RulePattern::Keyword("kya karna chahie".to_string()),
                RulePattern::Keyword("batao yaar".to_string()),
                RulePattern::Keyword("bata do yaar".to_string()),
                RulePattern::Keyword("koi bata sakta hai".to_string()),
                RulePattern::Keyword("koi bata do".to_string()),
                RulePattern::Keyword("help chahiye".to_string()),
                RulePattern::Keyword("help karo".to_string()),
                RulePattern::Keyword("help karo please".to_string()),
                RulePattern::Keyword("madad chahiye".to_string()),
                RulePattern::Keyword("suggestion do".to_string()),
                RulePattern::Keyword("suggestion chahiye".to_string()),
                RulePattern::Keyword("suggest karo".to_string()),
                RulePattern::Keyword("kuch suggest karo".to_string()),
                RulePattern::Keyword("advice chahiye".to_string()),
                RulePattern::Keyword("guidance chahiye".to_string()),
                RulePattern::Keyword("confused hun".to_string()),
                RulePattern::Keyword("samajh nahi aa raha".to_string()),
                RulePattern::Keyword("samajh nahi aaya".to_string()),
                // Standalone English generic help patterns (no specificity)
                RulePattern::Regex(Regex::new(r"(?i)^help[\s!?]*$").unwrap()),
                RulePattern::Regex(Regex::new(r"(?i)^please help[\s!?]*$").unwrap()),
                RulePattern::Regex(Regex::new(r"(?i)^(need|urgent)\s+help[\s!?]*$").unwrap()),
                RulePattern::Regex(Regex::new(r"(?i)^any\s+(ideas?|suggestions?|thoughts?|advice|help)[\s!?]*$").unwrap()),
                RulePattern::Regex(Regex::new(r"(?i)^what\s+(should\s+i|do\s+i)\s+do[\s!?]*$").unwrap()),
                RulePattern::Regex(Regex::new(r"(?i)^(thoughts?|opinions?|suggestions?)[\s!?]*$").unwrap()),
                RulePattern::Regex(Regex::new(r"(?i)^i\s+(am|m)\s+(confused|lost|stuck|clueless)[\s!?]*$").unwrap()),
                RulePattern::Regex(Regex::new(r"(?i)^(really\s+)?need\s+(some\s+)?(advice|guidance|help|suggestions?)[\s!?]*$").unwrap()),
            ],
            scope: RuleScope::TitleOnly,
            require_all: false,
            enabled: true,
        },

        TagRule {
            id: "inappropriate".to_string(),
            tag: "inappropriate".to_string(),
            confidence: 0.92,
            explanation_template: "matched inappropriate/solicitation pattern '{matched_text}'".to_string(),
            patterns: vec![
                // Couples accommodation seeking (common euphemism)
                RulePattern::Keyword("hotel for couple".to_string()),
                RulePattern::Keyword("hotel for couples".to_string()),
                RulePattern::Keyword("room for couple".to_string()),
                RulePattern::Keyword("room for couples".to_string()),
                RulePattern::Keyword("couple friendly hotel".to_string()),
                RulePattern::Keyword("couple friendly pg".to_string()),
                RulePattern::Keyword("unmarried couple".to_string()),
                RulePattern::Keyword("pg for couple".to_string()),
                RulePattern::Keyword("flat for couple".to_string()),
                RulePattern::Keyword("couples room".to_string()),
                RulePattern::Keyword("couples stay".to_string()),
                // Dating / relationship seeking
                RulePattern::Keyword("looking for girlfriend".to_string()),
                RulePattern::Keyword("need a girlfriend".to_string()),
                RulePattern::Keyword("want a girlfriend".to_string()),
                RulePattern::Keyword("looking for boyfriend".to_string()),
                RulePattern::Keyword("gf chahiye".to_string()),
                RulePattern::Keyword("bf chahiye".to_string()),
                RulePattern::Keyword("ladki chahiye".to_string()),
                RulePattern::Keyword("girl chahiye".to_string()),
                RulePattern::Keyword("friendship with girl".to_string()),
                RulePattern::Keyword("friendship with girls".to_string()),
                RulePattern::Keyword("girl for friendship".to_string()),
                RulePattern::Keyword("girls dm me".to_string()),
                RulePattern::Keyword("girls dm".to_string()),
                RulePattern::Keyword("any girls here".to_string()),
                RulePattern::Keyword("online dating".to_string()),
                RulePattern::Keyword("dating app".to_string()),
                // Explicit hookup seeking
                RulePattern::Keyword("hookup".to_string()),
                RulePattern::Keyword("hook up".to_string()),
                RulePattern::Keyword("one night stand".to_string()),
                RulePattern::Keyword(" fwb ".to_string()),
                RulePattern::Keyword(" nsa ".to_string()),
                RulePattern::Regex(Regex::new(r"(?i)\bnsa\s+(fun|meet|chat)\b").unwrap()),
                RulePattern::Regex(Regex::new(r"(?i)\bfwb\b").unwrap()),
            ],
            scope: RuleScope::TitleOnly,
            require_all: false,
            enabled: true,
        },

        TagRule {
            id: "noise".to_string(),
            tag: "noise".to_string(),
            confidence: 0.78,
            explanation_template: "matched personal noise pattern '{matched_text}'".to_string(),
            patterns: vec![
                // First-person personal achievements / updates (text-only noise)
                RulePattern::Regex(Regex::new(r"(?i)^(finally|just)\s+(had|ate|tried|got|did|made|finished|completed|reached|bought)\b").unwrap()),
                RulePattern::Regex(Regex::new(r"(?i)^look\s+at\s+my\b").unwrap()),
                RulePattern::Regex(Regex::new(r"(?i)^rate\s+my\b").unwrap()),
                RulePattern::Regex(Regex::new(r"(?i)^check\s+out\s+my\b").unwrap()),
                RulePattern::Regex(Regex::new(r"(?i)^my\s+(gym|workout|progress|gains|setup|room|bike|car|cat|dog|pet)\b").unwrap()),
                // Greetings / mood posts with no content
                RulePattern::Regex(Regex::new(r"(?i)^good\s+(morning|evening|night)\s+\w+[\s!]*$").unwrap()),
                RulePattern::Regex(Regex::new(r"(?i)^(weekend|sunday|monday|friday)\s+(vibes?|mood|feels?)[\s!]*$").unwrap()),
                RulePattern::Regex(Regex::new(r"(?i)^(feeling|today\s+is)\s+(amazing|great|blessed|happy|sad|bored)[\s!]*$").unwrap()),
                // Food sharing without a question
                RulePattern::Regex(Regex::new(r"(?i)^(beer|pizza|biryani|chai|coffee|tea|momos?)\s+(time|night|vibes?)[\s!]*$").unwrap()),
                // Random photo captions
                RulePattern::Regex(Regex::new(r"(?i)^(random\s+)?(click|shot|photo|pic)\s+(of\s+the\s+day|today)[\s!]*$").unwrap()),
            ],
            scope: RuleScope::TitleOnly,
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
    let has_short_body = item
        .body_text
        .as_deref()
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
        let item = make_item(
            "Critical vulnerability in OpenSSL allows remote code execution",
            None,
            None,
        );
        let results = engine.evaluate(&item, &FeedType::Rss);
        assert!(results.iter().any(|r| r.tag == "security"));
    }
}
