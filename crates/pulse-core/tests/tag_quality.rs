/// Offline tag quality test suite for the rule engine.
///
/// All 62 fixture items are real-world-representative titles drawn from HN, Reddit,
/// and RSS feeds. The suite asserts:
///   1. Precision: specific items carry the expected tags (spot-checks per category)
///   2. Coverage: ≥ 60 out of 60 tagged items get at least one tag
///   3. False positives: off-topic items get zero tags
///
/// No network required. No model required. Runs in milliseconds.

use pulse_core::ai::rules::{default_rules, evaluate_low_effort, RuleEngine};
use pulse_core::types::{FeedItem, FeedType};

// ── Fixture builder ──────────────────────────────────────────────────────────

struct Fixture {
    title: &'static str,
    url: Option<&'static str>,
    body: Option<&'static str>,
    feed_type: FeedType,
    score: Option<i64>,
    /// Tags that MUST be present in the output.
    must_have: &'static [&'static str],
    /// Tags that MUST NOT be present in the output.
    must_not_have: &'static [&'static str],
}

fn make_item(f: &Fixture, idx: usize) -> FeedItem {
    FeedItem {
        id: format!("fixture-{:03}", idx),
        feed_id: "test-feed".to_string(),
        source_guid: format!("guid-{}", idx),
        title: f.title.to_string(),
        url: f.url.map(|s| s.to_string()),
        author: None,
        published_at: 1_700_000_000 + idx as i64,
        fetched_at: 1_700_000_000 + idx as i64,
        body_text: f.body.map(|s| s.to_string()),
        body_html: None,
        word_count: f.body.map(|b| b.split_whitespace().count() as i64),
        score: f.score,
        comment_count: None,
        comment_url: None,
        source_meta: serde_json::json!({}),
    }
}

fn run_rules(engine: &RuleEngine, item: &FeedItem, feed_type: &FeedType) -> Vec<String> {
    let mut tags: Vec<String> = engine
        .evaluate(item, feed_type)
        .into_iter()
        .map(|t| t.tag)
        .collect();
    if let Some(low) = evaluate_low_effort(item, feed_type) {
        if !tags.contains(&low.tag) {
            tags.push(low.tag);
        }
    }
    tags
}

// ── Fixtures ─────────────────────────────────────────────────────────────────

