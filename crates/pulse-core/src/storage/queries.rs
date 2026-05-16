use sqlx::SqlitePool;
use crate::error::StorageError;
use crate::types::{
    Feed, FeedGroup, FeedId, ItemId, FeedType, FeedItemView, FeedItem,
    TimelineCursor, TimelineFilter, TimelinePage, AiTag, TaggerSource, DbStats,
};
use std::str::FromStr;

fn row_to_feed(row: &sqlx::sqlite::SqliteRow) -> Result<Feed, StorageError> {
    use sqlx::Row;
    let feed_type_str: String = row.try_get("feed_type").map_err(StorageError::Sqlite)?;
    let source_config_str: String = row.try_get("source_config").map_err(StorageError::Sqlite)?;
    let is_enabled_i: i64 = row.try_get("is_enabled").map_err(StorageError::Sqlite)?;

    let feed_type = FeedType::from_str(&feed_type_str)
        .map_err(|e| StorageError::Migration(format!("Invalid feed_type: {e}")))?;
    let source_config: serde_json::Value = serde_json::from_str(&source_config_str)?;

    Ok(Feed {
        id: row.try_get("id").map_err(StorageError::Sqlite)?,
        url: row.try_get("url").map_err(StorageError::Sqlite)?,
        feed_type,
        title: row.try_get("title").map_err(StorageError::Sqlite)?,
        description: row.try_get("description").map_err(StorageError::Sqlite)?,
        site_url: row.try_get("site_url").map_err(StorageError::Sqlite)?,
        icon_url: row.try_get("icon_url").map_err(StorageError::Sqlite)?,
        group_id: row.try_get("group_id").map_err(StorageError::Sqlite)?,
        poll_interval_secs: row.try_get("poll_interval_secs").map_err(StorageError::Sqlite)?,
        is_enabled: is_enabled_i != 0,
        etag: row.try_get("etag").map_err(StorageError::Sqlite)?,
        last_modified: row.try_get("last_modified").map_err(StorageError::Sqlite)?,
        last_fetched_at: row.try_get("last_fetched_at").map_err(StorageError::Sqlite)?,
        last_success_at: row.try_get("last_success_at").map_err(StorageError::Sqlite)?,
        last_item_at: row.try_get("last_item_at").map_err(StorageError::Sqlite)?,
        failure_streak: row.try_get("failure_streak").map_err(StorageError::Sqlite)?,
        total_fetches: row.try_get("total_fetches").map_err(StorageError::Sqlite)?,
        total_failures: row.try_get("total_failures").map_err(StorageError::Sqlite)?,
        avg_latency_ms: row.try_get("avg_latency_ms").map_err(StorageError::Sqlite)?,
        next_fetch_at: row.try_get("next_fetch_at").map_err(StorageError::Sqlite)?,
        source_config,
        language: row.try_get("language").map_err(StorageError::Sqlite)?,
        created_at: row.try_get("created_at").map_err(StorageError::Sqlite)?,
        updated_at: row.try_get("updated_at").map_err(StorageError::Sqlite)?,
    })
}

/// Fetch all feeds from the database
pub async fn get_feeds(pool: &SqlitePool) -> Result<Vec<Feed>, StorageError> {
    let rows = sqlx::query(
        "SELECT id, url, feed_type, title, description, site_url, icon_url,
                group_id, poll_interval_secs, is_enabled, etag, last_modified,
                last_fetched_at, last_success_at, last_item_at, failure_streak,
                total_fetches, total_failures, avg_latency_ms, next_fetch_at,
                source_config, language, created_at, updated_at
         FROM feeds
         ORDER BY created_at ASC"
    )
    .fetch_all(pool)
    .await
    .map_err(StorageError::Sqlite)?;

    rows.iter().map(|row| row_to_feed(row)).collect()
}

/// Fetch a single feed by ID
pub async fn get_feed(pool: &SqlitePool, feed_id: &FeedId) -> Result<Feed, StorageError> {
    let row = sqlx::query(
        "SELECT id, url, feed_type, title, description, site_url, icon_url,
                group_id, poll_interval_secs, is_enabled, etag, last_modified,
                last_fetched_at, last_success_at, last_item_at, failure_streak,
                total_fetches, total_failures, avg_latency_ms, next_fetch_at,
                source_config, language, created_at, updated_at
         FROM feeds WHERE id = ?"
    )
    .bind(feed_id)
    .fetch_optional(pool)
    .await
    .map_err(StorageError::Sqlite)?;

    match row {
        Some(r) => row_to_feed(&r),
        None => Err(StorageError::NotFound { id: feed_id.clone() }),
    }
}

