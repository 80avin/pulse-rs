pub mod ai;
pub mod config;
pub mod error;
pub mod feeds;
pub mod search;
pub mod storage;
pub mod sync;
pub mod timeline;
pub mod types;

use std::sync::Arc;
use tokio::sync::mpsc;

use crate::ai::{RuleEngine, TaggerHandle, OnnxTagger, default_rules, tagger_task, TAGGER_QUEUE_SIZE};
use crate::ai::tagger::process_tag_request;
use crate::ai::tagger::TagRequest;
use crate::feeds::{fetch_enrichment, should_enrich, is_image_url};
use crate::config::PulseConfig;
use crate::error::PulseError;
use crate::storage::actor::{db_writer_task, DbHandle};
use crate::storage::connection::{open_writer_pool, open_reader_pool};
use crate::storage::migrations::run_migrations;
use crate::sync::SyncScheduler;
use crate::timeline::TimelineService;
use crate::search::SearchService;
use crate::types::{
    FeedId, ItemId, ItemStatePatch, Feed, FeedGroup,
    TimelineCursor, TimelineFilter, TimelinePage, FeedItemView,
    AiTag, DbStats, EnrichStats, EnrichItemResult, EnrichStatus,
};
use crate::storage::queries::{get_pending_enrichment, count_pending_enrichment};

/// Top-level application core. Holds all subsystem handles.
pub struct PulseCore {
    pub db: DbHandle,
    pub scheduler: Arc<SyncScheduler>,
    pub tagger: TaggerHandle,
    pub timeline: TimelineService,
    pub search: SearchService,
    pub config: Arc<PulseConfig>,
    pub rule_engine: Arc<RuleEngine>,
    /// ONNX tagger — Some when a model is loaded, None when running rules-only.
    pub onnx_tagger: Option<Arc<OnnxTagger>>,
}

