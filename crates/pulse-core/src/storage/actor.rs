use crate::error::StorageError;
use crate::types::{Feed, FeedGroup, FeedId, FeedItem, ItemId, ItemStatePatch, TagResult};
use sqlx::SqlitePool;
use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;

type DbResult<T> = Result<T, StorageError>;

/// Commands sent to the DB writer actor
pub enum DbCommand {
    /// Upsert a batch of feed items (INSERT OR IGNORE + item_states + FTS)
    UpsertItems {
        items: Vec<FeedItem>,
        reply: oneshot::Sender<DbResult<usize>>,
    },

    /// Update the read/saved/hidden state of an item
    UpdateItemState {
        item_id: ItemId,
        patch: ItemStatePatch,
        reply: oneshot::Sender<DbResult<()>>,
    },

    /// Insert or update a feed
    UpsertFeed {
        feed: Feed,
        reply: oneshot::Sender<DbResult<()>>,
    },

    /// Insert a feed group
    InsertFeedGroup {
        group: FeedGroup,
        reply: oneshot::Sender<DbResult<()>>,
    },

    /// Update feed health after a sync
    UpdateFeedHealth {
        feed_id: FeedId,
        success: bool,
        latency_ms: Option<u64>,
        new_item_count: usize,
        etag: Option<String>,
        last_modified: Option<String>,
        last_item_at: Option<i64>,
        reply: oneshot::Sender<DbResult<()>>,
    },

    /// Store AI tags for an item
    InsertAiTags {
        item_id: ItemId,
        tags: Vec<TagResult>,
        reply: oneshot::Sender<DbResult<()>>,
    },

    /// Update the source_config JSON for a feed (e.g., last_seen_id for HN)
    UpdateFeedSourceConfig {
        feed_id: FeedId,
        source_config: serde_json::Value,
        reply: oneshot::Sender<DbResult<()>>,
    },

    /// Delete a feed (cascades to items, states, tags)
    DeleteFeed {
        feed_id: FeedId,
        reply: oneshot::Sender<DbResult<()>>,
    },

    /// Clear ETag, Last-Modified, and last_seen_id from source_config so the
    /// next sync performs a full re-fetch regardless of prior caching state.
    ClearFeedCache {
        feed_id: FeedId,
        reply: oneshot::Sender<DbResult<()>>,
    },

    /// Update an item's body_text and source_meta after enrichment.
    /// body_text is only written if the item currently has no body_text (COALESCE).
    /// source_meta is always updated via json_set.
    EnrichItem {
        item_id: ItemId,
        body_text: Option<String>,
        source_meta_patch: serde_json::Value,
        reply: oneshot::Sender<DbResult<()>>,
    },

    /// Delete a feed group (NULLs group_id on member feeds, then deletes group)
    DeleteFeedGroup {
        id: String,
        reply: oneshot::Sender<DbResult<()>>,
    },

    /// Delete all AI tags for a specific item (used by force-retag).
    DeleteItemTags {
        item_id: ItemId,
        reply: oneshot::Sender<DbResult<()>>,
    },

    /// Mark all items in a feed as read
    MarkFeedRead {
        feed_id: FeedId,
        reply: oneshot::Sender<DbResult<()>>,
    },


    /// VACUUM the database and rebuild the FTS index afterwards.
    /// VACUUM can renumber implicit rowids, which would silently corrupt the
    /// external-content FTS mapping, so the rebuild is mandatory.
    Vacuum { reply: oneshot::Sender<DbResult<()>> },
}