/// Fetch all feed groups
pub async fn get_feed_groups(pool: &SqlitePool) -> Result<Vec<FeedGroup>, StorageError> {
    let rows = sqlx::query(
        "SELECT id, name, description, color, sort_order, created_at, updated_at
         FROM feed_groups
         ORDER BY sort_order ASC, name ASC"
    )
    .fetch_all(pool)
    .await
    .map_err(StorageError::Sqlite)?;

    rows.iter().map(|row| {
        use sqlx::Row;
        Ok(FeedGroup {
            id: row.try_get("id").map_err(StorageError::Sqlite)?,
            name: row.try_get("name").map_err(StorageError::Sqlite)?,
            description: row.try_get("description").map_err(StorageError::Sqlite)?,
            color: row.try_get("color").map_err(StorageError::Sqlite)?,
            sort_order: row.try_get("sort_order").map_err(StorageError::Sqlite)?,
            created_at: row.try_get("created_at").map_err(StorageError::Sqlite)?,
            updated_at: row.try_get("updated_at").map_err(StorageError::Sqlite)?,
        })
    }).collect()
}

fn row_to_feed_item_view(row: &sqlx::sqlite::SqliteRow) -> Result<FeedItemView, StorageError> {
    use sqlx::Row;
    let feed_type_str: String = row.try_get("feed_type").map_err(StorageError::Sqlite)?;
    let ai_tags_json: String = row.try_get("ai_tags").map_err(StorageError::Sqlite)?;
    let is_read_i: i64 = row.try_get("is_read").map_err(StorageError::Sqlite)?;
    let is_saved_i: i64 = row.try_get("is_saved").map_err(StorageError::Sqlite)?;
    let is_hidden_i: i64 = row.try_get("is_hidden").map_err(StorageError::Sqlite)?;

    let feed_type = FeedType::from_str(&feed_type_str).unwrap_or(FeedType::Rss);
    let ai_tags: Vec<String> = serde_json::from_str(&ai_tags_json).unwrap_or_default();

    Ok(FeedItemView {
        id: row.try_get("id").map_err(StorageError::Sqlite)?,
        title: row.try_get("title").map_err(StorageError::Sqlite)?,
        url: row.try_get("url").map_err(StorageError::Sqlite)?,
        author: row.try_get("author").map_err(StorageError::Sqlite)?,
        published_at: row.try_get("published_at").map_err(StorageError::Sqlite)?,
        fetched_at: row.try_get("fetched_at").map_err(StorageError::Sqlite)?,
        word_count: row.try_get("word_count").map_err(StorageError::Sqlite)?,
        score: row.try_get("score").map_err(StorageError::Sqlite)?,
        comment_count: row.try_get("comment_count").map_err(StorageError::Sqlite)?,
        comment_url: row.try_get("comment_url").map_err(StorageError::Sqlite)?,
        feed_id: row.try_get("feed_id").map_err(StorageError::Sqlite)?,
        feed_title: row.try_get("feed_title").map_err(StorageError::Sqlite)?,
        feed_type,
        feed_url: row.try_get("feed_url").map_err(StorageError::Sqlite)?,
        group_id: row.try_get("group_id").map_err(StorageError::Sqlite)?,
        group_name: row.try_get("group_name").map_err(StorageError::Sqlite)?,
        is_read: is_read_i != 0,
        is_saved: is_saved_i != 0,
        is_hidden: is_hidden_i != 0,
        ai_tags,
    })
}

