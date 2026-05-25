pub mod ai;
pub mod config;
pub mod error;
pub mod feeds;
pub mod search;
pub mod storage;
pub mod sync;
pub mod timeline;
pub mod training;
pub mod types;

use std::sync::Arc;
use tokio::sync::mpsc;

use crate::ai::tagger::TagRequest;
use crate::ai::tagger::process_tag_request;
use crate::ai::{
    FastTextTagger, MiniMlTagger, ModelHandle, OnnxTagger, RuleEngine, TAGGER_QUEUE_SIZE,
    TaggerHandle, VisionTagger, default_rules, tagger_task,
};
use crate::config::PulseConfig;
use crate::error::PulseError;
use crate::feeds::{RedditAuth, fetch_enrichment, is_image_url, should_enrich};
use crate::search::SearchService;
use crate::storage::actor::{DbHandle, db_writer_task};
use crate::storage::connection::{open_reader_pool, open_writer_pool};
use crate::storage::migrations::run_migrations;
use crate::storage::queries::{count_pending_enrichment, get_pending_enrichment};
use crate::sync::SyncScheduler;
use crate::timeline::TimelineService;
use crate::types::{
    AiTag, DbStats, EnrichItemResult, EnrichStats, EnrichStatus, Feed, FeedGroup, FeedId,
    FeedItemView, FeedType, ItemId, ItemStatePatch, TimelineCursor, TimelineFilter, TimelinePage,
};

/// Top-level application core. Holds all subsystem handles.
pub struct PulseCore {
    pub db: DbHandle,
    pub scheduler: Arc<SyncScheduler>,
    pub tagger: TaggerHandle,
    pub timeline: TimelineService,
    pub search: SearchService,
    pub config: Arc<PulseConfig>,
    pub rule_engine: Arc<RuleEngine>,
    /// NLI cross-encoder — idle-unloadable; only loaded when TextBackend::Nli is active.
    pub onnx_tagger: ModelHandle<OnnxTagger>,
    /// CLIP vision encoder — idle-unloadable; loaded on first image post, dropped when idle.
    pub vision_tagger: ModelHandle<VisionTagger>,
    /// FastText supervised classifier — primary text tagger (<10MB, <1ms/item).
    pub fasttext_tagger: ModelHandle<FastTextTagger>,
    /// MiniLM + MLP semantic classifier — secondary for research/discussion/clickbait.
    pub miniml_tagger: ModelHandle<MiniMlTagger>,
}

