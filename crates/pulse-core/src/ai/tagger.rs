use std::sync::Arc;
use tokio::sync::mpsc;
use crate::error::TaggingError;
use crate::types::{ItemId, FeedType};
use crate::ai::rules::{RuleEngine, evaluate_low_effort};
use crate::ai::onnx::OnnxTagger;
use crate::ai::vision::VisionTagger;
use crate::feeds::enrich::is_image_url;
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
                    "Tagging queue is full (capacity {}); item skipped. Run 'pulse ai run' to retag.",
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
pub async fn tagger_task(
    mut rx: mpsc::Receiver<TagRequest>,
    db: DbHandle,
    rule_engine: Arc<RuleEngine>,
    onnx_tagger: Option<Arc<OnnxTagger>>,
    vision_tagger: Option<Arc<VisionTagger>>,
) {
    tracing::info!("Tagger task started");

    while let Some(req) = rx.recv().await {
        match process_tag_request(&db, &rule_engine, onnx_tagger.as_deref(), vision_tagger.as_deref(), &req).await {
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

pub(crate) async fn process_tag_request(
    db: &DbHandle,
    rule_engine: &RuleEngine,
    onnx_tagger: Option<&OnnxTagger>,
    vision_tagger: Option<&VisionTagger>,
    req: &TagRequest,
) -> Result<usize, TaggingError> {
    let item_id = req.item_id.clone();
    let feed_type = req.feed_type.clone();

    let item = db.with_reader(|pool| async move {
        get_item(&pool, &item_id).await
    }).await.map_err(TaggingError::Storage)?;

    // ── Vision path: image-only items bypass text pipeline ────────────────────
    // Image URLs (i.redd.it, etc.) have no body text; run CLIP instead of NLI.
    let is_image = item.url.as_deref().map(is_image_url).unwrap_or(false);
    if is_image {
        if let Some(vision) = vision_tagger {
            if let Some(ref url) = item.url {
                match vision.classify_image_url(url).await {
                    Ok(vision_tags) if !vision_tags.is_empty() => {
                        let count = vision_tags.len();
                        db.insert_ai_tags(req.item_id.clone(), vision_tags).await
                            .map_err(TaggingError::Storage)?;
                        return Ok(count);
                    }
                    Ok(_) => {
                        // No tags above threshold — that's fine, no tags inserted
                        return Ok(0);
                    }
                    Err(e) => {
                        tracing::debug!(url = %url, "Vision tagging failed: {}", e);
                        // Fall through to text pipeline (may also find nothing useful)
                    }
                }
            }
        }
    }

    // ── Text path: NLI + rule engine ──────────────────────────────────────────
    let mut tags = if let Some(onnx) = onnx_tagger {
        // ONNX path: semantic classification for content tags
        let mut onnx_tags = match onnx.classify(&item, &feed_type) {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!("ONNX classify failed, falling back to rules: {}", e);
                rule_engine.evaluate(&item, &feed_type)
            }
        };

        // Supplement with ALL rule tags — rules provide high-precision keyword matches
        // that ONNX misses (exact domain names, title prefixes, etc.)
        for rule_tag in rule_engine.evaluate(&item, &feed_type) {
            if !onnx_tags.iter().any(|t| t.tag == rule_tag.tag) {
                onnx_tags.push(rule_tag);
            }
        }

        onnx_tags
    } else {
        // Rules-only path
        rule_engine.evaluate(&item, &feed_type)
    };

    // low-effort always evaluated by compound logic regardless of model
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