/// Fetch a page of timeline items
pub async fn get_timeline(
    pool: &SqlitePool,
    filter: &TimelineFilter,
    cursor: Option<&TimelineCursor>,
    limit: usize,
) -> Result<TimelinePage, StorageError> {
    let fetch_limit = (limit + 1) as i64;

    let cursor_ts = cursor.map(|c| c.published_at).unwrap_or(i64::MAX);
    let cursor_id = cursor.map(|c| c.id.clone()).unwrap_or_else(|| "\u{FFFF}".repeat(40));

    // Build WHERE clauses dynamically
    let mut conditions = vec![
        "ist.is_hidden = 0".to_string(),
        "(fi.published_at < ? OR (fi.published_at = ? AND fi.id < ?))".to_string(),
    ];

    if filter.group_id.is_some() {
        conditions.push("f.group_id = ?".to_string());
    }
    if filter.feed_id.is_some() {
        conditions.push("fi.feed_id = ?".to_string());
    }
    if filter.is_read.is_some() {
        conditions.push("ist.is_read = ?".to_string());
    }
    if filter.is_saved.is_some() {
        conditions.push("ist.is_saved = ?".to_string());
    }
    if filter.tag.is_some() {
        conditions.push("EXISTS (SELECT 1 FROM ai_tags _tf WHERE _tf.item_id = fi.id AND _tf.tag = ?)".to_string());
    }

    let where_clause = conditions.join(" AND ");

    let sql = format!(
        "SELECT
            fi.id, fi.title, fi.url, fi.author, fi.published_at, fi.fetched_at,
            fi.word_count, fi.score, fi.comment_count, fi.comment_url,
            f.id AS feed_id, f.title AS feed_title, f.feed_type, f.url AS feed_url,
            f.group_id, fg.name AS group_name,
            ist.is_read, ist.is_saved, ist.is_hidden,
            COALESCE(json_group_array(DISTINCT at.tag) FILTER (WHERE at.tag IS NOT NULL), '[]') AS ai_tags
         FROM feed_items fi
         JOIN feeds f ON fi.feed_id = f.id
         LEFT JOIN feed_groups fg ON f.group_id = fg.id
         JOIN item_states ist ON ist.item_id = fi.id
         LEFT JOIN ai_tags at ON at.item_id = fi.id
         WHERE {where_clause}
         GROUP BY fi.id
         ORDER BY fi.published_at DESC, fi.id DESC
         LIMIT ?"
    );

    let mut query = sqlx::query(&sql);

    // Bind cursor params
    query = query.bind(cursor_ts).bind(cursor_ts).bind(&cursor_id);

    // Bind filter params
    if let Some(ref g) = filter.group_id {
        query = query.bind(g.as_str());
    }
    if let Some(ref f) = filter.feed_id {
        query = query.bind(f.as_str());
    }
    if let Some(r) = filter.is_read {
        query = query.bind(r as i64);
    }
    if let Some(s) = filter.is_saved {
        query = query.bind(s as i64);
    }
    if let Some(ref t) = filter.tag {
        query = query.bind(t.as_str());
    }
    query = query.bind(fetch_limit);

    let rows = query.fetch_all(pool).await.map_err(StorageError::Sqlite)?;

    let mut items: Vec<FeedItemView> = rows.iter().filter_map(|row| {
        row_to_feed_item_view(row).ok()
    }).collect();

    let has_more = items.len() > limit;
    if has_more {
        items.truncate(limit);
    }

    let next_cursor = if has_more {
        items.last().map(|item| TimelineCursor {
            published_at: item.published_at,
            id: item.id.clone(),
        })
    } else {
        None
    };

    Ok(TimelinePage { items, next_cursor, has_more })
}

/// Fetch a single feed item by ID
pub async fn get_item(pool: &SqlitePool, item_id: &ItemId) -> Result<FeedItem, StorageError> {
    let row = sqlx::query(
        "SELECT id, feed_id, source_guid, title, url, author, published_at, fetched_at,
                body_text, body_html, word_count, score, comment_count, comment_url, source_meta
         FROM feed_items WHERE id = ?"
    )
    .bind(item_id)
    .fetch_optional(pool)
    .await
    .map_err(StorageError::Sqlite)?;

    match row {
        Some(r) => {
            use sqlx::Row;
            let source_meta_str: String = r.try_get("source_meta").map_err(StorageError::Sqlite)?;
            let source_meta: serde_json::Value = serde_json::from_str(&source_meta_str)?;
            Ok(FeedItem {
                id: r.try_get("id").map_err(StorageError::Sqlite)?,
                feed_id: r.try_get("feed_id").map_err(StorageError::Sqlite)?,
                source_guid: r.try_get("source_guid").map_err(StorageError::Sqlite)?,
                title: r.try_get("title").map_err(StorageError::Sqlite)?,
                url: r.try_get("url").map_err(StorageError::Sqlite)?,
                author: r.try_get("author").map_err(StorageError::Sqlite)?,
                published_at: r.try_get("published_at").map_err(StorageError::Sqlite)?,
                fetched_at: r.try_get("fetched_at").map_err(StorageError::Sqlite)?,
                body_text: r.try_get("body_text").map_err(StorageError::Sqlite)?,
                body_html: r.try_get("body_html").map_err(StorageError::Sqlite)?,
                word_count: r.try_get("word_count").map_err(StorageError::Sqlite)?,
                score: r.try_get("score").map_err(StorageError::Sqlite)?,
                comment_count: r.try_get("comment_count").map_err(StorageError::Sqlite)?,
                comment_url: r.try_get("comment_url").map_err(StorageError::Sqlite)?,
                source_meta,
            })
        }
        None => Err(StorageError::NotFound { id: item_id.clone() }),
    }
}