/// The DB writer actor task. Uses a single-connection pool to serialize writes.
pub async fn db_writer_task(mut rx: mpsc::Receiver<DbCommand>, pool: SqlitePool) {
    while let Some(cmd) = rx.recv().await {
        match cmd {
            DbCommand::UpsertItems { items, reply } => {
                let result = upsert_items(&pool, &items).await;
                let _ = reply.send(result);
            }

            DbCommand::UpdateItemState {
                item_id,
                patch,
                reply,
            } => {
                let result = update_item_state(&pool, &item_id, &patch).await;
                let _ = reply.send(result);
            }

            DbCommand::UpsertFeed { feed, reply } => {
                let result = upsert_feed(&pool, &feed).await;
                let _ = reply.send(result);
            }

            DbCommand::InsertFeedGroup { group, reply } => {
                let result = insert_feed_group(&pool, &group).await;
                let _ = reply.send(result);
            }

            DbCommand::UpdateFeedHealth {
                feed_id,
                success,
                latency_ms,
                new_item_count,
                etag,
                last_modified,
                last_item_at,
                reply,
            } => {
                let result = update_feed_health(
                    &pool,
                    &feed_id,
                    success,
                    latency_ms,
                    new_item_count,
                    etag,
                    last_modified,
                    last_item_at,
                )
                .await;
                let _ = reply.send(result);
            }

            DbCommand::InsertAiTags {
                item_id,
                tags,
                reply,
            } => {
                let result = insert_ai_tags(&pool, &item_id, &tags).await;
                let _ = reply.send(result);
            }

            DbCommand::UpdateFeedSourceConfig {
                feed_id,
                source_config,
                reply,
            } => {
                let result = update_feed_source_config(&pool, &feed_id, &source_config).await;
                let _ = reply.send(result);
            }

            DbCommand::DeleteFeed { feed_id, reply } => {
                let result = delete_feed(&pool, &feed_id).await;
                let _ = reply.send(result);
            }

            DbCommand::ClearFeedCache { feed_id, reply } => {
                let result = clear_feed_cache(&pool, &feed_id).await;
                let _ = reply.send(result);
            }

            DbCommand::EnrichItem {
                item_id,
                body_text,
                source_meta_patch,
                reply,
            } => {
                let result =
                    enrich_item(&pool, &item_id, body_text.as_deref(), &source_meta_patch).await;
                let _ = reply.send(result);
            }

            DbCommand::DeleteFeedGroup { id, reply } => {
                let result = delete_feed_group(&pool, &id).await;
                let _ = reply.send(result);
            }

            DbCommand::DeleteItemTags { item_id, reply } => {
                let result = sqlx::query("DELETE FROM ai_tags WHERE item_id = ?")
                    .bind(&item_id)
                    .execute(&pool)
                    .await
                    .map(|_| ())
                    .map_err(StorageError::Sqlite);
                let _ = reply.send(result);
            }

            DbCommand::MarkFeedRead { feed_id, reply } => {
                let result = mark_feed_read(&pool, &feed_id).await;
                let _ = reply.send(result);
            }

            DbCommand::Vacuum { reply } => {
                let result = vacuum_db(&pool).await;
                let _ = reply.send(result);
            }
        }
    }

    tracing::info!("DB writer actor shutting down");
}

async fn upsert_items(pool: &SqlitePool, items: &[FeedItem]) -> DbResult<usize> {
    let now = chrono::Utc::now().timestamp();
    let mut new_count = 0usize;

    let mut tx = pool.begin().await.map_err(StorageError::Sqlite)?;

    for item in items {
        let source_meta = serde_json::to_string(&item.source_meta)?;

        let result = sqlx::query(
            "INSERT OR IGNORE INTO feed_items
             (id, feed_id, source_guid, title, url, author, published_at, fetched_at,
              body_text, body_html, word_count, score, comment_count, comment_url, source_meta)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&item.id)
        .bind(&item.feed_id)
        .bind(&item.source_guid)
        .bind(&item.title)
        .bind(&item.url)
        .bind(&item.author)
        .bind(item.published_at)
        .bind(item.fetched_at)
        .bind(&item.body_text)
        .bind(&item.body_html)
        .bind(item.word_count)
        .bind(item.score)
        .bind(item.comment_count)
        .bind(&item.comment_url)
        .bind(&source_meta)
        .execute(&mut *tx)
        .await
        .map_err(StorageError::Sqlite)?;

        // Check if row was actually inserted
        if result.rows_affected() > 0 {
            new_count += 1;

            // Insert item_states in same transaction
            sqlx::query(
                "INSERT OR IGNORE INTO item_states (item_id, is_read, is_saved, is_hidden, updated_at)
                 VALUES (?, 0, 0, 0, ?)"
            )
            .bind(&item.id)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(StorageError::Sqlite)?;

            // Insert into FTS5 index (application-managed, not trigger)
            // We need the rowid of the just-inserted feed_item
            let rowid: Option<i64> =
                sqlx::query_scalar("SELECT rowid FROM feed_items WHERE id = ?")
                    .bind(&item.id)
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(StorageError::Sqlite)?;

            if let Some(rowid) = rowid {
                sqlx::query(
                    "INSERT INTO feed_items_fts(rowid, id, title, body_text, author)
                     VALUES (?, ?, ?, ?, ?)",
                )
                .bind(rowid)
                .bind(&item.id)
                .bind(&item.title)
                .bind(&item.body_text)
                .bind(&item.author)
                .execute(&mut *tx)
                .await
                .map_err(StorageError::Sqlite)?;
            }
        }
    }

    tx.commit().await.map_err(StorageError::Sqlite)?;
    Ok(new_count)
}