impl PulseCore {
    /// Initialize PulseCore with the given configuration.
    pub async fn init(config: PulseConfig) -> Result<Self, PulseError> {
        let config = Arc::new(config);

        // Ensure data directory exists
        if let Some(parent) = config.db_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                PulseError::Config(format!("Failed to create data dir: {e}"))
            })?;
        }

        // Open writer pool (single connection) and run migrations
        let writer_pool = open_writer_pool(&config.db_path, &config).await
            .map_err(PulseError::Storage)?;
        run_migrations(&writer_pool).await
            .map_err(PulseError::Storage)?;

        // Open reader pool (concurrent reads via WAL)
        let reader_pool = open_reader_pool(&config.db_path, &config).await
            .map_err(PulseError::Storage)?;

        // Spawn the DB writer actor
        let (writer_tx, writer_rx) = mpsc::channel::<crate::storage::actor::DbCommand>(128);
        let writer_pool_for_actor = writer_pool.clone();
        tokio::spawn(async move {
            db_writer_task(writer_rx, writer_pool_for_actor).await;
        });

        let db = DbHandle::new(writer_tx, reader_pool);

        // Try loading the active ONNX model (non-fatal if absent or feature disabled)
        let onnx_tagger: Option<Arc<OnnxTagger>> = {
            let active_model_file = config.data_dir.join("active_model");
            if let Ok(model_name) = std::fs::read_to_string(&active_model_file) {
                let model_name = model_name.trim().to_string();
                let model_dir = config.data_dir.join("models").join(&model_name);
                match OnnxTagger::load(&model_dir) {
                    Ok(t) => {
                        tracing::info!(model = %model_name, "ONNX tagger loaded");
                        Some(Arc::new(t))
                    }
                    Err(crate::error::TaggingError::ModelNotLoaded) => {
                        tracing::debug!(model = %model_name, "Model files not found or ai-onnx feature disabled");
                        None
                    }
                    Err(e) => {
                        tracing::warn!(model = %model_name, error = %e, "Failed to load ONNX model, falling back to rules");
                        None
                    }
                }
            } else {
                None
            }
        };

        // Spawn the AI tagger task
        let (tagger_tx, tagger_rx) = mpsc::channel(TAGGER_QUEUE_SIZE);
        let tagger_handle = TaggerHandle::new(tagger_tx);

        let rule_engine = Arc::new(RuleEngine::new(default_rules()));
        let db_for_tagger = db.clone();
        let engine_for_task = rule_engine.clone();
        let onnx_for_task = onnx_tagger.clone();
        tokio::spawn(async move {
            tagger_task(tagger_rx, db_for_tagger, engine_for_task, onnx_for_task).await;
        });

        // Initialize the sync scheduler
        let scheduler = Arc::new(SyncScheduler::new(db.clone(), tagger_handle.clone()));

        let timeline = TimelineService::new(db.clone());
        let search = SearchService::new(db.clone());

        Ok(Self {
            db,
            scheduler,
            tagger: tagger_handle,
            timeline,
            search,
            config,
            rule_engine,
            onnx_tagger,
        })
    }

    /// Start syncing all enabled feeds in the background
    pub async fn start_sync(&self) {
        self.scheduler.start_all().await;
    }

    /// Run a sync for a single feed, awaiting completion. Returns new item count.
    pub async fn sync_feed(&self, feed_id: &FeedId) -> Result<usize, PulseError> {
        self.scheduler.sync_feed_blocking(feed_id).await.map_err(PulseError::Sync)
    }

    /// Shut down all background tasks
    pub async fn shutdown(&self) {
        self.scheduler.shutdown().await;
    }

    // ─── Enrichment ───────────────────────────────────────────────────────────

    /// Enrich pending items (fetch OG metadata for link posts).
    /// Returns stats about what happened.
    pub async fn enrich_pending(
        &self,
        feed_id: Option<&str>,
        limit: usize,
        concurrency: usize,
        progress_cb: impl Fn(&EnrichItemResult) + Send + Sync,
    ) -> Result<EnrichStats, PulseError> {
        let candidates = {
            let fid = feed_id.map(|s| s.to_string());
            self.db.with_reader(|pool| async move {
                get_pending_enrichment(&pool, fid.as_deref(), limit).await
            }).await.map_err(PulseError::Storage)?
        };

        let http = self.scheduler.http_client();
        let db = self.db.clone();
        let now = chrono::Utc::now().timestamp();

        let mut stats = EnrichStats::default();

        // Process with bounded concurrency using futures::stream
        use futures::stream::{self, StreamExt};

        let results: Vec<EnrichItemResult> = stream::iter(candidates)
            .map(|candidate| {
                let http = http.clone();
                let url = candidate.url.clone();
                async move {
                    let url_str = url.as_str();
                    if is_image_url(url_str) {
                        return EnrichItemResult {
                            item_id: candidate.id,
                            url,
                            status: EnrichStatus::Image,
                            og_description: None,
                            og_image: None,
                            og_title: None,
                        };
                    }
                    if !should_enrich(url_str) {
                        return EnrichItemResult {
                            item_id: candidate.id,
                            url,
                            status: EnrichStatus::Skipped,
                            og_description: None,
                            og_image: None,
                            og_title: None,
                        };
                    }
                    match fetch_enrichment(&http, url_str).await {
                        Ok(r) if r.skipped => EnrichItemResult {
                            item_id: candidate.id, url,
                            status: EnrichStatus::Skipped,
                            og_description: None, og_image: None, og_title: None,
                        },
                        Ok(r) if r.is_image => EnrichItemResult {
                            item_id: candidate.id, url,
                            status: EnrichStatus::Image,
                            og_description: None, og_image: None, og_title: None,
                        },
                        Ok(r) => EnrichItemResult {
                            item_id: candidate.id,
                            url,
                            status: EnrichStatus::Ok,
                            og_description: r.og_description,
                            og_image: r.og_image,
                            og_title: r.og_title,
                        },
                        Err(e) => {
                            tracing::debug!("Enrichment failed for {}: {}", url, e);
                            EnrichItemResult {
                                item_id: candidate.id, url,
                                status: EnrichStatus::Error(e.to_string()),
                                og_description: None, og_image: None, og_title: None,
                            }
                        }
                    }
                }
            })
            .buffer_unordered(concurrency)
            .collect()
            .await;

        for result in &results {
            progress_cb(result);

            match &result.status {
                EnrichStatus::Ok     => stats.enriched += 1,
                EnrichStatus::Image  => stats.image_posts += 1,
                EnrichStatus::Skipped => stats.skipped += 1,
                EnrichStatus::Error(_) => stats.errors += 1,
            }

            // Write enriched_at only when we got a definitive answer (ok/image/skipped).
            // Errors are NOT marked done — they can be retried on next run.
            if matches!(result.status, EnrichStatus::Error(_)) {
                continue;
            }
            let mut patch = serde_json::json!({ "enriched_at": now });
            if let Some(ref desc) = result.og_description {
                patch["og_description"] = serde_json::Value::String(desc.clone());
            }
            if let Some(ref img) = result.og_image {
                patch["og_image"] = serde_json::Value::String(img.clone());
            }
            if let Some(ref title) = result.og_title {
                patch["og_title"] = serde_json::Value::String(title.clone());
            }
            if matches!(result.status, EnrichStatus::Image) {
                patch["is_image"] = serde_json::Value::Bool(true);
            }

            let body_text = result.og_description.clone();
            let _ = db.enrich_item(result.item_id.clone(), body_text, patch).await;
        }

        Ok(stats)
    }

    /// Count items pending enrichment (no enriched_at in source_meta).
    pub async fn count_pending_enrichment(&self, feed_id: Option<&str>) -> Result<i64, PulseError> {
        let fid = feed_id.map(|s| s.to_string());
        self.db.with_reader(|pool| async move {
            count_pending_enrichment(&pool, fid.as_deref()).await
        }).await.map_err(PulseError::Storage)
    }

    // ─── Feed management ──────────────────────────────────────────────────────

    pub async fn get_feeds(&self) -> Result<Vec<Feed>, PulseError> {
        self.db.with_reader(|pool| async move {
            storage::queries::get_feeds(&pool).await
        }).await.map_err(PulseError::Storage)
    }

    pub async fn get_feed(&self, feed_id: &FeedId) -> Result<Feed, PulseError> {
        let fid = feed_id.clone();
        self.db.with_reader(|pool| async move {
            storage::queries::get_feed(&pool, &fid).await
        }).await.map_err(PulseError::Storage)
    }

    pub async fn add_feed(&self, feed: Feed) -> Result<(), PulseError> {
        let feed_id = feed.id.clone();
        self.db.upsert_feed(feed).await.map_err(PulseError::Storage)?;
        self.scheduler.add_feed(feed_id).await;
        Ok(())
    }

    pub async fn delete_feed(&self, feed_id: &FeedId) -> Result<(), PulseError> {
        self.scheduler.remove_feed(feed_id).await;
        self.db.delete_feed(feed_id.clone()).await.map_err(PulseError::Storage)
    }

    pub async fn get_feed_groups(&self) -> Result<Vec<FeedGroup>, PulseError> {
        self.db.with_reader(|pool| async move {
            storage::queries::get_feed_groups(&pool).await
        }).await.map_err(PulseError::Storage)
    }

    // ─── Timeline ─────────────────────────────────────────────────────────────

    pub async fn get_timeline_page(
        &self,
        filter: TimelineFilter,
        cursor: Option<TimelineCursor>,
        limit: usize,
    ) -> Result<TimelinePage, PulseError> {
        self.timeline.get_page(filter, cursor, limit).await.map_err(PulseError::Storage)
    }

    // ─── Item state ───────────────────────────────────────────────────────────

    /// Resolve a full or prefix item ID to the canonical full UUID.
    pub async fn resolve_item_id(&self, prefix: &str) -> Result<Option<ItemId>, PulseError> {
        let prefix = prefix.to_string();
        self.db.with_reader(|pool| async move {
            storage::queries::resolve_item_id(&pool, &prefix).await
        }).await.map_err(PulseError::Storage)
    }

    pub async fn update_item_state(
        &self,
        item_id: &ItemId,
        patch: ItemStatePatch,
    ) -> Result<(), PulseError> {
        self.db.update_item_state(item_id.clone(), patch).await.map_err(PulseError::Storage)
    }

    pub async fn mark_read(&self, item_id: &ItemId) -> Result<(), PulseError> {
        self.update_item_state(item_id, ItemStatePatch {
            is_read: Some(true),
            ..Default::default()
        }).await
    }

    pub async fn toggle_saved(&self, item_id: &ItemId, saved: bool) -> Result<(), PulseError> {
        self.update_item_state(item_id, ItemStatePatch {
            is_saved: Some(saved),
            ..Default::default()
        }).await
    }

    pub async fn hide_item(&self, item_id: &ItemId) -> Result<(), PulseError> {
        self.update_item_state(item_id, ItemStatePatch {
            is_hidden: Some(true),
            ..Default::default()
        }).await
    }

    // ─── Search ───────────────────────────────────────────────────────────────

    pub async fn search(&self, query: &str, limit: Option<usize>) -> Result<Vec<FeedItemView>, PulseError> {
        self.search.search(query, limit).await.map_err(PulseError::Storage)
    }

    // ─── AI tags ──────────────────────────────────────────────────────────────

    /// Tag all untagged items (or items for a specific feed) directly, without the async queue.
    /// Returns `(items_processed, tags_created)`. Safe to call from CLI — awaits completion.
    pub async fn run_tagger_direct(
        &self,
        feed_id: Option<&str>,
    ) -> Result<(usize, usize), PulseError> {
        let feeds = self.get_feeds().await?;
        let targets: Vec<_> = match feed_id {
            Some(fid) => feeds.into_iter().filter(|f| f.id == fid).collect(),
            None => feeds,
        };

        let mut items_processed = 0usize;
        let mut tags_created = 0usize;

        for feed in &targets {
            let page = self.get_timeline_page(
                TimelineFilter { feed_id: Some(feed.id.clone()), ..Default::default() },
                None,
                1000,
            ).await?;

            for item in page.items.iter().filter(|i| i.ai_tags.is_empty()) {
                let req = TagRequest {
                    item_id: item.id.clone(),
                    feed_type: feed.feed_type.clone(),
                };
                match process_tag_request(&self.db, &self.rule_engine, self.onnx_tagger.as_deref(), &req).await {
                    Ok(n) => {
                        tags_created += n;
                        items_processed += 1;
                    }
                    Err(e) => {
                        tracing::warn!(item_id = %item.id, "Direct tagging failed: {}", e);
                        items_processed += 1;
                    }
                }
            }
        }

        Ok((items_processed, tags_created))
    }

    pub async fn get_item_tags(&self, item_id: &ItemId) -> Result<Vec<AiTag>, PulseError> {
        let iid = item_id.clone();
        self.db.with_reader(|pool| async move {
            storage::queries::get_ai_tags(&pool, &iid).await
        }).await.map_err(PulseError::Storage)
    }

    // ─── AI model management ──────────────────────────────────────────────────

    /// Returns the name of the currently active model, or None if using rules-only.
    pub fn active_model_name(&self) -> Option<String> {
        let path = self.config.data_dir.join("active_model");
        std::fs::read_to_string(path).ok().map(|s| s.trim().to_string())
    }

    /// List all downloaded model names (directories under {data_dir}/models/).
    pub fn list_models(&self) -> Vec<String> {
        let models_dir = self.config.data_dir.join("models");
        std::fs::read_dir(&models_dir)
            .map(|entries| {
                entries.filter_map(|e| e.ok())
                    .filter(|e| {
                        let p = e.path();
                        p.is_dir() && (p.join("model_quantized.onnx").exists() || p.join("model.onnx").exists())
                    })
                    .filter_map(|e| e.file_name().into_string().ok())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Set the active model by name. The model directory must contain model_quantized.onnx or model.onnx.
    pub fn set_active_model(&self, model_name: &str) -> Result<(), PulseError> {
        let model_dir = self.config.data_dir.join("models").join(model_name);
        let has_model = model_dir.join("model_quantized.onnx").exists()
            || model_dir.join("model.onnx").exists();
        if !has_model {
            return Err(PulseError::NotFound(format!(
                "model '{}' not found at {:?} — download model_quantized.onnx (or model.onnx) and tokenizer.json there first",
                model_name, model_dir
            )));
        }
        let active_file = self.config.data_dir.join("active_model");
        std::fs::write(&active_file, model_name)
            .map_err(|e| PulseError::Config(format!("failed to write active_model: {e}")))?;
        Ok(())
    }

    /// Remove the active_model pointer (fall back to rules-only).
    pub fn unset_active_model(&self) -> Result<(), PulseError> {
        let active_file = self.config.data_dir.join("active_model");
        if active_file.exists() {
            std::fs::remove_file(&active_file)
                .map_err(|e| PulseError::Config(format!("failed to remove active_model: {e}")))?;
        }
        Ok(())
    }

    /// Remove a downloaded model directory.
    pub fn remove_model(&self, model_name: &str) -> Result<(), PulseError> {
        let model_dir = self.config.data_dir.join("models").join(model_name);
        if !model_dir.exists() {
            return Err(PulseError::NotFound(format!("model '{}' not found", model_name)));
        }
        std::fs::remove_dir_all(&model_dir)
            .map_err(|e| PulseError::Config(format!("failed to remove model directory: {e}")))?;
        // If this was the active model, clear the pointer
        if self.active_model_name().as_deref() == Some(model_name) {
            let _ = self.unset_active_model();
        }
        Ok(())
    }

    /// Return the path where model files should be placed for a given model name.
    pub fn model_dir(&self, model_name: &str) -> std::path::PathBuf {
        self.config.data_dir.join("models").join(model_name)
    }

    // ─── Stats ────────────────────────────────────────────────────────────────

    pub async fn get_db_stats(&self) -> Result<DbStats, PulseError> {
        self.db.with_reader(|pool| async move {
            storage::queries::get_db_stats(&pool).await
        }).await.map_err(PulseError::Storage)
    }
}