/// Resolve a full or prefix item ID to the canonical full UUID.
/// Returns `None` if no item matches.
pub async fn resolve_item_id(pool: &SqlitePool, prefix: &str) -> Result<Option<ItemId>, StorageError> {
    if prefix.len() == 36 {
        // Already a full UUID — verify it exists
        let row = sqlx::query_scalar::<_, String>(
            "SELECT id FROM feed_items WHERE id = ? LIMIT 1"
        )
        .bind(prefix)
        .fetch_optional(pool)
        .await
        .map_err(StorageError::Sqlite)?;
        return Ok(row);
    }
    let pattern = format!("{}%", prefix);
    let row = sqlx::query_scalar::<_, String>(
        "SELECT id FROM feed_items WHERE id LIKE ? LIMIT 1"
    )
    .bind(&pattern)
    .fetch_optional(pool)
    .await
    .map_err(StorageError::Sqlite)?;
    Ok(row)
}

/// Full-text search across feed items
pub async fn search_items(
    pool: &SqlitePool,
    query: &str,
    limit: usize,
) -> Result<Vec<FeedItemView>, StorageError> {
    let rows = sqlx::query(
        "SELECT fi.id, fi.title, fi.url, fi.author, fi.published_at, fi.fetched_at,
                fi.word_count, fi.score, fi.comment_count, fi.comment_url,
                f.id AS feed_id, f.title AS feed_title, f.feed_type, f.url AS feed_url,
                f.group_id, fg.name AS group_name,
                ist.is_read, ist.is_saved, ist.is_hidden,
                COALESCE(json_group_array(DISTINCT at.tag) FILTER (WHERE at.tag IS NOT NULL), '[]') AS ai_tags
         FROM feed_items_fts
         JOIN feed_items fi ON fi.rowid = feed_items_fts.rowid
         JOIN feeds f ON fi.feed_id = f.id
         LEFT JOIN feed_groups fg ON f.group_id = fg.id
         JOIN item_states ist ON ist.item_id = fi.id
         LEFT JOIN ai_tags at ON at.item_id = fi.id
         WHERE feed_items_fts MATCH ?
           AND ist.is_hidden = 0
         GROUP BY fi.id
         ORDER BY rank
         LIMIT ?"
    )
    .bind(query)
    .bind(limit as i64)
    .fetch_all(pool)
    .await
    .map_err(StorageError::Sqlite)?;

    rows.iter().map(|row| row_to_feed_item_view(row)).collect()
}

/// Get all AI tags for an item
pub async fn get_ai_tags(pool: &SqlitePool, item_id: &ItemId) -> Result<Vec<AiTag>, StorageError> {
    let rows = sqlx::query(
        "SELECT id, item_id, tag, confidence, tagger_source, rule_id, model_name,
                model_version, explanation, created_at
         FROM ai_tags WHERE item_id = ?
         ORDER BY confidence DESC"
    )
    .bind(item_id)
    .fetch_all(pool)
    .await
    .map_err(StorageError::Sqlite)?;

    rows.iter().map(|row| {
        use sqlx::Row;
        let source_str: String = row.try_get("tagger_source").map_err(StorageError::Sqlite)?;
        let tagger_source = TaggerSource::from_str(&source_str)
            .map_err(|e| StorageError::Migration(format!("Invalid tagger_source: {e}")))?;
        let confidence: f64 = row.try_get("confidence").map_err(StorageError::Sqlite)?;
        Ok(AiTag {
            id: row.try_get("id").map_err(StorageError::Sqlite)?,
            item_id: row.try_get("item_id").map_err(StorageError::Sqlite)?,
            tag: row.try_get("tag").map_err(StorageError::Sqlite)?,
            confidence: confidence as f32,
            tagger_source,
            rule_id: row.try_get("rule_id").map_err(StorageError::Sqlite)?,
            model_name: row.try_get("model_name").map_err(StorageError::Sqlite)?,
            model_version: row.try_get("model_version").map_err(StorageError::Sqlite)?,
            explanation: row.try_get("explanation").map_err(StorageError::Sqlite)?,
            created_at: row.try_get("created_at").map_err(StorageError::Sqlite)?,
        })
    }).collect()
}

