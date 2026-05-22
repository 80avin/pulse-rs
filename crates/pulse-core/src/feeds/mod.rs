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
