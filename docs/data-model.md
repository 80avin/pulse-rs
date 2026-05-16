# Pulse — Data Model

## Overview

All data lives in a single SQLite database file. SQLite is configured with WAL (Write-Ahead Logging) mode for concurrent read access and `PRAGMA journal_mode=WAL` set on every connection open.

Database location:
- **Linux/macOS**: `$XDG_DATA_HOME/pulse/pulse.db` (falls back to `~/.local/share/pulse/pulse.db`)
- **Android**: app-private internal storage (`/data/data/com.avinthakur080.pulse_rs/files/pulse.db`)
- **Windows**: `%APPDATA%\pulse\pulse.db`

## Schema

### `feed_groups`

```sql
CREATE TABLE feed_groups (
    id          TEXT PRIMARY KEY,                -- UUIDv4
    name        TEXT NOT NULL UNIQUE,
    description TEXT,
    color       TEXT,                            -- hex color for UI (#rrggbb), nullable
    sort_order  INTEGER NOT NULL DEFAULT 0,
    created_at  INTEGER NOT NULL,                -- Unix timestamp (seconds)
    updated_at  INTEGER NOT NULL
);
```

### `feeds`

```sql
CREATE TABLE feeds (
    id                  TEXT PRIMARY KEY,        -- UUIDv4
    url                 TEXT NOT NULL UNIQUE,    -- canonical fetch URL
    feed_type           TEXT NOT NULL,           -- 'rss', 'hn', 'reddit'
    title               TEXT,                   -- feed title (from feed metadata)
    description         TEXT,
    site_url            TEXT,                   -- the human-readable website URL
    icon_url            TEXT,
    group_id            TEXT REFERENCES feed_groups(id) ON DELETE SET NULL,
    poll_interval_secs  INTEGER NOT NULL DEFAULT 3600,
    is_enabled          INTEGER NOT NULL DEFAULT 1,

    -- HTTP caching headers (for conditional requests)
    etag                TEXT,
    last_modified       TEXT,                   -- RFC 7231 date string

    -- Sync state
    last_fetched_at     INTEGER,                -- last attempted fetch (Unix ts)
    last_success_at     INTEGER,                -- last successful fetch
    last_item_at        INTEGER,                -- published_at of most recent item

    -- Health tracking
    failure_streak      INTEGER NOT NULL DEFAULT 0,
    total_fetches       INTEGER NOT NULL DEFAULT 0,
    total_failures      INTEGER NOT NULL DEFAULT 0,
    avg_latency_ms      REAL,                   -- EMA rolling average (α=0.2), nullable until first success
    next_fetch_at       INTEGER,                -- scheduled next fetch (Unix ts)

    -- Source-specific config (JSON blob)
    -- RSS: {}
    -- HN: {"section": "topstories"|"newstories"|"beststories"|"askhn"|"showhn"|"jobstories", "initial_limit": 30}
    -- Reddit: {"subreddit": "rust", "sort": "hot"|"new"|"top", "time": "day"|"week"|"month"}
    source_config       TEXT NOT NULL DEFAULT '{}',

    -- Language hint (ISO 639-1, e.g. "en", "de"). Used to skip AI rules for non-English feeds.
    -- NULL = unknown / auto-detect from feed <language> element on first fetch.
    language            TEXT,

    created_at          INTEGER NOT NULL,
    updated_at          INTEGER NOT NULL
);

CREATE INDEX idx_feeds_group_id ON feeds(group_id);
CREATE INDEX idx_feeds_next_fetch_at ON feeds(next_fetch_at) WHERE is_enabled = 1;
```

### `feed_items`