/// Minimal item record returned for enrichment candidates
pub struct EnrichCandidate {
    pub id: ItemId,
    pub url: String,
    pub feed_id: String,
    pub body_text: Option<String>,
}

/// Return items that have not yet been enriched (no `enriched_at` in source_meta)
/// and have an external URL. Ordered newest-first.
pub async fn get_pending_enrichment(
    pool: &SqlitePool,
    feed_id_filter: Option<&str>,
    limit: usize,
) -> Result<Vec<EnrichCandidate>, StorageError> {
    let rows = if let Some(fid) = feed_id_filter {
        sqlx::query(
            "SELECT id, url, feed_id, body_text FROM feed_items
             WHERE url IS NOT NULL
               AND json_extract(source_meta, '$.enriched_at') IS NULL
               AND feed_id = ?
             ORDER BY published_at DESC
             LIMIT ?"
        )
        .bind(fid)
        .bind(limit as i64)
        .fetch_all(pool)
        .await
        .map_err(StorageError::Sqlite)?
    } else {
        sqlx::query(
            "SELECT id, url, feed_id, body_text FROM feed_items
             WHERE url IS NOT NULL
               AND json_extract(source_meta, '$.enriched_at') IS NULL
             ORDER BY published_at DESC
             LIMIT ?"
        )
        .bind(limit as i64)
        .fetch_all(pool)
        .await
        .map_err(StorageError::Sqlite)?
    };

    use sqlx::Row;
    rows.iter().map(|r| Ok(EnrichCandidate {
        id: r.try_get("id").map_err(StorageError::Sqlite)?,
        url: r.try_get("url").map_err(StorageError::Sqlite)?,
        feed_id: r.try_get("feed_id").map_err(StorageError::Sqlite)?,
        body_text: r.try_get("body_text").map_err(StorageError::Sqlite)?,
    })).collect()
}

/// Count items still pending enrichment
pub async fn count_pending_enrichment(
    pool: &SqlitePool,
    feed_id_filter: Option<&str>,
) -> Result<i64, StorageError> {
    if let Some(fid) = feed_id_filter {
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM feed_items
             WHERE url IS NOT NULL
               AND json_extract(source_meta, '$.enriched_at') IS NULL
               AND feed_id = ?"
        ).bind(fid).fetch_one(pool).await.map_err(StorageError::Sqlite)
    } else {
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM feed_items
             WHERE url IS NOT NULL
               AND json_extract(source_meta, '$.enriched_at') IS NULL"
        ).fetch_one(pool).await.map_err(StorageError::Sqlite)
    }
}

/// Get database statistics
pub async fn get_db_stats(pool: &SqlitePool) -> Result<DbStats, StorageError> {
    let feed_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM feeds WHERE is_enabled = 1"
    ).fetch_one(pool).await.map_err(StorageError::Sqlite)?;

    let item_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM feed_items"
    ).fetch_one(pool).await.map_err(StorageError::Sqlite)?;

    let unread_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM item_states WHERE is_read = 0 AND is_hidden = 0"
    ).fetch_one(pool).await.map_err(StorageError::Sqlite)?;

    let saved_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM item_states WHERE is_saved = 1"
    ).fetch_one(pool).await.map_err(StorageError::Sqlite)?;

    let tag_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM ai_tags"
    ).fetch_one(pool).await.map_err(StorageError::Sqlite)?;

    let db_size_bytes: i64 = sqlx::query_scalar(
        "SELECT page_count * page_size FROM pragma_page_count(), pragma_page_size()"
    ).fetch_one(pool).await.unwrap_or(0);

    Ok(DbStats {
        feed_count,
        item_count,
        unread_count,
        saved_count,
        tag_count,
        db_size_bytes,
    })
}
