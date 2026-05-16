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

use crate::ai::{RuleEngine, TaggerHandle, default_rules, tagger_task, TAGGER_QUEUE_SIZE};
use crate::ai::tagger::process_tag_request;
use crate::ai::tagger::TagRequest;
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
    AiTag, DbStats,
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

        // Spawn the AI tagger task
        let (tagger_tx, tagger_rx) = mpsc::channel(TAGGER_QUEUE_SIZE);
        let tagger_handle = TaggerHandle::new(tagger_tx);

        let rule_engine = Arc::new(RuleEngine::new(default_rules()));
        let db_for_tagger = db.clone();
        let engine_for_task = rule_engine.clone();
        tokio::spawn(async move {
            tagger_task(tagger_rx, db_for_tagger, engine_for_task).await;
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
                match process_tag_request(&self.db, &self.rule_engine, &req).await {
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

    // ─── Stats ────────────────────────────────────────────────────────────────

    pub async fn get_db_stats(&self) -> Result<DbStats, PulseError> {
        self.db.with_reader(|pool| async move {
            storage::queries::get_db_stats(&pool).await
        }).await.map_err(PulseError::Storage)
    }
}