```sql
CREATE TABLE feed_items (
    id              TEXT PRIMARY KEY,            -- UUIDv5(namespace=feed_url, name=source_guid)
    feed_id         TEXT NOT NULL REFERENCES feeds(id) ON DELETE CASCADE,
    source_guid     TEXT NOT NULL,               -- original ID/GUID from the source
    title           TEXT NOT NULL,
    url             TEXT,                        -- link to original content
    author          TEXT,
    published_at    INTEGER NOT NULL,            -- Unix timestamp; normalization must always provide this, falling back to fetched_at
    fetched_at      INTEGER NOT NULL,            -- when we fetched this item
    body_text       TEXT,                        -- cleaned plaintext (for AI, search)
    body_html       TEXT,                        -- raw HTML (for reader view)
    word_count      INTEGER,                     -- approximate, for reading time estimates

    -- Engagement metadata (source-specific, nullable)
    score           INTEGER,                     -- Reddit upvotes or HN points
    comment_count   INTEGER,
    comment_url     TEXT,                        -- direct link to comment thread

    -- Additional source-specific metadata (JSON blob)
    -- RSS: {"categories": [...], "enclosure_url": "..."}
    -- HN: {"type": "story"|"ask"|"show"|"job", "kids": [12345, 67890]}
    -- Reddit: {"subreddit": "rust", "flair": "...", "is_self": true, "thumbnail_url": "..."}
    source_meta     TEXT NOT NULL DEFAULT '{}',

    UNIQUE(feed_id, source_guid)                -- prevents duplicates at DB level
);

CREATE INDEX idx_feed_items_feed_id ON feed_items(feed_id);
CREATE INDEX idx_feed_items_published_at ON feed_items(published_at DESC);
CREATE INDEX idx_feed_items_fetched_at ON feed_items(fetched_at DESC);
-- Composite index for timeline cursor pagination
CREATE INDEX idx_feed_items_timeline ON feed_items(published_at DESC, id);
```

### `item_states`

Separated from `feed_items` to keep the items table immutable after insert. State is always user-initiated mutation.

```sql
CREATE TABLE item_states (
    item_id     TEXT PRIMARY KEY REFERENCES feed_items(id) ON DELETE CASCADE,
    is_read     INTEGER NOT NULL DEFAULT 0,
    is_saved    INTEGER NOT NULL DEFAULT 0,
    is_hidden   INTEGER NOT NULL DEFAULT 0,
    read_at     INTEGER,
    saved_at    INTEGER,
    hidden_at   INTEGER,
    updated_at  INTEGER NOT NULL
);

CREATE INDEX idx_item_states_is_saved ON item_states(is_saved) WHERE is_saved = 1;
CREATE INDEX idx_item_states_is_hidden ON item_states(is_hidden) WHERE is_hidden = 1;
```

A row in `item_states` is created with defaults when a `feed_item` is inserted. This ensures every item always has a state row (no LEFT JOIN needed for read/saved/hidden status).

### `ai_tags`

```sql
CREATE TABLE ai_tags (
    id              TEXT PRIMARY KEY,           -- UUIDv4
    item_id         TEXT NOT NULL REFERENCES feed_items(id) ON DELETE CASCADE,
    tag             TEXT NOT NULL,              -- tag name (e.g., "technical", "clickbait")
    confidence      REAL NOT NULL,              -- 0.0 to 1.0
    tagger_source   TEXT NOT NULL,              -- 'rule' | 'model'
    rule_id         TEXT,                       -- which rule triggered (if tagger_source='rule')
    model_name      TEXT,                       -- which model (if tagger_source='model')
    model_version   TEXT,
    explanation     TEXT NOT NULL,              -- human-readable reason (mandatory)
    created_at      INTEGER NOT NULL
);

CREATE INDEX idx_ai_tags_item_id ON ai_tags(item_id);
CREATE INDEX idx_ai_tags_tag ON ai_tags(tag);
-- Enforce one tag per tag name per item per tagger source
CREATE UNIQUE INDEX idx_ai_tags_unique ON ai_tags(item_id, tag, tagger_source);
```

### `ai_models`

