use reqwest::Client;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, broadcast};
use tokio::task::JoinHandle;

use crate::ai::TaggerHandle;
use crate::error::SyncError;
use crate::feeds::RedditAuth;
use crate::storage::DbHandle;
use crate::storage::queries::{get_feed, get_feeds};
use crate::sync::health::compute_next_fetch;
use crate::types::{FeedId, FeedType};

const USER_AGENT: &str = "Pulse/0.1 (+https://github.com/80avin/pulse-rs; feed-reader)";

/// Commands that can be sent to the sync scheduler
#[derive(Debug, Clone)]
pub enum SyncCommand {
    RefreshFeed(FeedId),
    AddFeed(FeedId),
    RemoveFeed(FeedId),
    PauseAll,
    ResumeAll,
    Shutdown,
}

/// Manages per-feed sync tasks
pub struct SyncScheduler {
    db: DbHandle,
    tagger: TaggerHandle,
    http: Client,
    reddit_auth: Option<Arc<RedditAuth>>,
    cmd_tx: broadcast::Sender<SyncCommand>,
    tasks: Arc<Mutex<HashMap<FeedId, JoinHandle<()>>>>,
}

impl SyncScheduler {
    pub fn new(db: DbHandle, tagger: TaggerHandle, reddit_auth: Option<Arc<RedditAuth>>) -> Self {
        let (cmd_tx, _) = broadcast::channel(64);
        let http = Client::builder()
            .user_agent(USER_AGENT)
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            db,
            tagger,
            http,
            reddit_auth,
            cmd_tx,
            tasks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Start sync tasks for all enabled feeds (respects each feed's schedule).
    pub async fn start_all(&self) {
        let feeds = self
            .db
            .with_reader(|pool| async move { get_feeds(&pool).await })
            .await
            .unwrap_or_default();

        for feed in feeds.into_iter().filter(|f| f.is_enabled) {
            self.spawn_feed_task(feed.id, false).await;
        }
    }

    /// Whether a live sync task exists for the feed. A finished handle (task that
    /// exited naturally — feed disabled, load error, removal) counts as absent.
    pub async fn live_task(&self, feed_id: &FeedId) -> bool {
        let tasks = self.tasks.lock().await;
        match tasks.get(feed_id) {
            Some(handle) => !handle.is_finished(),
            None => false,
        }
    }

    /// Spawn a sync task for a specific feed. Replaces any existing handle for the
    /// feed (aborting it) under a single lock section, so duplicate tasks are
    /// impossible and stale handles never block a respawn.
    ///
    /// When `sync_immediately` is true the task performs one sync right away —
    /// used for refresh, add-feed, and re-enable flows.
    pub async fn spawn_feed_task(&self, feed_id: FeedId, sync_immediately: bool) {
        let db = self.db.clone();
        let tagger = self.tagger.clone();
        let http = self.http.clone();
        let reddit_auth = self.reddit_auth.clone();
        let cmd_rx = self.cmd_tx.subscribe();
        let feed_id_clone = feed_id.clone();

        let handle = tokio::spawn(async move {
            feed_sync_task(
                feed_id_clone,
                db,
                tagger,
                http,
                reddit_auth,
                cmd_rx,
                sync_immediately,
            )
            .await;
        });

        let mut tasks = self.tasks.lock().await;
        if let Some(prev) = tasks.insert(feed_id, handle) {
            prev.abort();
        }
    }

    pub fn send_command(&self, cmd: SyncCommand) {
        let _ = self.cmd_tx.send(cmd);
    }

    /// Expose the shared HTTP client for enrichment and other direct callers.
    pub fn http_client(&self) -> Client {
        self.http.clone()
    }

    pub async fn refresh_feed(&self, feed_id: FeedId) {
        if self.live_task(&feed_id).await {
            self.send_command(SyncCommand::RefreshFeed(feed_id));
        } else {
            self.spawn_feed_task(feed_id, true).await;
        }
    }

    pub async fn add_feed(&self, feed_id: FeedId) {
        if self.live_task(&feed_id).await {
            self.send_command(SyncCommand::AddFeed(feed_id));
        } else {
            self.spawn_feed_task(feed_id, true).await;
        }
    }

    pub async fn remove_feed(&self, feed_id: &FeedId) {
        self.send_command(SyncCommand::RemoveFeed(feed_id.clone()));
        if let Some(handle) = self.tasks.lock().await.remove(feed_id) {
            handle.abort();
        }
    }

    pub async fn shutdown(&self) {
        self.send_command(SyncCommand::Shutdown);
        let mut tasks = self.tasks.lock().await;
        for (_, handle) in tasks.drain() {
            handle.abort();
        }
    }

    /// Run a sync for a single feed directly (blocking) and return the new item count.
    /// Does not go through the scheduler task; safe to call from CLI for testing.
    pub async fn sync_feed_blocking(&self, feed_id: &FeedId) -> Result<usize, SyncError> {
        perform_sync(
            feed_id,
            &self.db,
            &self.tagger,
            &self.http,
            self.reddit_auth.as_deref(),
        )
        .await
    }
}

async fn feed_sync_task(
    feed_id: FeedId,
    db: DbHandle,
    tagger: TaggerHandle,
    http: Client,
    reddit_auth: Option<Arc<RedditAuth>>,
    mut cmd_rx: broadcast::Receiver<SyncCommand>,
    sync_immediately: bool,
) {
    tracing::debug!(feed_id = %feed_id, "Feed sync task started");

    // Immediate first sync for refresh/add/re-enable spawns. The schedule loop
    // below picks up the freshly computed next_fetch_at afterwards.
    if sync_immediately {
        match load_feed(&db, &feed_id).await {
            Some(feed) if feed.is_enabled => {
                if let Err(e) = perform_sync(&feed_id, &db, &tagger, &http, reddit_auth.as_deref()).await
                {
                    tracing::warn!(feed_id = %feed_id, error = %e, "Initial sync failed");
                }
            }
            _ => {}
        }
    }

    loop {
        let feed = match load_feed(&db, &feed_id).await {
            Some(f) => f,
            None => {
                // Transient read errors (e.g. a busy DB) must not permanently
                // kill the task. Back off briefly and try again. A genuinely
                // removed feed is aborted via the RemoveFeed command instead.
                tokio::select! {
                    _ = tokio::time::sleep(tokio::time::Duration::from_secs(30)) => {}
                    cmd = cmd_rx.recv() => {
                        match cmd {
                            Ok(SyncCommand::RemoveFeed(id)) if id == feed_id => break,
                            Ok(SyncCommand::Shutdown) => break,
                            _ => {}
                        }
                    }
                }
                continue;
            }
        };

        if !feed.is_enabled {
            tracing::info!(feed_id = %feed_id, "Feed is disabled; stopping sync task");
            break;
        }

        let now = chrono::Utc::now().timestamp();
        let delay_secs = feed
            .next_fetch_at
            .map(|next| (next - now).max(0) as u64)
            .unwrap_or(0);

        let delay = tokio::time::Duration::from_secs(delay_secs);

        tokio::select! {
            _ = tokio::time::sleep(delay) => {
                if let Err(e) = perform_sync(&feed_id, &db, &tagger, &http, reddit_auth.as_deref()).await {
                    tracing::warn!(feed_id = %feed_id, error = %e, "Sync failed");
                }
            }
            cmd = cmd_rx.recv() => {
                match cmd {
                    Ok(SyncCommand::RefreshFeed(id)) if id == feed_id => {
                        if let Err(e) = perform_sync(&feed_id, &db, &tagger, &http, reddit_auth.as_deref()).await {
                            tracing::warn!(feed_id = %feed_id, error = %e, "Sync refresh failed");
                        }
                    }
                    Ok(SyncCommand::RemoveFeed(id)) if id == feed_id => break,
                    Ok(SyncCommand::Shutdown) => break,
                    Ok(SyncCommand::PauseAll) => {
                        loop {
                            match cmd_rx.recv().await {
                                Ok(SyncCommand::ResumeAll) => break,
                                Ok(SyncCommand::Shutdown) => return,
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    tracing::debug!(feed_id = %feed_id, "Feed sync task stopped");
}

async fn load_feed(db: &DbHandle, feed_id: &FeedId) -> Option<crate::types::Feed> {
    let fid = feed_id.clone();
    match db
        .with_reader(|pool| async move { get_feed(&pool, &fid).await })
        .await
    {
        Ok(feed) => Some(feed),
        Err(e) => {
            tracing::error!(feed_id = %feed_id, "Failed to load feed: {}", e);
            None
        }
    }
}

/// Perform a single sync cycle for the given feed and return the number of new items.
pub(crate) async fn perform_sync(
    feed_id: &FeedId,
    db: &DbHandle,
    tagger: &TaggerHandle,
    http: &Client,
    reddit_auth: Option<&RedditAuth>,
) -> Result<usize, SyncError> {
    let start = std::time::Instant::now();

    let fid = feed_id.clone();
    let feed = db
        .with_reader(|pool| async move { get_feed(&pool, &fid).await })
        .await
        .map_err(SyncError::Storage)?;

    tracing::debug!(feed_id = %feed_id, feed_type = %feed.feed_type, "Starting sync");

    let result = match feed.feed_type {
        FeedType::Rss => sync_rss(db, tagger, http, &feed).await,
        FeedType::Hn => sync_hn(db, tagger, http, &feed).await,
        FeedType::Reddit => sync_reddit(db, tagger, http, &feed, reddit_auth).await,
    };

    let elapsed_ms = start.elapsed().as_millis() as u64;

    match result {
        Ok((new_items, was_cached, etag, last_modified, last_item_at, source_config_update)) => {
            if let Some(new_config) = source_config_update
                && let Err(e) = db
                    .update_feed_source_config(feed_id.clone(), new_config)
                    .await
            {
                tracing::warn!(feed_id = %feed_id, error = %e, "Failed to update source config");
            }

            if let Err(e) = db
                .update_feed_health(
                    feed_id.clone(),
                    true,
                    Some(elapsed_ms),
                    new_items,
                    etag,
                    last_modified,
                    last_item_at,
                )
                .await
            {
                tracing::warn!(feed_id = %feed_id, error = %e, "Failed to update health after successful sync");
            }

            tracing::info!(feed_id = %feed_id, new_items, was_cached, elapsed_ms, "Sync complete");

            // Update next_fetch_at
            let fid2 = feed_id.clone();
            if let Ok(updated_feed) = db
                .with_reader(|pool| async move { get_feed(&pool, &fid2).await })
                .await
            {
                let next_fetch = compute_next_fetch(&updated_feed);
                let mut feed_update = updated_feed;
                feed_update.next_fetch_at = Some(next_fetch);
                feed_update.updated_at = chrono::Utc::now().timestamp();
                if let Err(e) = db.upsert_feed(feed_update).await {
                    tracing::warn!(feed_id = %feed_id, error = %e, "Failed to upsert feed after sync");
                }
            }

            Ok(new_items)
        }
        Err(e) => {
            tracing::warn!(feed_id = %feed_id, error = %e, "Sync failed");
            if let Err(e2) = db
                .update_feed_health(feed_id.clone(), false, None, 0, None, None, None)
                .await
            {
                tracing::warn!(feed_id = %feed_id, error = %e2, "Failed to update health after sync failure");
            }
            // Schedule the retry with exponential backoff based on the NEW
            // failure streak. Without this, a failing feed hot-loops (delay 0)
            // until it exhausts its failure budget and disables itself.
            let fid3 = feed_id.clone();
            if let Ok(updated_feed) = db
                .with_reader(|pool| async move { get_feed(&pool, &fid3).await })
                .await
            {
                let next_fetch = compute_next_fetch(&updated_feed);
                let mut feed_update = updated_feed;
                feed_update.next_fetch_at = Some(next_fetch);
                feed_update.updated_at = chrono::Utc::now().timestamp();
                if let Err(e3) = db.upsert_feed(feed_update).await {
                    tracing::warn!(feed_id = %feed_id, error = %e3, "Failed to schedule backoff after sync failure");
                }
            }
            Err(e)
        }
    }
}

type SyncSuccess = (
    usize,
    bool,
    Option<String>,
    Option<String>,
    Option<i64>,
    Option<serde_json::Value>,
);

async fn sync_rss(
    db: &DbHandle,
    tagger: &TaggerHandle,
    http: &Client,
    feed: &crate::types::Feed,
) -> Result<SyncSuccess, SyncError> {
    let result = crate::feeds::fetch_rss(http, feed)
        .await
        .map_err(SyncError::Feed)?;

    if result.was_cached {
        return Ok((0, true, result.etag, result.last_modified, None, None));
    }

    let last_item_at = result.items.iter().map(|i| i.published_at).max();
    let feed_type = feed.feed_type.clone();
    let items = result.items.clone();

    let new_items = db
        .upsert_items(result.items)
        .await
        .map_err(SyncError::Storage)?;

    for item in &items {
        tagger.tag_item(item.id.clone(), feed_type.clone()).await;
    }

    Ok((
        new_items,
        false,
        result.etag,
        result.last_modified,
        last_item_at,
        None,
    ))
}

async fn sync_hn(
    db: &DbHandle,
    tagger: &TaggerHandle,
    http: &Client,
    feed: &crate::types::Feed,
) -> Result<SyncSuccess, SyncError> {
    let result = crate::feeds::fetch_hn(http, feed)
        .await
        .map_err(SyncError::Feed)?;

    if result.was_cached {
        return Ok((0, true, None, None, None, None));
    }

    let last_item_at = result.items.iter().map(|i| i.published_at).max();
    let items = result.items.clone();

    let new_items = db
        .upsert_items(result.items)
        .await
        .map_err(SyncError::Storage)?;

    for item in &items {
        tagger.tag_item(item.id.clone(), FeedType::Hn).await;
    }

    let source_config_update = if let Some(last_id) = result.last_seen_id {
        let mut config = feed.source_config.clone();
        if let Some(obj) = config.as_object_mut() {
            obj.insert("last_seen_id".to_string(), serde_json::json!(last_id));
        }
        Some(config)
    } else {
        None
    };

    Ok((
        new_items,
        false,
        None,
        None,
        last_item_at,
        source_config_update,
    ))
}

async fn sync_reddit(
    db: &DbHandle,
    tagger: &TaggerHandle,
    http: &Client,
    feed: &crate::types::Feed,
    auth: Option<&RedditAuth>,
) -> Result<SyncSuccess, SyncError> {
    let result = crate::feeds::fetch_reddit(http, feed, auth)
        .await
        .map_err(SyncError::Feed)?;

    if result.was_cached {
        return Ok((0, true, result.etag, result.last_modified, None, None));
    }

    let last_item_at = result.items.iter().map(|i| i.published_at).max();
    let items = result.items.clone();

    let new_items = db
        .upsert_items(result.items)
        .await
        .map_err(SyncError::Storage)?;

    for item in &items {
        tagger.tag_item(item.id.clone(), FeedType::Reddit).await;
    }

    Ok((
        new_items,
        false,
        result.etag,
        result.last_modified,
        last_item_at,
        None,
    ))
}