impl PulseCore {
    /// Initialize PulseCore with the given configuration.
    pub async fn init(config: PulseConfig) -> Result<Self, PulseError> {
        let t0 = std::time::Instant::now();
        let config = Arc::new(config);

        // Ensure data directory exists
        if let Some(parent) = config.db_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| PulseError::Config(format!("Failed to create data dir: {e}")))?;
        }

        // Open writer pool (single connection) and run migrations
        let writer_pool = open_writer_pool(&config.db_path, &config)
            .await
            .map_err(PulseError::Storage)?;
        run_migrations(&writer_pool)
            .await
            .map_err(PulseError::Storage)?;
        tracing::info!(
            elapsed_ms = t0.elapsed().as_millis(),
            "coldstart: db open + migrations"
        );

        // Open reader pool (concurrent reads via WAL)
        let reader_pool = open_reader_pool(&config.db_path, &config)
            .await
            .map_err(PulseError::Storage)?;
        tracing::info!(
            elapsed_ms = t0.elapsed().as_millis(),
            "coldstart: reader pool open"
        );

        // Spawn the DB writer actor
        let (writer_tx, writer_rx) = mpsc::channel::<crate::storage::actor::DbCommand>(128);
        let writer_pool_for_actor = writer_pool.clone();
        tokio::spawn(async move {
            db_writer_task(writer_rx, writer_pool_for_actor).await;
        });

        let db = DbHandle::new(writer_tx, reader_pool);
        tracing::info!(elapsed_ms = t0.elapsed().as_millis(), "coldstart: DB ready");

        // Build loader closures — each captures the data_dir and loads from the active-model
        // pointer file. Returning None means "no active model configured"; errors are logged.
        let make_onnx_loader = {
            let data_dir = config.data_dir.clone();
            Arc::new(move || -> Option<Arc<OnnxTagger>> {
                let name = std::fs::read_to_string(data_dir.join("active_model")).ok()?;
                let dir = data_dir.join("models").join(name.trim());
                let t = std::time::Instant::now();
                match OnnxTagger::load(&dir) {
                    Ok(tagger) => {
                        tracing::info!(elapsed_ms = t.elapsed().as_millis(), "ONNX tagger loaded");
                        Some(Arc::new(tagger))
                    }
                    Err(crate::error::TaggingError::ModelNotLoaded) => None,
                    Err(e) => {
                        tracing::warn!(error=%e, "ONNX load failed");
                        None
                    }
                }
            }) as Arc<dyn Fn() -> Option<Arc<OnnxTagger>> + Send + Sync>
        };

        let make_vision_loader = {
            let data_dir = config.data_dir.clone();
            Arc::new(move || -> Option<Arc<VisionTagger>> {
                let name = std::fs::read_to_string(data_dir.join("active_vision_model")).ok()?;
                let dir = data_dir.join("models").join(name.trim());
                let t = std::time::Instant::now();
                match VisionTagger::load(&dir) {
                    Ok(tagger) => {
                        tracing::info!(
                            elapsed_ms = t.elapsed().as_millis(),
                            "Vision tagger loaded"
                        );
                        Some(Arc::new(tagger))
                    }
                    Err(crate::error::TaggingError::ModelNotLoaded) => None,
                    Err(e) => {
                        tracing::warn!(error=%e, "Vision load failed");
                        None
                    }
                }
            }) as Arc<dyn Fn() -> Option<Arc<VisionTagger>> + Send + Sync>
        };

        let make_fasttext_loader = {
            let data_dir = config.data_dir.clone();
            Arc::new(move || -> Option<Arc<FastTextTagger>> {
                let name = std::fs::read_to_string(data_dir.join("active_fasttext_model")).ok()?;
                let dir = data_dir.join("models").join(name.trim());
                let t = std::time::Instant::now();
                match FastTextTagger::load(&dir) {
                    Ok(tagger) => {
                        tracing::info!(
                            elapsed_ms = t.elapsed().as_millis(),
                            "FastText tagger loaded"
                        );
                        Some(Arc::new(tagger))
                    }
                    Err(crate::error::TaggingError::ModelNotLoaded) => None,
                    Err(e) => {
                        tracing::warn!(error=%e, "FastText load failed");
                        None
                    }
                }
            }) as Arc<dyn Fn() -> Option<Arc<FastTextTagger>> + Send + Sync>
        };

        let make_miniml_loader = {
            let data_dir = config.data_dir.clone();
            Arc::new(move || -> Option<Arc<MiniMlTagger>> {
                let name = std::fs::read_to_string(data_dir.join("active_miniml_model")).ok()?;
                let dir = data_dir.join("models").join(name.trim());
                let t = std::time::Instant::now();
                match MiniMlTagger::load(&dir) {
                    Ok(tagger) => {
                        tracing::info!(
                            elapsed_ms = t.elapsed().as_millis(),
                            "MiniLM tagger loaded"
                        );
                        Some(Arc::new(tagger))
                    }
                    Err(crate::error::TaggingError::ModelNotLoaded) => None,
                    Err(e) => {
                        tracing::warn!(error=%e, "MiniLM load failed");
                        None
                    }
                }
            }) as Arc<dyn Fn() -> Option<Arc<MiniMlTagger>> + Send + Sync>
        };

        // Create ModelHandles. snapshot() sets pending_reload=true before spawning so concurrent
        // calls from tagger_task cannot trigger a second load while init is still in flight.
        let onnx_tagger = ModelHandle::new(make_onnx_loader);
        let vision_tagger = ModelHandle::new(make_vision_loader);
        let fasttext_tagger = ModelHandle::new(make_fasttext_loader);
        let miniml_tagger = ModelHandle::new(make_miniml_loader);

        // Kick off background loads only when AI tagging is enabled.
        // When disabled, models stay unloaded until the user re-enables AI and triggers a retag;
        // snapshot() in run_tagger_direct / tagger_task will load them on demand at that point.
        if config.ai_enabled {
            if config.text_backend == crate::config::TextBackend::Nli {
                onnx_tagger.snapshot();
            }
            fasttext_tagger.snapshot();
            miniml_tagger.snapshot();
            vision_tagger.snapshot();
        } else {
            tracing::info!("AI tagging disabled — skipping model preload");
        }

        // Spawn idle-unload janitor — checks every 60 s and evicts models unused past their threshold.
        {
            let ft = fasttext_tagger.clone();
            let ml = miniml_tagger.clone();
            let onnx = onnx_tagger.clone();
            let vis = vision_tagger.clone();
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
                interval.tick().await; // first tick fires immediately; skip it
                loop {
                    interval.tick().await;
                    ft.idle_drop(std::time::Duration::from_secs(30 * 60));
                    ml.idle_drop(std::time::Duration::from_secs(10 * 60));
                    onnx.idle_drop(std::time::Duration::from_secs(5 * 60));
                    vis.idle_drop(std::time::Duration::from_secs(5 * 60));
                }
            });
        }

        // Spawn the AI tagger task
        let (tagger_tx, tagger_rx) = mpsc::channel(TAGGER_QUEUE_SIZE);
        let tagger_handle = TaggerHandle::new(tagger_tx);

        let rule_engine = Arc::new(RuleEngine::new(default_rules()));
        let db_for_tagger = db.clone();
        let rule_engine_for_task = rule_engine.clone();
        let fasttext_for_task = fasttext_tagger.clone();
        let miniml_for_task = miniml_tagger.clone();
        let vision_for_task = vision_tagger.clone();
        let text_backend_for_task = config.text_backend.clone();
        tokio::spawn(async move {
            tagger_task(
                tagger_rx,
                db_for_tagger,
                text_backend_for_task,
                rule_engine_for_task,
                fasttext_for_task,
                miniml_for_task,
                vision_for_task,
            )
            .await;
        });

        // Build Reddit auth from config if credentials are provided
        let reddit_auth = match (
            config.reddit_client_id.as_deref(),
            config.reddit_client_secret.as_deref(),
        ) {
            (Some(id), Some(secret)) => {
                tracing::info!(
                    "Reddit OAuth2 enabled (client_id={}...)",
                    &id[..id.len().min(8)]
                );
                Some(Arc::new(RedditAuth::new(
                    id.to_string(),
                    secret.to_string(),
                )))
            }
            _ => None,
        };

        // Initialize the sync scheduler
        let scheduler = Arc::new(SyncScheduler::new(
            db.clone(),
            tagger_handle.clone(),
            reddit_auth,
        ));

        let timeline = TimelineService::new(db.clone());
        let search = SearchService::new(db.clone());
        tracing::info!(
            elapsed_ms = t0.elapsed().as_millis(),
            "coldstart: PulseCore::init complete"
        );

        Ok(Self {
            db,
            scheduler,
            tagger: tagger_handle,
            timeline,
            search,
            config,
            rule_engine,
            onnx_tagger,
            vision_tagger,
            fasttext_tagger,
            miniml_tagger,
        })
    }

    /// Start syncing all enabled feeds in the background
    pub async fn start_sync(&self) {
        self.scheduler.start_all().await;
    }

    /// Run a sync for a single feed, awaiting completion. Returns new item count.
    pub async fn sync_feed(&self, feed_id: &FeedId) -> Result<usize, PulseError> {
        self.scheduler
            .sync_feed_blocking(feed_id)
            .await
            .map_err(PulseError::Sync)
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
            self.db
                .with_reader(|pool| async move {
                    get_pending_enrichment(&pool, fid.as_deref(), limit).await
                })
                .await
                .map_err(PulseError::Storage)?
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
                            item_id: candidate.id,
                            url,
                            status: EnrichStatus::Skipped,
                            og_description: None,
                            og_image: None,
                            og_title: None,
                        },
                        Ok(r) if r.is_image => EnrichItemResult {
                            item_id: candidate.id,
                            url,
                            status: EnrichStatus::Image,
                            og_description: None,
                            og_image: None,
                            og_title: None,
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
                            tracing::warn!(url = %url, error = %e, "enrichment fetch failed");
                            EnrichItemResult {
                                item_id: candidate.id,
                                url,
                                status: EnrichStatus::Error(e.to_string()),
                                og_description: None,
                                og_image: None,
                                og_title: None,
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
                EnrichStatus::Ok => stats.enriched += 1,
                EnrichStatus::Image => stats.image_posts += 1,
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
            let _ = db
                .enrich_item(result.item_id.clone(), body_text, patch)
                .await;

            // Re-queue for tagging if og_image was acquired — the vision tagger
            // needs the image URL which wasn't available at initial sync time.
            if let Some(ref img) = result.og_image
                && !img.is_empty()
            {
                self.tagger
                    .tag_item(result.item_id.clone(), FeedType::Rss)
                    .await;
            }
        }

        Ok(stats)
    }

    /// Count items pending enrichment (no enriched_at in source_meta).
    pub async fn count_pending_enrichment(&self, feed_id: Option<&str>) -> Result<i64, PulseError> {
        let fid = feed_id.map(|s| s.to_string());
        self.db
            .with_reader(
                |pool| async move { count_pending_enrichment(&pool, fid.as_deref()).await },
            )
            .await
            .map_err(PulseError::Storage)
    }

    // ─── Feed management ──────────────────────────────────────────────────────

    pub async fn get_feeds(&self) -> Result<Vec<Feed>, PulseError> {
        self.db
            .with_reader(|pool| async move { storage::queries::get_feeds(&pool).await })
            .await
            .map_err(PulseError::Storage)
    }

    pub async fn get_feed(&self, feed_id: &FeedId) -> Result<Feed, PulseError> {
        let fid = feed_id.clone();
        self.db
            .with_reader(|pool| async move { storage::queries::get_feed(&pool, &fid).await })
            .await
            .map_err(PulseError::Storage)
    }

    pub async fn add_feed(&self, feed: Feed) -> Result<(), PulseError> {
        let feed_id = feed.id.clone();
        self.db
            .upsert_feed(feed)
            .await
            .map_err(PulseError::Storage)?;
        self.scheduler.add_feed(feed_id).await;
        Ok(())
    }

    pub async fn delete_feed(&self, feed_id: &FeedId) -> Result<(), PulseError> {
        self.scheduler.remove_feed(feed_id).await;
        self.db
            .delete_feed(feed_id.clone())
            .await
            .map_err(PulseError::Storage)
    }

    pub async fn get_feed_groups(&self) -> Result<Vec<FeedGroup>, PulseError> {
        self.db
            .with_reader(|pool| async move { storage::queries::get_feed_groups(&pool).await })
            .await
            .map_err(PulseError::Storage)
    }

    /// Delete a feed group. Member feeds have their group_id set to NULL.
    pub async fn delete_feed_group(&self, id: &str) -> Result<(), PulseError> {
        self.db
            .delete_feed_group(id.to_string())
            .await
            .map_err(PulseError::Storage)
    }

    /// Delete all feed items (leaves feeds intact).
    pub async fn clear_all_items(&self) -> Result<(), PulseError> {
        self.db.clear_all_items().await.map_err(PulseError::Storage)
    }

    /// Mark all items in a feed as read.
    pub async fn mark_feed_read(&self, feed_id: &FeedId) -> Result<(), PulseError> {
        self.db
            .mark_feed_read(feed_id.clone())
            .await
            .map_err(PulseError::Storage)
    }

    /// Return a map of feed_id → unread item count.
    pub async fn get_unread_counts_by_feed(
        &self,
    ) -> Result<std::collections::HashMap<FeedId, i64>, PulseError> {
        self.db
            .with_reader(
                |pool| async move { storage::queries::get_unread_counts_by_feed(&pool).await },
            )
            .await
            .map_err(PulseError::Storage)
    }

    /// Return a map of feed_id → total (non-hidden) item count.
    pub async fn get_total_counts_by_feed(
        &self,
    ) -> Result<std::collections::HashMap<String, i64>, PulseError> {
        self.db
            .with_reader(
                |pool| async move { storage::queries::get_total_counts_by_feed(&pool).await },
            )
            .await
            .map_err(PulseError::Storage)
    }

    /// Fetch a single item by full or prefix ID. Returns body_text, body_html, source_meta.
    pub async fn get_item(&self, item_id: &ItemId) -> Result<crate::types::FeedItem, PulseError> {
        let iid = item_id.clone();
        self.db
            .with_reader(|pool| async move { storage::queries::get_item(&pool, &iid).await })
            .await
            .map_err(PulseError::Storage)
    }

    /// Clear the ETag, Last-Modified, and source_config cache keys for a feed,
    /// forcing the next sync to perform a full re-fetch regardless of prior state.
    pub async fn clear_feed_cache(&self, feed_id: &FeedId) -> Result<(), PulseError> {
        self.db
            .clear_feed_cache(feed_id.clone())
            .await
            .map_err(PulseError::Storage)
    }

    // ─── Timeline ─────────────────────────────────────────────────────────────

    pub async fn get_timeline_page(
        &self,
        filter: TimelineFilter,
        cursor: Option<TimelineCursor>,
        limit: usize,
    ) -> Result<TimelinePage, PulseError> {
        self.timeline
            .get_page(filter, cursor, limit)
            .await
            .map_err(PulseError::Storage)
    }

    // ─── Item state ───────────────────────────────────────────────────────────

    /// Resolve a full or prefix item ID to the canonical full UUID.
    pub async fn resolve_item_id(&self, prefix: &str) -> Result<Option<ItemId>, PulseError> {
        let prefix = prefix.to_string();
        self.db
            .with_reader(
                |pool| async move { storage::queries::resolve_item_id(&pool, &prefix).await },
            )
            .await
            .map_err(PulseError::Storage)
    }

    pub async fn update_item_state(
        &self,
        item_id: &ItemId,
        patch: ItemStatePatch,
    ) -> Result<(), PulseError> {
        self.db
            .update_item_state(item_id.clone(), patch)
            .await
            .map_err(PulseError::Storage)
    }

    pub async fn mark_read(&self, item_id: &ItemId) -> Result<(), PulseError> {
        self.update_item_state(
            item_id,
            ItemStatePatch {
                is_read: Some(true),
                ..Default::default()
            },
        )
        .await
    }

    pub async fn toggle_saved(&self, item_id: &ItemId, saved: bool) -> Result<(), PulseError> {
        self.update_item_state(
            item_id,
            ItemStatePatch {
                is_saved: Some(saved),
                ..Default::default()
            },
        )
        .await
    }

    pub async fn hide_item(&self, item_id: &ItemId) -> Result<(), PulseError> {
        self.update_item_state(
            item_id,
            ItemStatePatch {
                is_hidden: Some(true),
                ..Default::default()
            },
        )
        .await
    }

    // ─── Search ───────────────────────────────────────────────────────────────

    pub async fn search(
        &self,
        query: &str,
        limit: Option<usize>,
    ) -> Result<Vec<FeedItemView>, PulseError> {
        self.search
            .search(query, limit)
            .await
            .map_err(PulseError::Storage)
    }

    // ─── AI tags ──────────────────────────────────────────────────────────────

    /// Tag items directly without the async queue. Returns `(items_processed, tags_created)`.
    ///
    /// When `force = true`, deletes existing tags before retagging so vocabulary changes
    /// are fully applied. When `force = false`, only processes items with no tags.
    ///
    /// `on_progress(tagged, total)` is called after each item is processed.
    pub async fn run_tagger_direct(
        &self,
        feed_id: Option<&str>,
        force: bool,
        on_progress: Option<&(dyn Fn(usize, usize) + Send + Sync)>,
    ) -> Result<(usize, usize), PulseError> {
        let feeds = self.get_feeds().await?;
        let targets: Vec<_> = match feed_id {
            Some(fid) => feeds.into_iter().filter(|f| f.id == fid).collect(),
            None => feeds,
        };

        let mut work: Vec<(FeedItemView, crate::types::FeedType)> = Vec::new();
        for feed in &targets {
            let page = self
                .get_timeline_page(
                    TimelineFilter {
                        feed_id: Some(feed.id.clone()),
                        ..Default::default()
                    },
                    None,
                    10_000,
                )
                .await?;
            for item in page.items {
                if force || item.ai_tags.is_empty() {
                    work.push((item, feed.feed_type.clone()));
                }
            }
        }

        let total = work.len();
        let mut items_processed = 0usize;
        let mut tags_created = 0usize;

        let fasttext = self.fasttext_tagger.snapshot();
        let miniml = self.miniml_tagger.snapshot();
        let vision = self.vision_tagger.snapshot();

        for (item, feed_type) in work {
            if force {
                // Clear stale tags so removed/renamed tags don't persist.
                let _ = self.db.delete_item_tags(item.id.clone()).await;
            }
            let req = TagRequest {
                item_id: item.id.clone(),
                feed_type,
            };
            match process_tag_request(
                &self.db,
                &self.config.text_backend,
                &self.rule_engine,
                fasttext.as_deref(),
                miniml.as_deref(),
                vision.as_deref(),
                &req,
            )
            .await
            {
                Ok(n) => {
                    tags_created += n;
                }
                Err(e) => {
                    tracing::warn!(item_id = %item.id, "Direct tagging failed: {}", e);
                }
            }
            items_processed += 1;
            if let Some(cb) = on_progress {
                cb(items_processed, total);
            }
        }

        Ok((items_processed, tags_created))
    }

    /// Delete all AI tags with confidence below the given threshold (global post-filter).
    /// Used by `retag_all` to apply the user's confidence_threshold setting.
    pub async fn delete_tags_below_confidence(&self, threshold: f32) -> Result<(), PulseError> {
        self.db
            .delete_tags_below_confidence(threshold)
            .await
            .map_err(PulseError::Storage)
    }

    pub async fn get_item_tags(&self, item_id: &ItemId) -> Result<Vec<AiTag>, PulseError> {
        let iid = item_id.clone();
        self.db
            .with_reader(|pool| async move { storage::queries::get_ai_tags(&pool, &iid).await })
            .await
            .map_err(PulseError::Storage)
    }

    // ─── AI model management ──────────────────────────────────────────────────

    /// Returns the name of the currently active model, or None if using rules-only.
    pub fn active_model_name(&self) -> Option<String> {
        let path = self.config.data_dir.join("active_model");
        std::fs::read_to_string(path)
            .ok()
            .map(|s| s.trim().to_string())
    }

    /// List all downloaded model names (directories under {data_dir}/models/).
    pub fn list_models(&self) -> Vec<String> {
        let models_dir = self.config.data_dir.join("models");
        std::fs::read_dir(&models_dir)
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .filter(|e| {
                        let p = e.path();
                        p.is_dir()
                            && (p.join("model_quantized.onnx").exists()
                                || p.join("model.onnx").exists())
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
            return Err(PulseError::NotFound(format!(
                "model '{}' not found",
                model_name
            )));
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

    // ─── Vision model management ──────────────────────────────────────────────

    pub fn active_vision_model_name(&self) -> Option<String> {
        let path = self.config.data_dir.join("active_vision_model");
        std::fs::read_to_string(path)
            .ok()
            .map(|s| s.trim().to_string())
    }

    pub fn set_active_vision_model(&self, model_name: &str) -> Result<(), PulseError> {
        let model_dir = self.config.data_dir.join("models").join(model_name);
        // Accept any supported vision model filename (MobileCLIP int8 or CLIP ViT-B/32 q4f16)
        let has_model = [
            "vision_model_quantized.onnx",
            "vision_model_q4f16.onnx",
            "vision_model.onnx",
        ]
        .iter()
        .any(|f| model_dir.join(f).exists());
        if !has_model {
            return Err(PulseError::NotFound(format!(
                "vision model '{}' not found at {:?} — run 'pulse ai vision-download {}' first",
                model_name, model_dir, model_name
            )));
        }
        // label_embeddings.bin is generated at load time; no need to pre-check
        let active_file = self.config.data_dir.join("active_vision_model");
        std::fs::write(&active_file, model_name)
            .map_err(|e| PulseError::Config(format!("failed to write active_vision_model: {e}")))?;
        Ok(())
    }

    pub fn unset_active_vision_model(&self) -> Result<(), PulseError> {
        let active_file = self.config.data_dir.join("active_vision_model");
        if active_file.exists() {
            std::fs::remove_file(&active_file).map_err(|e| {
                PulseError::Config(format!("failed to remove active_vision_model: {e}"))
            })?;
        }
        Ok(())
    }

    // ─── Tagger hot-reload ────────────────────────────────────────────────────

    /// Whether an NLI text tagger is currently loaded.
    pub fn onnx_loaded(&self) -> bool {
        self.onnx_tagger.is_loaded()
    }

    /// Whether a CLIP vision tagger is currently loaded.
    pub fn vision_loaded(&self) -> bool {
        self.vision_tagger.is_loaded()
    }

    /// Reload the NLI tagger from the active_model file. Call after downloading a new model.
    /// The background tagger task sees the change on its next queued item.
    pub fn reload_onnx_tagger(&self) -> Result<(), PulseError> {
        let active_file = self.config.data_dir.join("active_model");
        let model_name = std::fs::read_to_string(&active_file)
            .map_err(|e| PulseError::Config(format!("no active model set: {e}")))?;
        let model_dir = self.config.data_dir.join("models").join(model_name.trim());
        let tagger = OnnxTagger::load(&model_dir).map_err(PulseError::Tagging)?;
        self.onnx_tagger.store(Arc::new(tagger));
        tracing::info!("NLI tagger hot-reloaded from {}", model_dir.display());
        Ok(())
    }

    /// Reload the CLIP vision tagger from the active_vision_model file.
    /// If `label_embeddings.bin` is missing, automatically computes it from
    /// the downloaded text encoder before loading the vision model.
    pub fn reload_vision_tagger(&self) -> Result<(), PulseError> {
        let active_file = self.config.data_dir.join("active_vision_model");
        let model_name = std::fs::read_to_string(&active_file)
            .map_err(|e| PulseError::Config(format!("no active vision model set: {e}")))?;
        let model_dir = self.config.data_dir.join("models").join(model_name.trim());

        // Auto-compute label embeddings from the text encoder if not yet present.
        #[cfg(feature = "ai-vision")]
        {
            let embeddings_path = model_dir.join("label_embeddings.bin");
            if !embeddings_path.exists() {
                tracing::info!("label_embeddings.bin missing — computing from text encoder");
                ai::vision::compute_clip_label_embeddings(&model_dir)
                    .map_err(PulseError::Tagging)?;
            }
        }

        let tagger = VisionTagger::load(&model_dir).map_err(PulseError::Tagging)?;
        self.vision_tagger.store(Arc::new(tagger));
        tracing::info!(
            "CLIP vision tagger hot-reloaded from {}",
            model_dir.display()
        );
        Ok(())
    }

    // ─── FastText model management ────────────────────────────────────────────

    pub fn fasttext_loaded(&self) -> bool {
        self.fasttext_tagger.is_loaded()
    }
    pub fn miniml_loaded(&self) -> bool {
        self.miniml_tagger.is_loaded()
    }

    pub fn active_fasttext_model_name(&self) -> Option<String> {
        let path = self.config.data_dir.join("active_fasttext_model");
        std::fs::read_to_string(path)
            .ok()
            .map(|s| s.trim().to_string())
    }

    pub fn active_miniml_model_name(&self) -> Option<String> {
        let path = self.config.data_dir.join("active_miniml_model");
        std::fs::read_to_string(path)
            .ok()
            .map(|s| s.trim().to_string())
    }

    pub fn set_active_fasttext_model(&self, model_name: &str) -> Result<(), PulseError> {
        let model_dir = self.config.data_dir.join("models").join(model_name);
        if !model_dir.join("fasttext.pftm").exists() {
            return Err(PulseError::NotFound(format!(
                "FastText model '{}' not found — run scripts/train_fasttext.py first",
                model_name
            )));
        }
        let ptr = self.config.data_dir.join("active_fasttext_model");
        std::fs::write(&ptr, model_name).map_err(|e| {
            PulseError::Config(format!("failed to write active_fasttext_model: {e}"))
        })?;
        Ok(())
    }

    pub fn set_active_miniml_model(&self, model_name: &str) -> Result<(), PulseError> {
        let model_dir = self.config.data_dir.join("models").join(model_name);
        let has_model = model_dir.join("model.onnx").exists()
            || model_dir.join("model_quantized.onnx").exists();
        if !has_model {
            return Err(PulseError::NotFound(format!(
                "MiniLM model '{}' not found — download model.onnx and run scripts/train_miniml.py first",
                model_name
            )));
        }
        let ptr = self.config.data_dir.join("active_miniml_model");
        std::fs::write(&ptr, model_name)
            .map_err(|e| PulseError::Config(format!("failed to write active_miniml_model: {e}")))?;
        Ok(())
    }

    pub fn reload_fasttext_tagger(&self) -> Result<(), PulseError> {
        let ptr = self.config.data_dir.join("active_fasttext_model");
        let model_name = std::fs::read_to_string(&ptr)
            .map_err(|e| PulseError::Config(format!("no active fasttext model set: {e}")))?;
        let model_dir = self.config.data_dir.join("models").join(model_name.trim());
        let tagger = FastTextTagger::load(&model_dir).map_err(PulseError::Tagging)?;
        self.fasttext_tagger.store(Arc::new(tagger));
        tracing::info!("FastText tagger hot-reloaded from {}", model_dir.display());
        Ok(())
    }

    pub fn reload_miniml_tagger(&self) -> Result<(), PulseError> {
        let ptr = self.config.data_dir.join("active_miniml_model");
        let model_name = std::fs::read_to_string(&ptr)
            .map_err(|e| PulseError::Config(format!("no active miniml model set: {e}")))?;
        let model_dir = self.config.data_dir.join("models").join(model_name.trim());
        let tagger = MiniMlTagger::load(&model_dir).map_err(PulseError::Tagging)?;
        self.miniml_tagger.store(Arc::new(tagger));
        tracing::info!("MiniLM tagger hot-reloaded from {}", model_dir.display());
        Ok(())
    }

    /// Unload the vision tagger from memory and clear the active-model pointer.
    pub fn remove_vision_model(&self, model_name: &str) -> Result<(), PulseError> {
        let model_dir = self.config.data_dir.join("models").join(model_name);
        if model_dir.exists() {
            std::fs::remove_dir_all(&model_dir)
                .map_err(|e| PulseError::Config(format!("failed to remove vision model: {e}")))?;
        }
        if self.active_vision_model_name().as_deref() == Some(model_name) {
            let _ = self.unset_active_vision_model();
        }
        self.vision_tagger.clear();
        Ok(())
    }

    /// Unload the MiniLM tagger from memory and clear the active-model pointer.
    pub fn remove_miniml_model(&self, model_name: &str) -> Result<(), PulseError> {
        let model_dir = self.config.data_dir.join("models").join(model_name);
        if model_dir.exists() {
            std::fs::remove_dir_all(&model_dir)
                .map_err(|e| PulseError::Config(format!("failed to remove miniml model: {e}")))?;
        }
        if self.active_miniml_model_name().as_deref() == Some(model_name) {
            let ptr = self.config.data_dir.join("active_miniml_model");
            let _ = std::fs::remove_file(&ptr);
        }
        self.miniml_tagger.clear();
        Ok(())
    }

    // ─── Stats ────────────────────────────────────────────────────────────────

    pub async fn get_db_stats(&self) -> Result<DbStats, PulseError> {
        self.db
            .with_reader(|pool| async move { storage::queries::get_db_stats(&pool).await })
            .await
            .map_err(PulseError::Storage)
    }
}