```sql
CREATE TABLE ai_models (
    id              TEXT PRIMARY KEY,           -- model name slug (e.g., 'mobilebert-q8')
    display_name    TEXT NOT NULL,
    model_url       TEXT NOT NULL,              -- HuggingFace download URL
    tokenizer_url   TEXT,
    file_path       TEXT,                       -- local path after download (nullable if not downloaded)
    file_size_bytes INTEGER,
    sha256          TEXT,                       -- expected checksum
    is_downloaded   INTEGER NOT NULL DEFAULT 0,
    is_active       INTEGER NOT NULL DEFAULT 0, -- only one model active at a time
    download_progress REAL,                     -- 0.0-1.0 during download, null otherwise
    created_at      INTEGER NOT NULL,
    downloaded_at   INTEGER
);
```

### `user_tags`

User-assigned tags (separate from AI tags).

```sql
CREATE TABLE user_tags (
    item_id     TEXT NOT NULL REFERENCES feed_items(id) ON DELETE CASCADE,
    tag         TEXT NOT NULL,
    created_at  INTEGER NOT NULL,
    PRIMARY KEY (item_id, tag)
);

CREATE INDEX idx_user_tags_tag ON user_tags(tag);
```

### `filter_rules`

User-configurable filter rules applied post-AI-tagging.

```sql
CREATE TABLE filter_rules (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    scope_type  TEXT NOT NULL DEFAULT 'global',  -- 'global' | 'group' | 'feed'
    scope_id    TEXT,                            -- NULL for 'global'; group.id or feed.id otherwise
    -- Note: scope_id is NOT a FK constraint because the referenced table depends on scope_type.
    -- Application code must clean up scope_id on group/feed deletion.
    action      TEXT NOT NULL,                   -- 'hide' | 'highlight' | 'tag:<name>'
    conditions  TEXT NOT NULL,                   -- JSON: [{field, op, value}, ...]
    is_enabled  INTEGER NOT NULL DEFAULT 1,
    sort_order  INTEGER NOT NULL DEFAULT 0,
    created_at  INTEGER NOT NULL
);
```

### FTS5 Virtual Table

```sql
CREATE VIRTUAL TABLE feed_items_fts USING fts5(
    item_id    UNINDEXED,       -- not searchable, used to join back to feed_items
    title,
    body_text,
    author,
    content    = 'feed_items',  -- content table for auto-population
    content_rowid = 'rowid',
    tokenize   = 'unicode61 remove_diacritics 1'
);

-- NOTE: FTS index is maintained from application code, NOT via INSERT trigger.
--
-- Reason: `INSERT OR IGNORE` fires the AFTER INSERT trigger even when the row is
-- ignored (duplicate). In that case, new.rowid is still populated but points to
-- the *existing* row's rowid — which may differ if the existing row was inserted
-- in a previous session. This produces phantom FTS entries that JOIN back to
-- wrong items (silent data corruption).
--
-- Correct approach: after INSERT OR IGNORE, check `conn.changes() == 1`. Only
-- call the FTS insert if the row was actually new:
--
--   conn.execute("INSERT OR IGNORE INTO feed_items (...) VALUES (...)", params)?;
--   if conn.changes() > 0 {
--       conn.execute(
--           "INSERT INTO feed_items_fts(rowid, item_id, title, body_text, author)
--            VALUES (last_insert_rowid(), ?, ?, ?, ?)",
--           params![item.id, item.title, item.body_text, item.author]
--       )?;
--   }
--
-- DELETE trigger is safe to keep as a DB-level trigger since deletions are
-- always deliberate (no OR IGNORE ambiguity):
CREATE TRIGGER feed_items_fts_delete AFTER DELETE ON feed_items BEGIN
    INSERT INTO feed_items_fts(feed_items_fts, rowid, item_id, title, body_text, author)
    VALUES ('delete', old.rowid, old.id, old.title, old.body_text, old.author);
END;

-- UPDATE trigger stub: feed_items are treated as immutable after insert.
-- If body_text is ever updated (e.g., full article fetch added), this trigger
-- MUST be implemented AND `fts5 rebuild` run on all existing rows.
-- The stub fires a RAISE to catch accidental updates during development:
CREATE TRIGGER feed_items_fts_update AFTER UPDATE OF body_text, title, author ON feed_items BEGIN
    SELECT RAISE(FAIL, 'feed_items FTS UPDATE trigger not implemented; run fts5 rebuild after implementing');
END;
```