async fn update_item_state(
    pool: &SqlitePool,
    item_id: &ItemId,
    patch: &ItemStatePatch,
) -> DbResult<()> {
    let now = chrono::Utc::now().timestamp();

    // Ensure a state row exists (created lazily for items synced before this fix);
    // otherwise the UPDATEs below would silently no-op.
    sqlx::query(
        "INSERT INTO item_states (item_id, updated_at) VALUES (?, ?)
         ON CONFLICT(item_id) DO NOTHING",
    )
    .bind(item_id)
    .bind(now)
    .execute(pool)
    .await
    .map_err(StorageError::Sqlite)?;

    if let Some(r) = patch.is_read {
        sqlx::query(
            "UPDATE item_states SET is_read = ?,
             read_at = CASE WHEN ? = 1 THEN ? ELSE read_at END,
             updated_at = ? WHERE item_id = ?",
        )
        .bind(r as i64)
        .bind(r as i64)
        .bind(now)
        .bind(now)
        .bind(item_id)
        .execute(pool)
        .await
        .map_err(StorageError::Sqlite)?;
    }

    if let Some(s) = patch.is_saved {
        sqlx::query(
            "UPDATE item_states SET is_saved = ?,
             saved_at = CASE WHEN ? = 1 THEN ? ELSE saved_at END,
             updated_at = ? WHERE item_id = ?",
        )
        .bind(s as i64)
        .bind(s as i64)
        .bind(now)
        .bind(now)
        .bind(item_id)
        .execute(pool)
        .await
        .map_err(StorageError::Sqlite)?;
    }

    if let Some(h) = patch.is_hidden {
        sqlx::query(
            "UPDATE item_states SET is_hidden = ?,
             hidden_at = CASE WHEN ? = 1 THEN ? ELSE hidden_at END,
             updated_at = ? WHERE item_id = ?",
        )
        .bind(h as i64)
        .bind(h as i64)
        .bind(now)
        .bind(now)
        .bind(item_id)
        .execute(pool)
        .await
        .map_err(StorageError::Sqlite)?;
    }

    if let Some(n) = &patch.note {
        sqlx::query("UPDATE item_states SET note = ?, updated_at = ? WHERE item_id = ?")
            .bind(n.clone())
            .bind(now)
            .bind(item_id)
            .execute(pool)
            .await
            .map_err(StorageError::Sqlite)?;
    }

    Ok(())
}

async fn vacuum_db(pool: &SqlitePool) -> DbResult<()> {
    // Must run outside a transaction; the writer connection is a single,
    // otherwise-idle connection at this point.
    sqlx::raw_sql("VACUUM")
        .execute(pool)
        .await
        .map_err(StorageError::Sqlite)?;
    // VACUUM can renumber implicit rowids, which would silently break the
    // external-content FTS mapping (indexed by rowid). Rebuild the index so it
    // points at the correct rows again.
    sqlx::raw_sql("INSERT INTO feed_items_fts(feed_items_fts) VALUES('rebuild')")
        .execute(pool)
        .await
        .map_err(StorageError::Sqlite)?;
    Ok(())
}

