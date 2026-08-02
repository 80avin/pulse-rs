use crate::types::Feed;

const MAX_BACKOFF_SECS: u64 = 14400; // 4 hours

/// Compute the next fetch timestamp for a feed based on its health state.
/// Applies exponential backoff with ±10% jitter on failure streaks.
pub fn compute_next_fetch(feed: &Feed) -> i64 {
    let now = chrono::Utc::now().timestamp();
    let base = feed.poll_interval_secs as u64;
    let streak = feed.failure_streak.max(0) as u32;

    let interval_secs = if streak == 0 {
        base
    } else {
        // Exponential backoff: base * 2^streak, capped at MAX_BACKOFF_SECS
        let raw = base.saturating_mul(1u64 << streak.min(20));
        raw.min(MAX_BACKOFF_SECS)
    };

    // Apply ±10% jitter using a simple deterministic approach
    let jitter_factor = jitter();
    let jittered = (interval_secs as f64 * jitter_factor) as i64;

    now + jittered
}

/// Simple pseudo-jitter in range [0.9, 1.1] based on current time
fn jitter() -> f64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);

    // Map nanos to [0.9, 1.1]
    0.9 + (nanos % 1000) as f64 / 5000.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Feed;

    fn feed_with(streak: u32) -> Feed {
        let now = chrono::Utc::now().timestamp();
        Feed {
            id: "f".into(),
            url: "https://example.com".into(),
            feed_type: crate::types::FeedType::Rss,
            title: Some("t".into()),
            description: None,
            site_url: None,
            icon_url: None,
            group_id: None,
            poll_interval_secs: 60,
            is_enabled: true,
            etag: None,
            last_modified: None,
            last_fetched_at: None,
            last_success_at: None,
            last_item_at: None,
            failure_streak: streak as i64,
            total_fetches: 0,
            total_failures: 0,
            avg_latency_ms: None,
            next_fetch_at: None,
            source_config: serde_json::json!({}),
            language: None,
            hue: None,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn backoff_grows_with_failure_streak() {
        let now = chrono::Utc::now().timestamp();
        let healthy = compute_next_fetch(&feed_with(0));
        let f1 = compute_next_fetch(&feed_with(1));
        let f3 = compute_next_fetch(&feed_with(3));
        // Healthy feeds schedule roughly poll_interval (60s) out.
        assert!((healthy - now) < 120);
        // Backoff must push the retry strictly further out than a fresh feed.
        assert!(f1 > healthy);
        assert!(f3 > f1);
    }

    #[test]
    fn backoff_is_capped_at_four_hours() {
        let now = chrono::Utc::now().timestamp();
        let far = compute_next_fetch(&feed_with(20));
        assert!((far - now) <= 14400 + 1500, "backoff exceeded 4h cap: {}", far - now);
    }
}