We use `content=` mode to avoid duplicating data. The triggers keep the FTS index synchronized. Note: `UPDATE` triggers are omitted intentionally — `feed_items` rows are immutable after insert (only `item_states` changes). If body_text ever needs updating (e.g., fetching full article content), an UPDATE trigger must be added.

## ID Generation

### Feed IDs and Group IDs: UUIDv4

Random UUIDs generated at creation time. These are internal identifiers not derived from external data.

### Feed Item IDs: UUIDv5 (Deterministic)

```
namespace = UUID_v5(NAMESPACE_URL, feed.url)
item_id   = UUID_v5(namespace, source_guid)
```

This ensures:
- The same item always gets the same ID regardless of when it was fetched
- Upsert (`INSERT OR IGNORE`) is sufficient for deduplication — no SELECT-then-INSERT needed
- Item IDs are stable across database wipes and re-syncs (useful for bookmarks/saved items)

The `UNIQUE(feed_id, source_guid)` constraint provides a secondary deduplication safety net at the DB level.

## Feed Normalization

All three source types normalize to the same `FeedItem` struct.

### RSS/Atom (via `feed-rs`)

```
feed_rs::Entry.id           → source_guid (fallback: hash of link URL)
feed_rs::Entry.title        → title (strip HTML entities)
feed_rs::Entry.links[0].href → url
feed_rs::Entry.authors[0]   → author
feed_rs::Entry.published    → published_at
feed_rs::Entry.summary      → body_text (strip HTML tags)
feed_rs::Entry.content      → body_html
```

Edge cases:
- No `id` field: use SHA256 of the item URL as `source_guid`
- `published` missing: use `updated` if present, else use `fetched_at`
- Relative URLs: resolve against feed base URL

### Hacker News (Firebase JSON API)

HN's API is item-based. We fetch the list endpoint (e.g., `topstories.json`) to get up to 500 IDs, then fetch each item individually at `/v0/item/{id}.json`.

**Batching strategy**:
- **First sync**: fetch only the top N items from the list (default: 30, set via `source_config.initial_limit`). Fetching all 500 IDs at ~200ms each = 25+ seconds on first sync, which is unacceptable.
- **Subsequent syncs**: compare fetched IDs against `source_config.last_seen_id` (the max HN item ID seen so far). Only fetch item details for IDs > `last_seen_id`. Typical incremental sync fetches 0-15 new items.
- Concurrent requests per batch: 10.

```
hn_item.id           → source_guid (stringified integer)
hn_item.title        → title
hn_item.url          → url (null for Ask HN → use https://news.ycombinator.com/item?id={id})
hn_item.by           → author
hn_item.time         → published_at (Unix timestamp; always present — safe for NOT NULL)
hn_item.text         → body_html (Ask HN posts have body text)
hn_item.score        → score
hn_item.descendants  → comment_count
"https://news.ycombinator.com/item?id={id}" → comment_url
hn_item.type         → source_meta.type ("story"|"ask"|"show"|"job") — used by AI rules
```

### Reddit JSON API

Fetch `https://reddit.com/r/{subreddit}/{sort}.json?limit=100`. No auth required. Rate limit: ~60 req/min for unauthenticated IPs.

```
reddit_post.data.id          → source_guid
reddit_post.data.title       → title
reddit_post.data.url         → url (for link posts)
reddit_post.data.permalink   → comment_url ("https://reddit.com" + permalink)
reddit_post.data.author      → author
reddit_post.data.created_utc → published_at
reddit_post.data.selftext    → body_text (for self posts)
reddit_post.data.selftext_html → body_html
reddit_post.data.score       → score
reddit_post.data.num_comments → comment_count
```

