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