async fn upsert_feed(pool: &SqlitePool, feed: &Feed) -> DbResult<()> {
    let source_config = serde_json::to_string(&feed.source_config)?;

    sqlx::query(
        "INSERT INTO feeds
         (id, url, feed_type, title, description, site_url, icon_url, group_id,
          poll_interval_secs, is_enabled, etag, last_modified, last_fetched_at,
          last_success_at, last_item_at, failure_streak, total_fetches, total_failures,
          avg_latency_ms, next_fetch_at, source_config, language, hue, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(id) DO UPDATE SET
             url = excluded.url,
             feed_type = excluded.feed_type,
             title = excluded.title,
             description = excluded.description,
             site_url = excluded.site_url,
             icon_url = excluded.icon_url,
             group_id = excluded.group_id,
             poll_interval_secs = excluded.poll_interval_secs,
             is_enabled = excluded.is_enabled,
             etag = excluded.etag,
             last_modified = excluded.last_modified,
             language = excluded.language,
             hue = excluded.hue,
             source_config = excluded.source_config,
             next_fetch_at = excluded.next_fetch_at,
             updated_at = excluded.updated_at",
    )
    .bind(&feed.id)
    .bind(&feed.url)
    .bind(feed.feed_type.as_str())
    .bind(&feed.title)
    .bind(&feed.description)
    .bind(&feed.site_url)
    .bind(&feed.icon_url)
    .bind(&feed.group_id)
    .bind(feed.poll_interval_secs)
    .bind(feed.is_enabled as i64)
    .bind(&feed.etag)
    .bind(&feed.last_modified)
    .bind(feed.last_fetched_at)
    .bind(feed.last_success_at)
    .bind(feed.last_item_at)
    .bind(feed.failure_streak)
    .bind(feed.total_fetches)
    .bind(feed.total_failures)
    .bind(feed.avg_latency_ms)
    .bind(feed.next_fetch_at)
    .bind(&source_config)
    .bind(&feed.language)
    .bind(feed.hue)
    .bind(feed.created_at)
    .bind(feed.updated_at)
    .execute(pool)
    .await
    .map_err(StorageError::Sqlite)?;

    Ok(())
}

async fn insert_feed_group(pool: &SqlitePool, group: &FeedGroup) -> DbResult<()> {
    sqlx::query(
        "INSERT INTO feed_groups (id, name, description, color, sort_order, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(id) DO UPDATE SET
             name = excluded.name,
             description = excluded.description,
             color = excluded.color,
             sort_order = excluded.sort_order,
             updated_at = excluded.updated_at",
    )
    .bind(&group.id)
    .bind(&group.name)
    .bind(&group.description)
    .bind(&group.color)
    .bind(group.sort_order)
    .bind(group.created_at)
    .bind(group.updated_at)
    .execute(pool)
    .await
    .map_err(StorageError::Sqlite)?;

    Ok(())
}

async fn update_feed_health(
    pool: &SqlitePool,
    feed_id: &FeedId,
    success: bool,
    latency_ms: Option<u64>,
    _new_item_count: usize,
    etag: Option<String>,
    last_modified: Option<String>,
    last_item_at: Option<i64>,
) -> DbResult<()> {
    let now = chrono::Utc::now().timestamp();

    if success {
        // EMA update for latency: new_avg = 0.2 * new + 0.8 * old
        if let Some(ms) = latency_ms {
            sqlx::query(
                "UPDATE feeds SET
                 last_fetched_at = ?,
                 last_success_at = ?,
                 avg_latency_ms = COALESCE(0.2 * ? + 0.8 * avg_latency_ms, ?),
                 last_item_at = CASE WHEN ? IS NOT NULL AND (last_item_at IS NULL OR last_item_at < ?) THEN ? ELSE last_item_at END,
                 failure_streak = 0,
                 total_fetches = total_fetches + 1,
                 etag = COALESCE(?, etag),
                 last_modified = COALESCE(?, last_modified),
                 updated_at = ?
                 WHERE id = ?"
            )
            .bind(now)
            .bind(now)
            .bind(ms as f64)
            .bind(ms as f64)
            .bind(last_item_at)
            .bind(last_item_at)
            .bind(last_item_at)
            .bind(&etag)
            .bind(&last_modified)
            .bind(now)
            .bind(feed_id)
            .execute(pool)
            .await
            .map_err(StorageError::Sqlite)?;
        } else {
            sqlx::query(
                "UPDATE feeds SET
                 last_fetched_at = ?,
                 last_success_at = ?,
                 last_item_at = CASE WHEN ? IS NOT NULL AND (last_item_at IS NULL OR last_item_at < ?) THEN ? ELSE last_item_at END,
                 failure_streak = 0,
                 total_fetches = total_fetches + 1,
                 etag = COALESCE(?, etag),
                 last_modified = COALESCE(?, last_modified),
                 updated_at = ?
                 WHERE id = ?"
            )
            .bind(now)
            .bind(now)
            .bind(last_item_at)
            .bind(last_item_at)
            .bind(last_item_at)
            .bind(&etag)
            .bind(&last_modified)
            .bind(now)
            .bind(feed_id)
            .execute(pool)
            .await
            .map_err(StorageError::Sqlite)?;
        }
    } else {
        sqlx::query(
            "UPDATE feeds SET
             last_fetched_at = ?,
             failure_streak = failure_streak + 1,
             total_fetches = total_fetches + 1,
             total_failures = total_failures + 1,
             updated_at = ?
             WHERE id = ?",
        )
        .bind(now)
        .bind(now)
        .bind(feed_id)
        .execute(pool)
        .await
        .map_err(StorageError::Sqlite)?;

        // Check if streak >= max and disable the feed
        let streak: i64 = sqlx::query_scalar("SELECT failure_streak FROM feeds WHERE id = ?")
            .bind(feed_id)
            .fetch_one(pool)
            .await
            .unwrap_or(0);

        // FIXME: Use config.max_failure_streak instead of hardcoded 10
        if streak >= 10 {
            sqlx::query("UPDATE feeds SET is_enabled = 0, updated_at = ? WHERE id = ?")
                .bind(now)
                .bind(feed_id)
                .execute(pool)
                .await
                .map_err(StorageError::Sqlite)?;
            tracing::warn!(feed_id = %feed_id, "Feed disabled after {} consecutive failures", streak);
        }
    }

    Ok(())
}

async fn insert_ai_tags(pool: &SqlitePool, item_id: &ItemId, tags: &[TagResult]) -> DbResult<()> {
    let now = chrono::Utc::now().timestamp();

    for tag in tags {
        let tag_id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO ai_tags (id, item_id, tag, confidence, tagger_source, rule_id,
                                  model_name, model_version, explanation, created_at)
             VALUES (?, ?, ?, ?, ?, ?, NULL, NULL, ?, ?)
             ON CONFLICT(item_id, tag, tagger_source) DO UPDATE SET
                 confidence = excluded.confidence,
                 explanation = excluded.explanation",
        )
        .bind(&tag_id)
        .bind(item_id)
        .bind(&tag.tag)
        .bind(tag.confidence as f64)
        .bind(tag.source.as_str())
        .bind(&tag.rule_id)
        .bind(&tag.explanation)
        .bind(now)
        .execute(pool)
        .await
        .map_err(StorageError::Sqlite)?;
    }

    Ok(())
}

