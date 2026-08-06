use crate::ai::rules::RuleEngine;
use crate::error::TaggingError;
use crate::feeds::enrich::is_image_url;
use crate::storage::DbHandle;
use crate::storage::queries::get_item;
use crate::types::{FeedItem, FeedType, ItemId, TagResult};
use std::sync::Arc;
use tokio::sync::mpsc;

/// The bounded channel capacity for the tagging queue
pub const TAGGER_QUEUE_SIZE: usize = 200;

/// Request to tag a specific item
pub struct TagRequest {
    pub item_id: ItemId,
    pub feed_type: FeedType,
}

/// A cloneable sender handle for queuing items to be tagged
#[derive(Clone)]
pub struct TaggerHandle {
    tx: mpsc::Sender<TagRequest>,
}

impl TaggerHandle {
    pub fn new(tx: mpsc::Sender<TagRequest>) -> Self {
        Self { tx }
    }

    /// Queue an item for tagging. If the channel is full, the item is dropped (non-fatal).
    pub async fn tag_item(&self, item_id: ItemId, feed_type: FeedType) {
        let req = TagRequest {
            item_id: item_id.clone(),
            feed_type,
        };
        match self.tx.try_send(req) {
            Ok(()) => {}
            Err(mpsc::error::TrySendError::Full(_)) => {
                tracing::warn!(
                    item_id = %item_id,
                    "Tagging queue is full (capacity {}); item skipped.",
                    TAGGER_QUEUE_SIZE
                );
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                tracing::error!(item_id = %item_id, "Tagger task has exited; item will not be tagged");
            }
        }
    }
}

/// A deterministic tagger for an item. The rule engine is the built-in
/// implementation; a future BYO hosted-model adapter can implement this trait.
pub trait Tagger: Send + Sync {
    fn tag(&self, item: &FeedItem, feed_type: &FeedType) -> Vec<TagResult>;
}

/// The deterministic, offline tagger: structural rules + the low-effort check.
pub struct RulesTagger {
    engine: Arc<RuleEngine>,
}

impl RulesTagger {
    pub fn new(engine: Arc<RuleEngine>) -> Self {
        Self { engine }
    }
}

impl Tagger for RulesTagger {
    fn tag(&self, item: &FeedItem, feed_type: &FeedType) -> Vec<TagResult> {
        let mut tags = self.engine.evaluate(item, feed_type);
        // low-effort requires runtime score — not expressible as a plain TagRule pattern
        if let Some(le) = crate::ai::rules::evaluate_low_effort(item, feed_type) {
            tags.push(le);
        }
        tags
    }
}

pub async fn tagger_task(mut rx: mpsc::Receiver<TagRequest>, db: DbHandle, tagger: Arc<dyn Tagger>) {
    tracing::info!("Tagger task started");

    while let Some(req) = rx.recv().await {
        match process_tag_request(&db, tagger.as_ref(), &req).await {
            Ok(tag_count) => {
                tracing::debug!(item_id = %req.item_id, tags = tag_count, "Item tagged");
            }
            Err(e) => {
                tracing::warn!(item_id = %req.item_id, error = %e, "Tagging failed (non-fatal)");
            }
        }
    }

    tracing::info!("Tagger task shutting down");
}

/// A substantive semantic match makes `no-context` inappropriate (specific ≠ vague).
const SUBSTANTIVE_TAGS: &[&str] = &[
    "technical",
    "tutorial",
    "research",
    "news",
    "security",
    "ai-ml",
    "privacy",
    "policy",
    "science",
    "clickbait",
    "show-hn",
    "ask-hn",
    "job-posting",
    "paywall",
    "video",
    "civic",
    "local-rec",
    "culture",
    "marketplace",
];

pub(crate) async fn process_tag_request(
    db: &DbHandle,
    tagger: &dyn Tagger,
    req: &TagRequest,
) -> Result<usize, TaggingError> {
    let item_id = req.item_id.clone();

    let item = db
        .with_reader(|pool| async move { get_item(&pool, &item_id).await })
        .await
        .map_err(TaggingError::Storage)?;

    // Direct image URLs (i.redd.it, imgur, …) have no text for structural rules
    let is_direct_image = item.url.as_deref().map(is_image_url).unwrap_or(false);

    let mut tags: Vec<TagResult> = if is_direct_image {
        vec![]
    } else {
        tagger.tag(&item, &req.feed_type)
    };

    // A specific question (local-rec, civic, …) is not vague even if short
    let has_substantive = tags
        .iter()
        .any(|t| SUBSTANTIVE_TAGS.contains(&t.tag.as_str()));
    if has_substantive {
        tags.retain(|t| t.tag != "no-context");
    }

    // High-confidence `noise` strips semantic topic tags — a personal food post
    // cannot also be technical/security/policy/ai-ml content.
    const NOISE_SUPPRESSED_TAGS: &[&str] = &[
        "technical",
        "security",
        "ai-ml",
        "policy",
        "privacy",
        "science",
        "research",
        "tutorial",
    ];
    let noise_conf = tags
        .iter()
        .find(|t| t.tag == "noise")
        .map(|t| t.confidence)
        .unwrap_or(0.0);
    if noise_conf >= 0.70 {
        tags.retain(|t| !NOISE_SUPPRESSED_TAGS.contains(&t.tag.as_str()));
    }

    let tag_count = tags.len();
    if !tags.is_empty() {
        db.insert_ai_tags(req.item_id.clone(), tags)
            .await
            .map_err(TaggingError::Storage)?;
    }

    Ok(tag_count)
}