For self posts (is_self=true), `url` is set to the Reddit post URL (`comment_url`).

## Migration Strategy

Migrations use `rusqlite_migration` with a version table. Migrations run in order on every startup; already-applied migrations are skipped.

```rust
// In pulse-core/src/storage/migrations.rs
const MIGRATIONS: &[Migration] = migrations![
    sql!("M0001_initial.sql"),
    sql!("M0002_add_filter_rules.sql"),
    // ...
];
```

Migration files live at `crates/pulse-core/migrations/`. Each file is embedded in the binary via `include_str!`.

Rules:
- Migrations are append-only (never edit an applied migration)
- Destructive changes require a new migration (never `DROP` in place)
- Each migration runs in a transaction; failure rolls back cleanly

## Key Query Patterns

### Timeline (with cursor pagination)

```sql
SELECT
    fi.id, fi.title, fi.url, fi.author, fi.published_at,
    fi.score, fi.comment_count, fi.word_count,
    f.title AS feed_title, f.feed_type,
    fg.name AS group_name,
    ist.is_read, ist.is_saved, ist.is_hidden,
    json_group_array(DISTINCT at.tag) AS ai_tags   -- JSON array avoids comma-in-tag ambiguity
FROM feed_items fi
JOIN feeds f ON fi.feed_id = f.id
LEFT JOIN feed_groups fg ON f.group_id = fg.id
JOIN item_states ist ON ist.item_id = fi.id
LEFT JOIN ai_tags at ON at.item_id = fi.id
WHERE
    ist.is_hidden = 0
    AND (? IS NULL OR f.group_id = ?)          -- group filter
    AND (? IS NULL OR ist.is_read = ?)          -- read filter
    -- Cursor pagination: items before the cursor
    AND (fi.published_at < ? OR (fi.published_at = ? AND fi.id < ?))
GROUP BY fi.id
ORDER BY fi.published_at DESC, fi.id DESC
LIMIT ?;
```

### Full-Text Search

```sql
SELECT fi.id, fi.title, fi.url, fi.published_at, f.title AS feed_title,
       rank
FROM feed_items_fts
JOIN feed_items fi ON fi.rowid = feed_items_fts.rowid
JOIN feeds f ON fi.feed_id = f.id
WHERE feed_items_fts MATCH ?
ORDER BY rank
LIMIT 50;
```

`rank` is FTS5's built-in BM25 ranking (lower is better; negate for DESC order).

### Feed Health Summary

```sql
SELECT
    id, title, url, feed_type,
    last_success_at, last_item_at, failure_streak,
    ROUND(CAST(total_fetches - total_failures AS REAL) / NULLIF(total_fetches, 0) * 100, 1) AS success_rate_pct,
    avg_latency_ms, next_fetch_at
FROM feeds
WHERE is_enabled = 1
ORDER BY success_rate_pct ASC, failure_streak DESC;
```

## SQLite Configuration

Applied on every connection open:

```sql
PRAGMA journal_mode = WAL;         -- concurrent readers during writes
-- synchronous: FULL on Android (process death = power failure equivalent),
--              NORMAL on desktop (acceptable tradeoff for ~30% write perf gain)
PRAGMA synchronous = FULL;         -- overridden at runtime based on platform
PRAGMA foreign_keys = ON;
PRAGMA busy_timeout = 5000;        -- wait up to 5s on SQLITE_BUSY before erroring
PRAGMA cache_size = -32768;        -- 32MB page cache (heap-based)
PRAGMA temp_store = MEMORY;
-- mmap_size: disabled on Android (256MB mmap inflates VSZ and can trigger OOM killer).
--            On desktop: 256MB is safe and improves read performance.
-- Set at runtime: Android → 0, Desktop → 268435456
PRAGMA mmap_size = 0;              -- overridden at runtime based on platform
```

`synchronous = NORMAL` with WAL is safe against OS crashes (not power failure, but acceptable for a non-critical app). If we want full durability, use `FULL`, accepting ~30% write performance penalty.