async fn update_feed_source_config(
    pool: &SqlitePool,
    feed_id: &FeedId,
    source_config: &serde_json::Value,
) -> DbResult<()> {
    let config_str = serde_json::to_string(source_config)?;
    let now = chrono::Utc::now().timestamp();
    sqlx::query("UPDATE feeds SET source_config = ?, updated_at = ? WHERE id = ?")
        .bind(&config_str)
        .bind(now)
        .bind(feed_id)
        .execute(pool)
        .await
        .map_err(StorageError::Sqlite)?;
    Ok(())
}

async fn delete_feed(pool: &SqlitePool, feed_id: &FeedId) -> DbResult<()> {
    sqlx::query("DELETE FROM feeds WHERE id = ?")
        .bind(feed_id)
        .execute(pool)
        .await
        .map_err(StorageError::Sqlite)?;
    Ok(())
}

async fn clear_feed_cache(pool: &SqlitePool, feed_id: &FeedId) -> DbResult<()> {
    let now = chrono::Utc::now().timestamp();
    // Clear ETag + Last-Modified + last_seen_id from source_config in one statement.
    // json_remove strips last_seen_id; the rest are top-level columns.
    sqlx::query(
        "UPDATE feeds SET
            etag = NULL,
            last_modified = NULL,
            source_config = json_remove(source_config, '$.last_seen_id'),
            updated_at = ?
         WHERE id = ?",
    )
    .bind(now)
    .bind(feed_id)
    .execute(pool)
    .await
    .map_err(StorageError::Sqlite)?;
    Ok(())
}

async fn enrich_item(
    pool: &SqlitePool,
    item_id: &ItemId,
    body_text: Option<&str>,
    source_meta_patch: &serde_json::Value,
) -> DbResult<()> {
    // Merge patch fields into existing source_meta using json_set.
    // Build a dynamic json_set expression for each patch key.
    let patch_obj = match source_meta_patch.as_object() {
        Some(m) => m,
        None => return Ok(()),
    };

    if patch_obj.is_empty() {
        return Ok(());
    }

    // Build: json_set(source_meta, '$.k1', ?, '$.k2', ?, ...)
    let mut set_expr = String::from("json_set(source_meta");
    let mut bindings: Vec<String> = Vec::new();
    for (k, v) in patch_obj {
        set_expr.push_str(&format!(", '$.{}', ?", k));
        bindings.push(match v {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Null => "null".to_string(),
            other => other.to_string(),
        });
    }
    set_expr.push(')');

    // Two separate updates to avoid unnecessary FTS trigger when body_text is unchanged.
    // 1. Update source_meta always (no FTS trigger — trigger only watches body_text/title/author)
    let meta_sql = format!(
        "UPDATE feed_items SET source_meta = {} WHERE id = ?",
        set_expr
    );
    let mut q = sqlx::query(&meta_sql);
    for b in &bindings {
        q = q.bind(b);
    }
    q.bind(item_id)
        .execute(pool)
        .await
        .map_err(StorageError::Sqlite)?;

    // 2. Update body_text only if provided AND item currently has none (triggers FTS update)
    if let Some(bt) = body_text {
        sqlx::query("UPDATE feed_items SET body_text = ? WHERE id = ? AND body_text IS NULL")
            .bind(bt)
            .bind(item_id)
            .execute(pool)
            .await
            .map_err(StorageError::Sqlite)?;
    }

    Ok(())
}