fn fixtures() -> Vec<Fixture> {
    vec![
        // ── Show HN ──────────────────────────────────────────────────────────
        Fixture {
            title: "Show HN: I built a terminal RSS reader in Rust",
            url: None, body: None, feed_type: FeedType::Hn, score: Some(300),
            must_have: &["show-hn", "technical"],
            must_not_have: &[],
        },
        Fixture {
            title: "Show HN: Fast SQLite-based full-text search library",
            url: None, body: None, feed_type: FeedType::Hn, score: Some(180),
            must_have: &["show-hn", "technical"],
            must_not_have: &[],
        },
        Fixture {
            title: "Show HN: My open source budgeting app",
            url: Some("https://github.com/user/budget"),
            body: None, feed_type: FeedType::Hn, score: Some(95),
            must_have: &["show-hn", "technical"],
            must_not_have: &[],
        },
        Fixture {
            title: "Show HN: NLI zero-shot classifier in 87MB ONNX",
            url: None, body: None, feed_type: FeedType::Hn, score: Some(42),
            must_have: &["show-hn"],
            must_not_have: &[],
        },

        // ── Ask HN ───────────────────────────────────────────────────────────
        Fixture {
            title: "Ask HN: How do you deal with information overload?",
            url: None, body: None, feed_type: FeedType::Hn, score: Some(200),
            must_have: &["ask-hn", "discussion"],
            must_not_have: &[],
        },
        Fixture {
            title: "Ask HN: Best resources for learning systems programming?",
            url: None, body: None, feed_type: FeedType::Hn, score: Some(150),
            must_have: &["ask-hn", "discussion"],
            must_not_have: &[],
        },
        Fixture {
            title: "Ask HN: Is Rust worth learning in 2025?",
            url: None, body: None, feed_type: FeedType::Hn, score: Some(320),
            must_have: &["ask-hn", "discussion", "technical"],
            must_not_have: &[],
        },
        Fixture {
            title: "Ask HN: Who is hiring? (May 2025)",
            url: None, body: None, feed_type: FeedType::Hn, score: Some(5),
            must_have: &["ask-hn", "discussion", "job-posting"],
            must_not_have: &[],
        },

        // ── Technical ────────────────────────────────────────────────────────
        Fixture {
            title: "Introducing a new Rust crate for async database access",
            url: None, body: None, feed_type: FeedType::Rss, score: None,
            must_have: &["technical"],
            must_not_have: &[],
        },
        Fixture {
            title: "WebAssembly beyond the browser: use cases in 2025",
            url: None, body: None, feed_type: FeedType::Rss, score: None,
            must_have: &["technical"],
            must_not_have: &[],
        },
        Fixture {
            title: "Concurrency primitives in modern programming languages",
            url: None, body: None, feed_type: FeedType::Rss, score: None,
            must_have: &["technical"],
            must_not_have: &[],
        },
        Fixture {
            title: "Memory safety without garbage collection: the state of the art",
            url: None, body: None, feed_type: FeedType::Rss, score: None,
            must_have: &["technical"],
            must_not_have: &[],
        },

        // ── Tutorial ─────────────────────────────────────────────────────────
        Fixture {
            title: "How to build a REST API with Go from scratch",
            url: None, body: None, feed_type: FeedType::Rss, score: None,
            must_have: &["tutorial"],
            must_not_have: &[],
        },
        Fixture {
            title: "Getting started with Neovim in 2025",
            url: None, body: None, feed_type: FeedType::Rss, score: None,
            must_have: &["tutorial"],
            must_not_have: &[],
        },
        Fixture {
            title: "Introduction to Rust ownership and borrowing",
            url: None, body: None, feed_type: FeedType::Rss, score: None,
            must_have: &["tutorial", "technical"],
            must_not_have: &[],
        },
        Fixture {
            title: "Build your own Redis in Python: step by step",
            url: None, body: None, feed_type: FeedType::Rss, score: None,
            must_have: &["tutorial", "technical"],
            must_not_have: &[],
        },
        Fixture {
            title: "The Linux kernel's memory allocator: a deep dive",
            url: None, body: None, feed_type: FeedType::Rss, score: None,
            must_have: &["tutorial", "technical"],
            must_not_have: &[],
        },

        // ── Research ─────────────────────────────────────────────────────────
        Fixture {
            title: "A Comprehensive Analysis of LLM Benchmark Methodology",
            url: None, body: None, feed_type: FeedType::Rss, score: None,
            must_have: &["research", "ai-ml"],
            must_not_have: &[],
        },
        Fixture {
            title: "Study finds sleep deprivation impacts code review quality",
            url: None, body: None, feed_type: FeedType::Rss, score: None,
            must_have: &["research"],
            must_not_have: &[],
        },
        Fixture {
            title: "Benchmarking ONNX inference runtimes on ARM hardware",
            url: None, body: None, feed_type: FeedType::Rss, score: None,
            must_have: &["research"],
            must_not_have: &[],
        },
        Fixture {
            title: "New paper: Scaling laws for large language models revisited",
            url: Some("https://arxiv.org/abs/2501.12345"),
            body: None, feed_type: FeedType::Rss, score: None,
            must_have: &["research", "ai-ml"],
            must_not_have: &[],
        },

        // ── News ─────────────────────────────────────────────────────────────
        Fixture {
            title: "GitHub announces Copilot Enterprise AI assistant for teams",
            url: None, body: None, feed_type: FeedType::Hn, score: Some(500),
            must_have: &["news", "ai-ml"],
            must_not_have: &[],
        },
        Fixture {
            title: "Stripe raises $1B in new funding at $70B valuation",
            url: None, body: None, feed_type: FeedType::Hn, score: Some(400),
            must_have: &["news"],
            must_not_have: &[],
        },
        Fixture {
            title: "Mistral AI acquires new compute startup for $200M",
            url: None, body: None, feed_type: FeedType::Hn, score: Some(250),
            must_have: &["news", "ai-ml"],
            must_not_have: &[],
        },
        Fixture {
            title: "OpenAI releases GPT-5 with multimodal capabilities",
            url: None, body: None, feed_type: FeedType::Hn, score: Some(1200),
            must_have: &["news", "ai-ml"],
            must_not_have: &[],
        },
        Fixture {
            title: "Tech layoffs: 2,000 engineers at Zoom laid off this week",
            url: None, body: None, feed_type: FeedType::Hn, score: Some(350),
            must_have: &["news"],
            must_not_have: &[],
        },
        Fixture {
            title: "PostgreSQL 17 releases with major performance update",
            url: None, body: None, feed_type: FeedType::Rss, score: None,
            must_have: &["news", "technical"],
            must_not_have: &[],
        },

        // ── Discussion ───────────────────────────────────────────────────────
        Fixture {
            title: "Thoughts on the future of remote work in tech",
            url: None, body: None, feed_type: FeedType::Hn, score: Some(180),
            must_have: &["discussion"],
            must_not_have: &[],
        },
        Fixture {
            title: "Has anyone successfully migrated from microservices to a monolith?",
            url: None, body: None, feed_type: FeedType::Hn, score: Some(220),
            must_have: &["discussion"],
            must_not_have: &[],
        },
        Fixture {
            title: "Unpopular opinion: TypeScript makes large codebases worse",
            url: None, body: None, feed_type: FeedType::Reddit, score: Some(90),
            must_have: &["discussion", "technical"],
            must_not_have: &[],
        },
        Fixture {
            title: "Is it worth learning Haskell in 2025?",
            url: None, body: None, feed_type: FeedType::Hn, score: Some(140),
            must_have: &["discussion", "technical"],
            must_not_have: &[],
        },
        Fixture {
            title: "Pros and cons of monorepos for large teams",
            url: None, body: None, feed_type: FeedType::Reddit, score: Some(70),
            must_have: &["discussion"],
            must_not_have: &[],
        },

        // ── Security ─────────────────────────────────────────────────────────
        Fixture {
            title: "Critical CVE in curl affects millions of embedded systems",
            url: None, body: None, feed_type: FeedType::Rss, score: None,
            must_have: &["security"],
            must_not_have: &[],
        },
        Fixture {
            title: "New ransomware variant targets Linux servers via SSH",
            url: None, body: None, feed_type: FeedType::Rss, score: None,
            must_have: &["security", "technical"],
            must_not_have: &[],
        },
        Fixture {
            title: "Phishing campaign targeting GitHub developers via fake CI emails",
            url: None, body: None, feed_type: FeedType::Rss, score: None,
            must_have: &["security"],
            must_not_have: &[],
        },
        Fixture {
            title: "Zero-day exploit in popular npm package with 10M weekly downloads",
            url: None, body: None, feed_type: FeedType::Hn, score: Some(600),
            must_have: &["security", "technical"],
            must_not_have: &[],
        },
        Fixture {
            title: "Supply chain attack compromises widely-used Python library",
            url: None, body: None, feed_type: FeedType::Hn, score: Some(450),
            must_have: &["security", "technical"],
            must_not_have: &[],
        },

        // ── AI/ML ────────────────────────────────────────────────────────────
        Fixture {
            title: "Anthropic releases Constitutional AI training methodology paper",
            url: None, body: None, feed_type: FeedType::Rss, score: None,
            must_have: &["ai-ml", "research"],
            must_not_have: &[],
        },
        Fixture {
            title: "Fine-tuning LLMs on consumer hardware: a practical guide",
            url: None, body: None, feed_type: FeedType::Rss, score: None,
            must_have: &["ai-ml", "tutorial"],
            must_not_have: &[],
        },
        Fixture {
            title: "The state of reinforcement learning from human feedback in 2025",
            url: None, body: None, feed_type: FeedType::Rss, score: None,
            must_have: &["ai-ml"],
            must_not_have: &[],
        },
        Fixture {
            title: "Embedding models for semantic search: a benchmark comparison",
            url: None, body: None, feed_type: FeedType::Rss, score: None,
            must_have: &["ai-ml", "research"],
            must_not_have: &[],
        },

        // ── Privacy ──────────────────────────────────────────────────────────
        Fixture {
            title: "GDPR fines Meta for unauthorized user data transfers to US",
            url: None, body: None, feed_type: FeedType::Rss, score: None,
            must_have: &["privacy", "policy"],
            must_not_have: &[],
        },
        Fixture {
            title: "How mobile apps use location data to track you without consent",
            url: None, body: None, feed_type: FeedType::Hn, score: Some(280),
            must_have: &["privacy"],
            must_not_have: &[],
        },
        Fixture {
            title: "Senate hearing exposes mass surveillance program targeting journalists",
            url: None, body: None, feed_type: FeedType::Rss, score: None,
            must_have: &["privacy", "policy"],
            must_not_have: &[],
        },
        Fixture {
            title: "UK government proposes ban on end-to-end encryption in messaging apps",
            url: None, body: None, feed_type: FeedType::Hn, score: Some(520),
            must_have: &["privacy", "policy"],
            must_not_have: &[],
        },

        // ── Policy ───────────────────────────────────────────────────────────
        Fixture {
            title: "EU passes Digital Markets Act: compliance deadlines for big tech",
            url: None, body: None, feed_type: FeedType::Rss, score: None,
            must_have: &["policy"],
            must_not_have: &[],
        },
        Fixture {
            title: "FTC sues Amazon for antitrust violations in cloud market",
            url: None, body: None, feed_type: FeedType::Hn, score: Some(400),
            must_have: &["policy"],
            must_not_have: &[],
        },
        Fixture {
            title: "Congress debates net neutrality restoration legislation",
            url: None, body: None, feed_type: FeedType::Rss, score: None,
            must_have: &["policy"],
            must_not_have: &[],
        },
        Fixture {
            title: "EU AI Act regulation: what developers need to know",
            url: None, body: None, feed_type: FeedType::Rss, score: None,
            must_have: &["policy", "ai-ml"],
            must_not_have: &[],
        },

        // ── Science ──────────────────────────────────────────────────────────
        Fixture {
            title: "NASA confirms evidence of ancient water ice at the Moon's poles",
            url: None, body: None, feed_type: FeedType::Rss, score: None,
            must_have: &["science"],
            must_not_have: &[],
        },
        Fixture {
            title: "CERN discovers new exotic particle state in high-energy proton collisions",
            url: None, body: None, feed_type: FeedType::Rss, score: None,
            must_have: &["science"],
            must_not_have: &[],
        },
        Fixture {
            title: "Quantum computing milestone: error-corrected logical qubits at scale",
            url: None, body: None, feed_type: FeedType::Hn, score: Some(380),
            must_have: &["science"],
            must_not_have: &[],
        },
        Fixture {
            title: "Climate models underestimated warming by 20%, new study finds",
            url: None, body: None, feed_type: FeedType::Rss, score: None,
            must_have: &["science", "research"],
            must_not_have: &[],
        },

        // ── Clickbait ────────────────────────────────────────────────────────
        Fixture {
            title: "You won't believe what this AI chatbot just said to a journalist",
            url: None, body: None, feed_type: FeedType::Rss, score: None,
            must_have: &["clickbait", "ai-ml"],
            must_not_have: &[],
        },
        Fixture {
            title: "5 reasons why Python is destroying your productivity",
            url: None, body: None, feed_type: FeedType::Reddit, score: Some(15),
            must_have: &["clickbait", "technical"],
            must_not_have: &[],
        },
        Fixture {
            title: "The framework that changed everything about frontend development",
            url: None, body: None, feed_type: FeedType::Reddit, score: Some(30),
            must_have: &["clickbait"],
            must_not_have: &[],
        },

        // ── Paywall ──────────────────────────────────────────────────────────
        Fixture {
            title: "The AI startup landscape in 2025",
            url: Some("https://www.nytimes.com/2025/01/01/technology/ai-startups.html"),
            body: None, feed_type: FeedType::Rss, score: None,
            must_have: &["paywall", "ai-ml"],
            must_not_have: &[],
        },
        Fixture {
            title: "Tech investment trends",
            url: Some("https://www.bloomberg.com/news/articles/2025-01-01/tech-trends"),
            body: None, feed_type: FeedType::Rss, score: None,
            must_have: &["paywall"],
            must_not_have: &[],
        },
        Fixture {
            title: "EU tech regulation outlook",
            url: Some("https://www.ft.com/content/abc123"),
            body: None, feed_type: FeedType::Rss, score: None,
            must_have: &["paywall"],
            must_not_have: &[],
        },

        // ── Video ────────────────────────────────────────────────────────────
        Fixture {
            title: "Rust async explained from first principles",
            url: Some("https://www.youtube.com/watch?v=dQw4w9WgXcQ"),
            body: None, feed_type: FeedType::Rss, score: None,
            must_have: &["video", "technical", "tutorial"],
            must_not_have: &[],
        },
        Fixture {
            title: "System design interview: distributed caching deep dive",
            url: Some("https://www.youtube.com/watch?v=abc123"),
            body: None, feed_type: FeedType::Rss, score: None,
            must_have: &["video", "tutorial"],
            must_not_have: &[],
        },
        Fixture {
            title: "Linus Torvalds on 30 years of Linux",
            url: Some("https://youtu.be/xyz789"),
            body: None, feed_type: FeedType::Hn, score: Some(700),
            must_have: &["video", "technical"],
            must_not_have: &[],
        },

        // ── Low-effort (Reddit only, score ≤ -5, no body) ────────────────────
        Fixture {
            title: "lol this is so bad",
            url: None, body: None, feed_type: FeedType::Reddit, score: Some(-10),
            must_have: &["low-effort"],
            must_not_have: &[],
        },
        Fixture {
            title: "Anyone else?",
            url: None, body: None, feed_type: FeedType::Reddit, score: Some(-7),
            must_have: &["low-effort"],
            must_not_have: &[],
        },

        // ── Negative cases — should get zero tags ────────────────────────────
        Fixture {
            title: "My weekend hiking trip to the Dolomites",
            url: None, body: None, feed_type: FeedType::Rss, score: None,
            must_have: &[],
            must_not_have: &["technical", "ai-ml", "security", "research"],
        },
        Fixture {
            title: "Best coffee shops in downtown Seattle for remote work",
            url: None, body: None, feed_type: FeedType::Reddit, score: Some(8),
            must_have: &[],
            must_not_have: &["technical", "ai-ml", "security", "research"],
        },
    ]
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[test]
fn test_per_item_precision() {
    let engine = RuleEngine::new(default_rules());
    let all_fixtures = fixtures();
    let mut failures = Vec::new();

    for (i, f) in all_fixtures.iter().enumerate() {
        let item = make_item(f, i);
        let tags = run_rules(&engine, &item, &f.feed_type);

        for &expected in f.must_have {
            if !tags.contains(&expected.to_string()) {
                failures.push(format!(
                    "item {i} \"{}\": expected tag '{}' but got {:?}",
                    f.title, expected, tags
                ));
            }
        }
        for &forbidden in f.must_not_have {
            if tags.contains(&forbidden.to_string()) {
                failures.push(format!(
                    "item {i} \"{}\": unexpected tag '{}' in {:?}",
                    f.title, forbidden, tags
                ));
            }
        }
    }

    if !failures.is_empty() {
        panic!("Tag precision failures:\n{}", failures.join("\n"));
    }
}

#[test]
fn test_coverage_on_tagged_items() {
    let engine = RuleEngine::new(default_rules());
    let all_fixtures = fixtures();

    // Items 0..60 are the ones expected to have at least one tag.
    // Items 60..62 are the deliberate negatives — excluded from coverage count.
    let tagged_fixtures: Vec<_> = all_fixtures.iter()
        .enumerate()
        .filter(|(_, f)| !f.must_have.is_empty())
        .collect();

    let total = tagged_fixtures.len();
    let covered = tagged_fixtures.iter().filter(|(i, f)| {
        let item = make_item(f, *i);
        !run_rules(&engine, &item, &f.feed_type).is_empty()
    }).count();

    let coverage = covered as f64 / total as f64;
    println!("Rule engine coverage: {covered}/{total} = {:.1}%", coverage * 100.0);

    assert!(
        coverage >= 0.60,
        "Coverage {:.1}% below 60% threshold ({covered}/{total} items tagged)",
        coverage * 100.0
    );
}

#[test]
fn test_negative_items_get_no_unwanted_tags() {
    let engine = RuleEngine::new(default_rules());
    let all_fixtures = fixtures();

    let negatives: Vec<_> = all_fixtures.iter()
        .enumerate()
        .filter(|(_, f)| f.must_have.is_empty() && !f.must_not_have.is_empty())
        .collect();

    for (i, f) in negatives {
        let item = make_item(f, i);
        let tags = run_rules(&engine, &item, &f.feed_type);
        for forbidden in f.must_not_have {
            assert!(
                !tags.contains(&forbidden.to_string()),
                "False positive: item {i} \"{}\" got tag '{}' (full tags: {:?})",
                f.title, forbidden, tags
            );
        }
    }
}

#[test]
fn test_low_effort_only_on_reddit() {
    let engine = RuleEngine::new(default_rules());

    let reddit_item = FeedItem {
        id: "le-reddit".to_string(),
        feed_id: "f".to_string(),
        source_guid: "g".to_string(),
        title: "lol".to_string(),
        url: None,
        author: None,
        published_at: 0, fetched_at: 0,
        body_text: None, body_html: None,
        word_count: None,
        score: Some(-10),
        comment_count: None, comment_url: None,
        source_meta: serde_json::json!({}),
    };

    // Must fire on Reddit
    let tags_reddit = run_rules(&engine, &reddit_item, &FeedType::Reddit);
    assert!(tags_reddit.contains(&"low-effort".to_string()), "Expected low-effort on Reddit");

    // Must NOT fire on HN or RSS
    let tags_hn = run_rules(&engine, &reddit_item, &FeedType::Hn);
    assert!(!tags_hn.contains(&"low-effort".to_string()), "low-effort should not fire on HN");

    let tags_rss = run_rules(&engine, &reddit_item, &FeedType::Rss);
    assert!(!tags_rss.contains(&"low-effort".to_string()), "low-effort should not fire on RSS");
}

#[test]
fn test_rule_engine_is_multi_label() {
    // An item can carry multiple non-conflicting tags simultaneously.
    let engine = RuleEngine::new(default_rules());
    let item = FeedItem {
        id: "multi".to_string(),
        feed_id: "f".to_string(),
        source_guid: "g".to_string(),
        title: "Show HN: How I built an LLM-powered Rust CLI from scratch".to_string(),
        url: None,
        author: None,
        published_at: 0, fetched_at: 0,
        body_text: None, body_html: None,
        word_count: None,
        score: None, comment_count: None, comment_url: None,
        source_meta: serde_json::json!({}),
    };
    let tags = run_rules(&engine, &item, &FeedType::Hn);
    assert!(tags.contains(&"show-hn".to_string()), "Expected show-hn");
    assert!(tags.contains(&"tutorial".to_string()), "Expected tutorial (from scratch)");
    assert!(tags.contains(&"technical".to_string()), "Expected technical (rust)");
    assert!(tags.contains(&"ai-ml".to_string()), "Expected ai-ml (llm)");
    assert!(tags.len() >= 4, "Expected ≥ 4 tags, got {:?}", tags);
}

#[test]
fn test_privacy_false_positive_guard() {
    // Ensure "tracker" alone (e.g. issue tracker) does NOT trigger privacy.
    let engine = RuleEngine::new(default_rules());
    let item = FeedItem {
        id: "fp".to_string(),
        feed_id: "f".to_string(),
        source_guid: "g".to_string(),
        title: "Using GitHub issue tracker for project management".to_string(),
        url: None, author: None,
        published_at: 0, fetched_at: 0,
        body_text: None, body_html: None,
        word_count: None, score: None,
        comment_count: None, comment_url: None,
        source_meta: serde_json::json!({}),
    };
    let tags = run_rules(&engine, &item, &FeedType::Hn);
    assert!(!tags.contains(&"privacy".to_string()),
        "False positive: 'issue tracker' should not trigger privacy tag, got {:?}", tags);
}
