pub mod ai;
pub mod config;
pub mod error;
pub mod feeds;
pub mod onboarding;
pub mod search;
pub mod storage;
pub mod sync;
pub mod timeline;
pub mod types;

use std::sync::Arc;
use tokio::sync::mpsc;

use crate::ai::tagger::TagRequest;
use crate::ai::tagger::process_tag_request;
use crate::ai::{
    RuleEngine, RulesTagger, TAGGER_QUEUE_SIZE, TaggerHandle, default_rules, tagger_task,
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
    AiTag, DbStats, EnrichItemResult, EnrichStats, EnrichStatus, Feed, FeedGroup, FeedId, FeedItem,
    FeedItemView, GroupOverview, ItemId, ItemStatePatch, PreviewItem, TimelineCounts,
    TimelineCursor, TimelineFilter, TimelinePage,
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
    pub async fn init(config: PulseConfig) -> Result<Self, PulseError> {
        let t0 = std::time::Instant::now();
        let config = Arc::new(config);

        if let Some(parent) = config.db_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| PulseError::Config(format!("Failed to create data dir: {e}")))?;
        }

        // Writer pool is a single connection; run migrations on it
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

        // Reader pool for concurrent reads (WAL)
        let reader_pool = open_reader_pool(&config.db_path, &config)
            .await
            .map_err(PulseError::Storage)?;
        tracing::info!(
            elapsed_ms = t0.elapsed().as_millis(),
            "coldstart: reader pool open"
        );

        let (writer_tx, writer_rx) = mpsc::channel::<crate::storage::actor::DbCommand>(128);
        let writer_pool_for_actor = writer_pool.clone();
        tokio::spawn(async move {
            db_writer_task(writer_rx, writer_pool_for_actor).await;
        });

        let db = DbHandle::new(writer_tx, reader_pool);
        tracing::info!(elapsed_ms = t0.elapsed().as_millis(), "coldstart: DB ready");

        let (tagger_tx, tagger_rx) = mpsc::channel(TAGGER_QUEUE_SIZE);
        let tagger_handle = TaggerHandle::new(tagger_tx);

        let rule_engine = Arc::new(RuleEngine::new(default_rules()));
        let tagger: Arc<dyn crate::ai::Tagger> = Arc::new(RulesTagger::new(rule_engine.clone()));
        let db_for_tagger = db.clone();
        let tagger_for_task = tagger.clone();
        tokio::spawn(async move {
            tagger_task(tagger_rx, db_for_tagger, tagger_for_task).await;
        });

        // Reddit OAuth2 (only when credentials are configured)
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
        })
    }

    /// Run a sync for a single feed, awaiting completion. Returns new item count.
    pub async fn sync_feed(&self, feed_id: &FeedId) -> Result<usize, PulseError> {
        self.scheduler
            .sync_feed_blocking(feed_id)
            .await
            .map_err(PulseError::Sync)
    }

    /// Start syncing all enabled feeds in the background
    pub async fn start_sync(&self) {
        self.scheduler.start_all().await;
    }

    /// Shut down all background tasks
    pub async fn shutdown(&self) {
        self.scheduler.shutdown().await;
    }

    // ─── Enrichment ───────────────────────────────────────────────────────────

    /// Enrich pending items (fetch OG metadata for link posts). Returns stats.
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

            // Errors stay unmarked so they retry next run; only definitive
            // results (ok/image/skipped) write enriched_at.
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
            if let Err(e) = db
                .enrich_item(result.item_id.clone(), body_text, patch)
                .await
            {
                tracing::warn!(item_id = %result.item_id, error = %e, "Failed to write enrichment result");
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

    /// Bulk-add curated feeds, auto-creating (or reusing) their category group.
    /// Returns the number of feeds added.
    pub async fn add_onboard_feeds(
        &self,
        selections: &[crate::onboarding::OnboardSelection],
    ) -> Result<usize, PulseError> {
        use std::collections::HashMap;
        let existing_groups = self.get_feed_groups().await?;
        let mut group_ids: HashMap<String, String> = existing_groups
            .into_iter()
            .map(|g| (g.name.clone(), g.id))
            .collect();
        // Skip feeds the user already subscribes to (feeds.url is UNIQUE); re-adding
        // would abort the whole batch on a constraint error.
        let subscribed: std::collections::HashSet<String> =
            self.get_feeds().await?.into_iter().map(|f| f.url).collect();
        let mut added = 0usize;
        for sel in selections {
            if subscribed.contains(&sel.url) {
                continue;
            }
            let group_id = if let Some(id) = group_ids.get(&sel.category) {
                id.clone()
            } else {
                let gid = uuid::Uuid::new_v4().to_string();
                let now = chrono::Utc::now().timestamp();
                let group = crate::types::FeedGroup {
                    id: gid.clone(),
                    name: sel.category.clone(),
                    description: None,
                    color: None,
                    sort_order: 0,
                    created_at: now,
                    updated_at: now,
                };
                self.db
                    .insert_feed_group(group)
                    .await
                    .map_err(PulseError::Storage)?;
                group_ids.insert(sel.category.clone(), gid.clone());
                gid
            };
            let now = chrono::Utc::now().timestamp();
            let feed = crate::types::Feed {
                id: uuid::Uuid::new_v4().to_string(),
                url: sel.url.clone(),
                feed_type: sel.kind.clone(),
                title: Some(sel.name.clone()),
                description: None,
                site_url: None,
                icon_url: None,
                group_id: Some(group_id),
                poll_interval_secs: 3600,
                is_enabled: true,
                etag: None,
                last_modified: None,
                last_fetched_at: None,
                last_success_at: None,
                last_item_at: None,
                failure_streak: 0,
                total_fetches: 0,
                total_failures: 0,
                avg_latency_ms: None,
                next_fetch_at: Some(now),
                source_config: serde_json::json!({}),
                language: None,
                hue: None,
                created_at: now,
                updated_at: now,
            };
            self.add_feed(feed).await?;
            added += 1;
        }
        Ok(added)
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

    /// Fetch a single item as a fully-joined view (feed + group + state + tags),
    /// with no recency cap.
    pub async fn get_item_view(
        &self,
        item_id: &ItemId,
    ) -> Result<crate::types::FeedItemView, PulseError> {
        let iid = item_id.clone();
        self.db
            .with_reader(|pool| async move { storage::queries::get_item_view(&pool, &iid).await })
            .await
            .map_err(PulseError::Storage)
    }

    /// Clear the ETag, Last-Modified, and source_config cache keys so the next
    /// sync performs a full re-fetch regardless of prior state.
    pub async fn clear_feed_cache(&self, feed_id: &FeedId) -> Result<(), PulseError> {
        self.db
            .clear_feed_cache(feed_id.clone())
            .await
            .map_err(PulseError::Storage)
    }

    /// Fetch a feed's current items without subscribing (discover preview).
    /// The temp feed is never persisted; only the fetcher's URL/type/etag are
    /// read, so a transient `Feed` with default health fields is sufficient.
    pub async fn preview_feed(
        &self,
        url: &str,
        kind: crate::types::FeedType,
        limit: usize,
    ) -> Result<Vec<PreviewItem>, PulseError> {
        let now = chrono::Utc::now().timestamp();
        let temp_feed = Feed {
            id: uuid::Uuid::new_v4().to_string(),
            url: url.to_string(),
            feed_type: kind,
            title: None,
            description: None,
            site_url: None,
            icon_url: None,
            group_id: None,
            poll_interval_secs: 0,
            is_enabled: true,
            etag: None,
            last_modified: None,
            last_fetched_at: None,
            last_success_at: None,
            last_item_at: None,
            failure_streak: 0,
            total_fetches: 0,
            total_failures: 0,
            avg_latency_ms: None,
            next_fetch_at: None,
            source_config: serde_json::json!({ "initial_limit": limit }),
            language: None,
            hue: None,
            created_at: now,
            updated_at: now,
        };

        let http = self.scheduler.http_client();

        let items = match temp_feed.feed_type {
            crate::types::FeedType::Rss => crate::feeds::fetch_rss(&http, &temp_feed)
                .await
                .map(|r| r.items)?,
            crate::types::FeedType::Hn => crate::feeds::fetch_hn(&http, &temp_feed)
                .await
                .map(|r| r.items)?,
            crate::types::FeedType::Reddit => {
                let Some(auth) = self.scheduler.reddit_auth() else {
                    return Err(PulseError::Config(
                        "reddit preview needs REDDIT_CLIENT_ID and REDDIT_CLIENT_SECRET to be configured"
                            .to_string(),
                    ));
                };
                crate::feeds::fetch_reddit(&http, &temp_feed, Some(auth))
                    .await
                    .map(|r| r.items)?
            }
        };

        Ok(items.iter().take(limit).map(preview_item).collect())
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

    /// Counts for a timeline view (total/unread/saved), scoped by filter.
    pub async fn get_timeline_counts(
        &self,
        filter: TimelineFilter,
    ) -> Result<TimelineCounts, PulseError> {
        self.db
            .with_reader(|pool| async move {
                storage::queries::get_timeline_counts(&pool, &filter).await
            })
            .await
            .map_err(PulseError::Storage)
    }

    /// Per-group recent items for the overview screen. Groups with no items
    /// (no feeds or nothing synced) are skipped.
    pub async fn get_overview(&self, limit: usize) -> Result<Vec<GroupOverview>, PulseError> {
        let groups = self.get_feed_groups().await?;
        let mut out = Vec::with_capacity(groups.len());
        for g in groups {
            let counts = self
                .get_timeline_counts(TimelineFilter {
                    group_id: Some(g.id.clone()),
                    ..Default::default()
                })
                .await?;
            if counts.total == 0 {
                continue;
            }
            let page = self
                .get_timeline_page(
                    TimelineFilter {
                        group_id: Some(g.id.clone()),
                        ..Default::default()
                    },
                    None,
                    limit,
                )
                .await?;
            out.push(GroupOverview {
                group_id: g.id,
                group_name: g.name,
                total_items: counts.total,
                unread_count: counts.unread,
                items: page.items,
            });
        }
        Ok(out)
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

    /// Set (or clear) the user note on an item. `note = None` clears it;
    /// the saved state is never touched.
    pub async fn set_item_note(
        &self,
        item_id: &ItemId,
        note: Option<String>,
    ) -> Result<(), PulseError> {
        self.update_item_state(
            item_id,
            ItemStatePatch {
                note: Some(note),
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
        sort: &str,
    ) -> Result<Vec<FeedItemView>, PulseError> {
        self.search
            .search(query, limit, sort)
            .await
            .map_err(PulseError::Storage)
    }

    // ─── AI tags ──────────────────────────────────────────────────────────────

    /// Tag items directly without the async queue. Returns `(items_processed, tags_created)`.
    ///
    /// `force = true` deletes existing tags first so vocabulary changes are fully applied;
    /// `force = false` only processes items with no tags.
    /// `on_progress(tagged, total)` fires after each item.
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

        let tagger: Arc<dyn crate::ai::Tagger> =
            Arc::new(RulesTagger::new(self.rule_engine.clone()));

        for (item, feed_type) in work {
            if force {
                // Drop stale tags so removed/renamed tags don't persist
                let _ = self.db.delete_item_tags(item.id.clone()).await;
            }
            let req = TagRequest {
                item_id: item.id.clone(),
                feed_type,
            };
            match process_tag_request(&self.db, tagger.as_ref(), &req).await {
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

    pub async fn get_item_tags(&self, item_id: &ItemId) -> Result<Vec<AiTag>, PulseError> {
        let iid = item_id.clone();
        self.db
            .with_reader(|pool| async move { storage::queries::get_ai_tags(&pool, &iid).await })
            .await
            .map_err(PulseError::Storage)
    }

    pub async fn get_user_tags(&self, item_id: &ItemId) -> Result<Vec<String>, PulseError> {
        let iid = item_id.clone();
        self.db
            .with_reader(|pool| async move { storage::queries::get_user_tags(&pool, &iid).await })
            .await
            .map_err(PulseError::Storage)
    }

    /// Replace the user-defined tags for an item (full-set semantics).
    pub async fn set_user_tags(
        &self,
        item_id: &ItemId,
        tags: Vec<String>,
    ) -> Result<(), PulseError> {
        self.db
            .replace_user_tags(item_id.clone(), tags)
            .await
            .map_err(PulseError::Storage)
    }

    // ─── Stats ────────────────────────────────────────────────────────────────

    pub async fn get_db_stats(&self) -> Result<DbStats, PulseError> {
        self.db
            .with_reader(|pool| async move { storage::queries::get_db_stats(&pool).await })
            .await
            .map_err(PulseError::Storage)
    }

    pub async fn get_tag_stats(&self) -> Result<storage::queries::TagStats, PulseError> {
        self.db
            .with_reader(|pool| async move { storage::queries::get_tag_stats(&pool).await })
            .await
            .map_err(PulseError::Storage)
    }
}

/// Map a fetched `FeedItem` to a transient preview row. The id only needs to
/// be unique per list (url + timestamp); it is not persisted.
pub(crate) fn preview_item(item: &FeedItem) -> PreviewItem {
    PreviewItem {
        id: format!(
            "{}-{}",
            item.url.as_deref().unwrap_or(""),
            item.published_at
        ),
        title: item.title.clone(),
        url: item.url.clone(),
        author: item.author.clone(),
        published_at: Some(item.published_at),
    }
}
