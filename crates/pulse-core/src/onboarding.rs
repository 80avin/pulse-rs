//! Curated popular-feed catalog for onboarding / discovery, and the
//! auto-grouped bulk-add path. Feeds are developer-focused and organized into
//! categories; adding a feed also creates (or reuses) its category group.

use crate::types::FeedType;

/// A single curated feed in the catalog.
pub struct PopularFeed {
    pub name: &'static str,
    pub url: &'static str,
    pub kind: FeedType,
    pub category: &'static str,
}

/// A feed the user selected during onboarding.
#[derive(Debug, Clone)]
pub struct OnboardSelection {
    pub name: String,
    pub url: String,
    pub kind: FeedType,
    pub category: String,
}

/// The curated catalog, organized by category (order = display order).
pub const POPULAR_FEEDS: &[PopularFeed] = &[
    // ── News & Communities ──
    PopularFeed { name: "Hacker News", url: "https://news.ycombinator.com", kind: FeedType::Hn, category: "News & Communities" },
    PopularFeed { name: "r/programming", url: "https://www.reddit.com/r/programming", kind: FeedType::Reddit, category: "News & Communities" },
    PopularFeed { name: "Ars Technica", url: "https://feeds.arstechnica.com/arstechnica/index", kind: FeedType::Rss, category: "News & Communities" },
    PopularFeed { name: "The Verge", url: "https://www.theverge.com/rss/index.xml", kind: FeedType::Rss, category: "News & Communities" },
    // ── Rust & Systems ──
    PopularFeed { name: "r/rust", url: "https://www.reddit.com/r/rust", kind: FeedType::Reddit, category: "Rust & Systems" },
    PopularFeed { name: "r/golang", url: "https://www.reddit.com/r/golang", kind: FeedType::Reddit, category: "Rust & Systems" },
    PopularFeed { name: "This Week in Rust", url: "https://this-week-in-rust.org/rss.xml", kind: FeedType::Rss, category: "Rust & Systems" },
    PopularFeed { name: "Julia Evans", url: "https://jvns.ca/atom.xml", kind: FeedType::Rss, category: "Rust & Systems" },
    PopularFeed { name: "r/sysadmin", url: "https://www.reddit.com/r/sysadmin", kind: FeedType::Reddit, category: "Rust & Systems" },
    PopularFeed { name: "r/linux", url: "https://www.reddit.com/r/linux", kind: FeedType::Reddit, category: "Rust & Systems" },
    // ── AI & ML ──
    PopularFeed { name: "r/LocalLLaMA", url: "https://www.reddit.com/r/LocalLLaMA", kind: FeedType::Reddit, category: "AI & ML" },
    PopularFeed { name: "r/MachineLearning", url: "https://www.reddit.com/r/MachineLearning", kind: FeedType::Reddit, category: "AI & ML" },
    // ── Security ──
    PopularFeed { name: "r/netsec", url: "https://www.reddit.com/r/netsec", kind: FeedType::Reddit, category: "Security" },
    PopularFeed { name: "Krebs on Security", url: "https://krebsonsecurity.com/feed/", kind: FeedType::Rss, category: "Security" },
    PopularFeed { name: "Schneier on Security", url: "https://www.schneier.com/feed/atom/", kind: FeedType::Rss, category: "Security" },
    // ── Privacy & Self-hosted ──
    PopularFeed { name: "r/selfhosted", url: "https://www.reddit.com/r/selfhosted", kind: FeedType::Reddit, category: "Privacy & Self-hosted" },
    PopularFeed { name: "r/PrivacyGuides", url: "https://www.reddit.com/r/PrivacyGuides", kind: FeedType::Reddit, category: "Privacy & Self-hosted" },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_has_unique_feed_urls_and_nonempty_categories() {
        let mut urls = std::collections::HashSet::new();
        for f in POPULAR_FEEDS {
            assert!(!f.url.is_empty() && !f.name.is_empty() && !f.category.is_empty());
            assert!(urls.insert(f.url), "duplicate feed URL: {}", f.url);
        }
        assert!(POPULAR_FEEDS.len() >= 15);
    }

    #[test]
    fn every_category_has_at_least_two_feeds() {
        let mut counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
        for f in POPULAR_FEEDS {
            *counts.entry(f.category).or_insert(0) += 1;
        }
        for (cat, n) in &counts {
            assert!(n >= &2, "category '{}' has only {} feed(s)", cat, n);
        }
    }
}
