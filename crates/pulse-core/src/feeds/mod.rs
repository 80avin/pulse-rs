pub mod hackernews;
pub mod normalize;
pub mod reddit;
pub mod rss;

pub use hackernews::fetch_hn;
pub use reddit::fetch_reddit;
pub use rss::fetch_rss;