async fn delete_feed_group(pool: &SqlitePool, id: &str) -> DbResult<()> {
    let now = chrono::Utc::now().timestamp();
    sqlx::query("UPDATE feeds SET group_id = NULL, updated_at = ? WHERE group_id = ?")
        .bind(now)
        .bind(id)
        .execute(pool)
        .await
        .map_err(StorageError::Sqlite)?;
    sqlx::query("DELETE FROM feed_groups WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .map_err(StorageError::Sqlite)?;
    Ok(())
}

async fn mark_feed_read(pool: &SqlitePool, feed_id: &str) -> DbResult<()> {
    let now = chrono::Utc::now().timestamp();
    sqlx::query(
        "UPDATE item_states SET is_read = 1, read_at = ?, updated_at = ?
         WHERE item_id IN (SELECT id FROM feed_items WHERE feed_id = ?)
           AND is_read = 0",
    )
    .bind(now)
    .bind(now)
    .bind(feed_id)
    .execute(pool)
    .await
    .map_err(StorageError::Sqlite)?;
    Ok(())
}

// ─── DbHandle ───────────────────────────────────────────────────────────────

/// A cloneable handle to the DB writer actor and a read pool.
#[derive(Clone)]
pub struct DbHandle {
    tx: mpsc::Sender<DbCommand>,
    reader_pool: SqlitePool,
}

impl DbHandle {
    pub fn new(tx: mpsc::Sender<DbCommand>, reader_pool: SqlitePool) -> Self {
        Self { tx, reader_pool }
    }

    /// Send a write command and await its reply
    async fn send<T>(
        &self,
        make_cmd: impl FnOnce(oneshot::Sender<DbResult<T>>) -> DbCommand,
    ) -> DbResult<T> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(make_cmd(reply_tx))
            .await
            .map_err(|_| StorageError::ActorDisconnected)?;
        reply_rx
            .await
            .map_err(|_| StorageError::ActorDisconnected)?
    }

    /// Upsert a batch of feed items; returns count of newly inserted items
    pub async fn upsert_items(&self, items: Vec<FeedItem>) -> DbResult<usize> {
        self.send(|reply| DbCommand::UpsertItems { items, reply })
            .await
    }

    /// Update item state
    pub async fn update_item_state(&self, item_id: ItemId, patch: ItemStatePatch) -> DbResult<()> {
        self.send(|reply| DbCommand::UpdateItemState {
            item_id,
            patch,
            reply,
        })
        .await
    }

    /// Upsert a feed record
    pub async fn upsert_feed(&self, feed: Feed) -> DbResult<()> {
        self.send(|reply| DbCommand::UpsertFeed { feed, reply })
            .await
    }

    /// Insert or update a feed group
    pub async fn insert_feed_group(&self, group: FeedGroup) -> DbResult<()> {
        self.send(|reply| DbCommand::InsertFeedGroup { group, reply })
            .await
    }

    /// Update feed health metrics after a sync attempt
    pub async fn update_feed_health(
        &self,
        feed_id: FeedId,
        success: bool,
        latency_ms: Option<u64>,
        new_item_count: usize,
        etag: Option<String>,
        last_modified: Option<String>,
        last_item_at: Option<i64>,
    ) -> DbResult<()> {
        self.send(|reply| DbCommand::UpdateFeedHealth {
            feed_id,
            success,
            latency_ms,
            new_item_count,
            etag,
            last_modified,
            last_item_at,
            reply,
        })
        .await
    }

    /// Store AI tags for an item
    pub async fn insert_ai_tags(&self, item_id: ItemId, tags: Vec<TagResult>) -> DbResult<()> {
        self.send(|reply| DbCommand::InsertAiTags {
            item_id,
            tags,
            reply,
        })
        .await
    }

    /// Delete all AI tags for an item (used before force-retag).
    pub async fn delete_item_tags(&self, item_id: ItemId) -> DbResult<()> {
        self.send(|reply| DbCommand::DeleteItemTags { item_id, reply })
            .await
    }

    /// Update the source_config for a feed (e.g., last_seen_id for HN)
    pub async fn update_feed_source_config(
        &self,
        feed_id: FeedId,
        source_config: serde_json::Value,
    ) -> DbResult<()> {
        self.send(|reply| DbCommand::UpdateFeedSourceConfig {
            feed_id,
            source_config,
            reply,
        })
        .await
    }

    /// Delete a feed and all its items
    pub async fn delete_feed(&self, feed_id: FeedId) -> DbResult<()> {
        self.send(|reply| DbCommand::DeleteFeed { feed_id, reply })
            .await
    }

    /// Clear ETag, Last-Modified, and last_seen_id so the next sync does a full re-fetch.
    pub async fn clear_feed_cache(&self, feed_id: FeedId) -> DbResult<()> {
        self.send(|reply| DbCommand::ClearFeedCache { feed_id, reply })
            .await
    }

    /// Update an item's body_text (if currently null) and merge source_meta fields.
    pub async fn enrich_item(
        &self,
        item_id: ItemId,
        body_text: Option<String>,
        source_meta_patch: serde_json::Value,
    ) -> DbResult<()> {
        self.send(|reply| DbCommand::EnrichItem {
            item_id,
            body_text,
            source_meta_patch,
            reply,
        })
        .await
    }

    /// Delete a feed group and null-out group_id on member feeds
    pub async fn delete_feed_group(&self, id: String) -> DbResult<()> {
        self.send(|reply| DbCommand::DeleteFeedGroup { id, reply })
            .await
    }

    /// Mark all items in a feed as read
    pub async fn mark_feed_read(&self, feed_id: FeedId) -> DbResult<()> {
        self.send(|reply| DbCommand::MarkFeedRead { feed_id, reply })
            .await
    }

    /// VACUUM the database (writer connection) and rebuild the FTS index.
    pub async fn vacuum(&self) -> DbResult<()> {
        self.send(|reply| DbCommand::Vacuum { reply }).await
    }

    /// Get reference to reader pool for read-only queries
    pub fn reader_pool(&self) -> &SqlitePool {
        &self.reader_pool
    }

    /// Run an async read operation using the read pool
    pub async fn with_reader<F, T, Fut>(&self, f: F) -> DbResult<T>
    where
        F: FnOnce(SqlitePool) -> Fut,
        Fut: std::future::Future<Output = DbResult<T>>,
    {
        f(self.reader_pool.clone()).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::PulseConfig;
    use crate::storage::connection::open_writer_pool;
    use crate::storage::migrations::run_migrations;
    use chrono::Utc;
    use uuid::Uuid;

    async fn test_pool() -> SqlitePool {
        let dir = std::env::temp_dir().join(format!("pulse-core-actor-test-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let config = PulseConfig::default().with_data_dir(dir.clone());
        let pool = open_writer_pool(&config.db_path, &config).await.unwrap();
        run_migrations(&pool).await.unwrap();
        pool
    }

    async fn seed_item(pool: &SqlitePool, item_id: &str, with_state: bool) {
        let now = Utc::now().timestamp();
        sqlx::query(
            "INSERT INTO feeds (id, url, feed_type, title, created_at, updated_at)
             VALUES ('f1', 'https://example.com/rss', 'rss', 'test', ?, ?)",
        )
        .bind(now)
        .bind(now)
        .execute(pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO feed_items (id, feed_id, source_guid, title, published_at, fetched_at)
             VALUES (?, 'f1', 'g1', 'title', ?, ?)",
        )
        .bind(item_id)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await
        .unwrap();
        if with_state {
            sqlx::query("INSERT INTO item_states (item_id, updated_at) VALUES (?, ?)")
                .bind(item_id)
                .bind(now)
                .execute(pool)
                .await
                .unwrap();
        }
    }

    async fn note_of(pool: &SqlitePool, item_id: &str) -> Option<String> {
        sqlx::query_scalar::<_, Option<String>>(
            "SELECT note FROM item_states WHERE item_id = ?",
        )
        .bind(item_id)
        .fetch_one(pool)
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn note_is_set_and_cleared() {
        let pool = test_pool().await;
        seed_item(&pool, "i1", true).await;

        update_item_state(
            &pool,
            &"i1".into(),
            &ItemStatePatch { note: Some(Some("keep me".into())), ..Default::default() },
        )
        .await
        .unwrap();
        assert_eq!(note_of(&pool, "i1").await.as_deref(), Some("keep me"));

        // Clearing: note = None clears the column (must not be a silent no-op).
        update_item_state(
            &pool,
            &"i1".into(),
            &ItemStatePatch { note: Some(None), ..Default::default() },
        )
        .await
        .unwrap();
        assert_eq!(note_of(&pool, "i1").await, None);
    }

    #[tokio::test]
    async fn saved_toggle_preserves_note() {
        let pool = test_pool().await;
        seed_item(&pool, "i1", true).await;
        update_item_state(
            &pool,
            &"i1".into(),
            &ItemStatePatch { note: Some(Some("keep me".into())), ..Default::default() },
        )
        .await
        .unwrap();

        // Mirrors PulseCore::toggle_saved: is_saved set, note untouched.
        update_item_state(
            &pool,
            &"i1".into(),
            &ItemStatePatch { is_saved: Some(true), note: None, ..Default::default() },
        )
        .await
        .unwrap();
        assert_eq!(note_of(&pool, "i1").await.as_deref(), Some("keep me"));

        update_item_state(
            &pool,
            &"i1".into(),
            &ItemStatePatch { is_saved: Some(false), note: None, ..Default::default() },
        )
        .await
        .unwrap();
        assert_eq!(note_of(&pool, "i1").await.as_deref(), Some("keep me"));
    }

    #[tokio::test]
    async fn missing_state_row_is_created_not_silently_dropped() {
        let pool = test_pool().await;
        seed_item(&pool, "i1", false).await;

        update_item_state(
            &pool,
            &"i1".into(),
            &ItemStatePatch { is_read: Some(true), ..Default::default() },
        )
        .await
        .unwrap();

        let read: i64 =
            sqlx::query_scalar("SELECT is_read FROM item_states WHERE item_id = ?")
                .bind("i1")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(read, 1);
    }
}

#[cfg(test)]
mod upsert_feed_tests {
    use super::*;
    use crate::config::PulseConfig;
    use crate::storage::connection::open_writer_pool;
    use crate::storage::migrations::run_migrations;
    use chrono::Utc;
    use sqlx::Row;
    use uuid::Uuid;

    fn feed(id: &str, url: &str) -> Feed {
        let now = Utc::now().timestamp();
        Feed {
            id: id.into(),
            url: url.into(),
            feed_type: crate::types::FeedType::Rss,
            title: Some("t".into()),
            description: None,
            site_url: None,
            icon_url: None,
            group_id: None,
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
            next_fetch_at: None,
            source_config: serde_json::json!({}),
            language: None,
            hue: None,
            created_at: now,
            updated_at: now,
        }
    }

    #[tokio::test]
    async fn upsert_feed_persists_url_and_feed_type_edits() {
        let dir = std::env::temp_dir().join(format!("pulse-core-feed-test-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let config = PulseConfig::default().with_data_dir(dir);
        let pool = open_writer_pool(&config.db_path, &config).await.unwrap();
        run_migrations(&pool).await.unwrap();

        upsert_feed(&pool, &feed("f1", "https://example.com/old")).await.unwrap();

        let mut edited = feed("f1", "https://example.com/new");
        edited.feed_type = crate::types::FeedType::Hn;
        edited.source_config = serde_json::json!({ "last_seen_id": 42 });
        upsert_feed(&pool, &edited).await.unwrap();

        let row = sqlx::query("SELECT url, feed_type, source_config FROM feeds WHERE id = 'f1'")
            .fetch_one(&pool)
            .await
            .unwrap();
        let url: String = row.get("url");
        let feed_type: String = row.get("feed_type");
        let source_config: String = row.get("source_config");
        assert_eq!(url, "https://example.com/new");
        assert_eq!(feed_type, "hn");
        assert_eq!(source_config, r#"{"last_seen_id":42}"#);
    }
}


#[cfg(test)]
mod vacuum_tests {
    use super::*;
    use crate::config::PulseConfig;
    use crate::storage::connection::open_writer_pool;
    use crate::storage::migrations::run_migrations;
    use chrono::Utc;
    use uuid::Uuid;

    async fn test_pool() -> SqlitePool {
        let dir = std::env::temp_dir().join(format!("pulse-core-vac-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let config = PulseConfig::default().with_data_dir(dir);
        let pool = open_writer_pool(&config.db_path, &config).await.unwrap();
        run_migrations(&pool).await.unwrap();
        pool
    }

    async fn seed_item(pool: &SqlitePool, item_id: &str, title: &str) {
        let now = Utc::now().timestamp();
        sqlx::query(
            "INSERT INTO feeds (id, url, feed_type, title, created_at, updated_at)
             VALUES ('f1', 'https://example.com/rss', 'rss', 'test', ?, ?)",
        )
        .bind(now)
        .bind(now)
        .execute(pool)
        .await
        .unwrap();
        let rowid: i64 = sqlx::query(
            "INSERT INTO feed_items (id, feed_id, source_guid, title, published_at, fetched_at, body_text)
             VALUES (?, 'f1', ?, ?, ?, ?, ?)",
        )
        .bind(item_id)
        .bind(item_id)
        .bind(title)
        .bind(now)
        .bind(now)
        .bind(title)
        .execute(pool)
        .await
        .unwrap()
        .last_insert_rowid();
        sqlx::query(
            "INSERT INTO feed_items_fts(rowid, id, title, body_text, author)
             VALUES (?, ?, ?, ?, '')",
        )
        .bind(rowid)
        .bind(item_id)
        .bind(title)
        .bind(title)
        .execute(pool)
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn vacuum_preserves_fts_search() {
        let pool = test_pool().await;
        seed_item(&pool, "i1", "Rust borrow checker deep dive").await;

        vacuum_db(&pool).await.unwrap();

        // Search must still hit the item after VACUUM + FTS rebuild.
        let found: Option<String> = sqlx::query_scalar(
            "SELECT fi.id FROM feed_items_fts
             JOIN feed_items fi ON fi.rowid = feed_items_fts.rowid
             WHERE feed_items_fts MATCH ?",
        )
        .bind("borrow")
        .fetch_optional(&pool)
        .await
        .unwrap();
        assert_eq!(found.as_deref(), Some("i1"));
    }
}
