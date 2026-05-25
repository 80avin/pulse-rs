use crate::ai::rules::RuleEngine;
use crate::ai::vision::VisionTagger;
use crate::error::TaggingError;
use crate::feeds::enrich::is_image_url;
use crate::storage::DbHandle;
use crate::storage::queries::get_item;
use crate::types::{FeedType, ItemId, TagResult};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
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

/// The tagging background task. Receives shared RwLock handles so hot-reloaded models
/// are picked up immediately on the next queued item.
pub async fn tagger_task(
    mut rx: mpsc::Receiver<TagRequest>,
    db: DbHandle,
    text_backend: crate::config::TextBackend,
    rule_engine: Arc<RuleEngine>,
    fasttext_tagger: Arc<RwLock<Option<Arc<crate::ai::fasttext::FastTextTagger>>>>,
    miniml_tagger: Arc<RwLock<Option<Arc<crate::ai::miniml::MiniMlTagger>>>>,
    vision_tagger: Arc<RwLock<Option<Arc<crate::ai::vision::VisionTagger>>>>,
) {
    tracing::info!("Tagger task started");

    while let Some(req) = rx.recv().await {
        // Snapshot current tagger Arcs (hold lock only briefly).
        let fasttext = fasttext_tagger.read().unwrap().clone();
        let miniml = miniml_tagger.read().unwrap().clone();
        let vision = vision_tagger.read().unwrap().clone();
        match process_tag_request(
            &db,
            &text_backend,
            &rule_engine,
            fasttext.as_deref(),
            miniml.as_deref(),
            vision.as_deref(),
            &req,
        )
        .await
        {
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

/// Tags where a substantive semantic match means `no-context` should be suppressed.
/// If any of these fire, the post has enough content signal to not be vague.
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
    text_backend: &crate::config::TextBackend,
    rule_engine: &RuleEngine,
    fasttext: Option<&crate::ai::fasttext::FastTextTagger>,
    miniml: Option<&crate::ai::miniml::MiniMlTagger>,
    vision: Option<&VisionTagger>,
    req: &TagRequest,
) -> Result<usize, TaggingError> {
    let item_id = req.item_id.clone();

    let item = db
        .with_reader(|pool| async move { get_item(&pool, &item_id).await })
        .await
        .map_err(TaggingError::Storage)?;

    let is_direct_image = item.url.as_deref().map(is_image_url).unwrap_or(false);

    // ── Rules: always run first (deterministic, high-confidence) ─────────────
    let rule_tags: Vec<TagResult> = if !is_direct_image {
        let mut tags = rule_engine.evaluate(&item, &req.feed_type);
        // low-effort requires runtime score — not expressible as a plain TagRule pattern
        if let Some(le) = crate::ai::rules::evaluate_low_effort(&item, &req.feed_type) {
            tags.push(le);
        }
        tags
    } else {
        vec![]
    };

    // ── Text path: run for all items that are not direct image URLs ───────────
    // Direct image URLs (i.redd.it, imgur, etc.) have no meaningful body text.
    // Short titles (< 5 words) have too few char n-gram features for reliable ML
    // classification — bag-of-ngrams models produce spurious high-confidence tags
    // from n-gram collisions. Rules already handle the clear short-title cases.
    let title_words = item.title.split_whitespace().count();
    let ml_tags: Vec<TagResult> = if !is_direct_image && title_words >= 5 {
        let text = crate::training::build_input_text(&item.title, item.url.as_deref());
        match text_backend {
            crate::config::TextBackend::FastText => fasttext
                .and_then(|ft| {
                    ft.classify(&text)
                        .map_err(|e| tracing::warn!(error = %e, "fasttext classify failed"))
                        .ok()
                })
                .unwrap_or_default(),
            crate::config::TextBackend::MiniMl => miniml
                .and_then(|ml| {
                    ml.classify(&text)
                        .map_err(|e| tracing::warn!(error = %e, "miniml classify failed"))
                        .ok()
                })
                .unwrap_or_default(),
            crate::config::TextBackend::HybridFastTextMiniMl => {
                hybrid_classify(fasttext, miniml, &text)
            }
            crate::config::TextBackend::Nli => vec![],
        }
    } else {
        vec![]
    };

    // Merge: rules take priority (higher confidence); ML fills in the rest.
    let mut tags = merge_tags(ml_tags, rule_tags);

    // ── Vision path: run for direct image URLs and items with og_image ────────
    // og_image is set by the enrichment pass for articles that have a thumbnail.
    let image_url: Option<&str> = if is_direct_image {
        item.url.as_deref()
    } else {
        item.source_meta
            .get("og_image")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
    };

    if let (Some(vision), Some(url)) = (vision, image_url) {
        match vision.classify_image_url(url).await {
            Ok(vision_tags) if !vision_tags.is_empty() => {
                tags = merge_tags(tags, vision_tags);
            }
            Ok(_) => {}
            Err(e) => {
                tracing::debug!(url, "Vision tagging skipped: {e}");
            }
        }
    }

    // Suppress `no-context` when a substantive topic tag is present.
    // A specific question (local-rec, civic, etc.) is not vague even if short.
    let has_substantive = tags
        .iter()
        .any(|t| SUBSTANTIVE_TAGS.contains(&t.tag.as_str()));
    if has_substantive {
        tags.retain(|t| t.tag != "no-context");
    }

    // When `noise` fires with high confidence, strip semantic topic tags that
    // would otherwise be false positives — a personal food post or scenery share
    // cannot simultaneously be technical/security/policy/ai-ml content.
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

/// Merge text and vision tag lists.
/// Tags present in both: keep the one with higher confidence.
/// Tags present in only one: include as-is.
fn merge_tags(text_tags: Vec<TagResult>, vision_tags: Vec<TagResult>) -> Vec<TagResult> {
    let mut by_tag: HashMap<String, TagResult> =
        text_tags.into_iter().map(|t| (t.tag.clone(), t)).collect();

    for vt in vision_tags {
        let entry = by_tag.entry(vt.tag.clone()).or_insert_with(|| vt.clone());
        if vt.confidence > entry.confidence {
            *entry = vt;
        }
    }

    by_tag.into_values().collect()
}

const MINIML_CATEGORIES: &[&str] = &[
    "research",
    "clickbait",
    "technical",
    "civic",
    "culture",
    "local-rec",
    "no-context",
    "noise",
];
const HYBRID_FASTTEXT_CONFIDENCE: f32 = 0.65;

fn hybrid_classify(
    fasttext: Option<&crate::ai::fasttext::FastTextTagger>,
    miniml: Option<&crate::ai::miniml::MiniMlTagger>,
    text: &str,
) -> Vec<TagResult> {
    let ft_tags = fasttext
        .and_then(|ft| {
            ft.classify(text)
                .map_err(|e| tracing::warn!(error = %e, "fasttext classify failed"))
                .ok()
        })
        .unwrap_or_default();

    let ml_tags = miniml
        .and_then(|ml| {
            ml.classify(text)
                .map_err(|e| tracing::warn!(error = %e, "miniml classify failed"))
                .ok()
        })
        .unwrap_or_default();

    // Start with FastText results
    let mut result: HashMap<String, TagResult> =
        ft_tags.into_iter().map(|t| (t.tag.clone(), t)).collect();

    // For semantic categories: MiniLM overrides FastText when FastText isn't confident
    for ml_tag in ml_tags {
        if !MINIML_CATEGORIES.contains(&ml_tag.tag.as_str()) {
            continue;
        }
        let ft_confident = result
            .get(&ml_tag.tag)
            .map(|t| t.confidence >= HYBRID_FASTTEXT_CONFIDENCE)
            .unwrap_or(false);
        if !ft_confident {
            result.insert(ml_tag.tag.clone(), ml_tag);
        }
    }

    result.into_values().collect()
}
