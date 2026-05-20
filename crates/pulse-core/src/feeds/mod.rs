pub mod detect;
pub mod enrich;
pub mod hackernews;
pub mod normalize;
pub mod reddit;
pub mod reddit_auth;
pub mod rss;

pub use detect::{detect_feed_url, FeedCandidate, FeedLink};
pub use enrich::{fetch_enrichment, is_image_url, should_enrich, EnrichmentResult};
pub use hackernews::fetch_hn;
pub use reddit::fetch_reddit;
pub use reddit_auth::RedditAuth;
pub use rss::fetch_rss;
