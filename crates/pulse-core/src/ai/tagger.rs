use std::sync::Arc;
use tokio::sync::mpsc;
use crate::error::TaggingError;
use crate::types::{ItemId, FeedType};
use crate::ai::rules::{RuleEngine, evaluate_low_effort};
use crate::storage::DbHandle;
use crate::storage::queries::get_item;

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
        let req = TagRequest { item_id: item_id.clone(), feed_type };
        match self.tx.try_send(req) {
            Ok(()) => {}
            Err(mpsc::error::TrySendError::Full(_)) => {
                tracing::warn!(
                    item_id = %item_id,
                    "Tagging queue is full (capacity {}); item skipped. Run 'pulse ai retag --pending' to re-queue.",
                    TAGGER_QUEUE_SIZE
                );
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                tracing::error!(item_id = %item_id, "Tagger task has exited; item will not be tagged");
            }
        }
    }
}

/// The tagging background task.
/// Runs a rule engine against incoming item IDs and stores the results via DbHandle.
pub async fn tagger_task(
    mut rx: mpsc::Receiver<TagRequest>,
    db: DbHandle,
    rule_engine: Arc<RuleEngine>,
) {
    tracing::info!("Tagger task started");

    while let Some(req) = rx.recv().await {
        match process_tag_request(&db, &rule_engine, &req).await {
            Ok(tag_count) => {
                tracing::debug!(
                    item_id = %req.item_id,
                    tags = tag_count,
                    "Item tagged successfully"
                );
            }
            Err(TaggingError::ModelNotLoaded) => {
                tracing::error!(
                    "AI model not available; disabling tagging. \
                     Run 'pulse ai model download' to configure a model."
                );
                break;
            }
            Err(e) => {
                tracing::warn!(
                    item_id = %req.item_id,
                    error = %e,
                    "Tagging failed (non-fatal)"
                );
            }
        }
    }

    tracing::info!("Tagger task shutting down");
}

pub(crate) async fn process_tag_request(
    db: &DbHandle,
    rule_engine: &RuleEngine,
    req: &TagRequest,
) -> Result<usize, TaggingError> {
    let item_id = req.item_id.clone();
    let feed_type = req.feed_type.clone();

    // Fetch the item from the database using the reader pool
    let item = db.with_reader(|pool| async move {
        get_item(&pool, &item_id).await
    }).await.map_err(TaggingError::Storage)?;

    // Evaluate rules
    let mut tags = rule_engine.evaluate(&item, &feed_type);

    // Special-case: low-effort (requires compound condition)
    if let Some(low_effort_tag) = evaluate_low_effort(&item, &feed_type) {
        if !tags.iter().any(|t| t.tag == "low-effort") {
            tags.push(low_effort_tag);
        }
    }

    let tag_count = tags.len();

    if !tags.is_empty() {
        db.insert_ai_tags(req.item_id.clone(), tags).await
            .map_err(TaggingError::Storage)?;
    }

    Ok(tag_count)
}
