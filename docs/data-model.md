# Pulse — Data Model

## Overview

All data lives in a single SQLite database file. SQLite is configured with WAL (Write-Ahead Logging) mode, a **single-connection writer pool** for all mutations, and a **read-only reader pool** (up to 3 connections) for queries.

Database location (`PulseConfig::platform_data_dir()`):
- **Linux/macOS**: `$XDG_DATA_HOME/pulse/pulse.db` (fallback `~/.local/share/pulse/pulse.db`)
- **Windows**: `%APPDATA%\pulse\pulse.db`
- **Android**: app-private internal storage (keyed to package ID; survives APK updates)

## Schema

The schema below reflects migrations `M0001`–`M0006`. It is created by `crates/pulse-core/migrations/M0001_initial.sql` plus the ALTERs in `M0002`–`M0005`; `M0006` is a programmatic data migration (relative-URL backfill) recorded in `schema_migrations`.

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

Groups are the on-boarding categories: `add_onboard_feeds` creates (or reuses) one group per catalog category, and the UI synthesizes an `all` pseudo-group on top.

### `feeds`

```sql
CREATE TABLE feeds (
    id                  TEXT PRIMARY KEY,        -- UUIDv4
    url                 TEXT NOT NULL UNIQUE,    -- canonical fetch URL
    feed_type           TEXT NOT NULL,           -- 'rss', 'hn', 'reddit'
    title               TEXT,                    -- feed title (user-set or from metadata)
    description         TEXT,
    site_url            TEXT,                    -- the human-readable website URL
    icon_url            TEXT,
    group_id            TEXT REFERENCES feed_groups(id) ON DELETE SET NULL,
    poll_interval_secs  INTEGER NOT NULL DEFAULT 3600,
    is_enabled          INTEGER NOT NULL DEFAULT 1,

    -- HTTP caching headers (for conditional requests)
    etag                TEXT,
    last_modified       TEXT,                    -- RFC 7231 date string

    -- Sync state
    last_fetched_at     INTEGER,                 -- last attempted fetch (Unix ts)
    last_success_at     INTEGER,                 -- last successful fetch
    last_item_at        INTEGER,                 -- published_at of most recent item

    -- Health tracking
    failure_streak      INTEGER NOT NULL DEFAULT 0,
    total_fetches       INTEGER NOT NULL DEFAULT 0,
    total_failures      INTEGER NOT NULL DEFAULT 0,
    avg_latency_ms      REAL,                    -- EMA rolling average (α=0.2), nullable until first success
    next_fetch_at       INTEGER,                 -- scheduled next fetch (Unix ts)

    -- Source-specific config (JSON blob)
    -- RSS: {}
    -- HN: {"section": "...", "initial_limit": 30, "last_seen_id": 12345}
    -- Reddit: {"subreddit": "rust", "sort": "hot"|"new"|"top"}
    source_config       TEXT NOT NULL DEFAULT '{}',

    -- Language hint (ISO 639-1, e.g. "en"). NULL = unknown / auto-detect.
    language            TEXT,

    -- User-customizable source colour (M0004)
    hue                 INTEGER,

    created_at          INTEGER NOT NULL,
    updated_at          INTEGER NOT NULL
);

CREATE INDEX idx_feeds_group_id ON feeds(group_id);
CREATE INDEX idx_feeds_next_fetch_at ON feeds(next_fetch_at) WHERE is_enabled = 1;
```

Feeds are upserted via `ON CONFLICT(id) DO UPDATE` (`DbCommand::UpsertFeed`), which also updates URL, type, title, group, interval, enabled, caching headers, language, hue, and source_config.

### `feed_items`

```sql
CREATE TABLE feed_items (
    id              TEXT PRIMARY KEY,            -- UUIDv5(namespace=feed_url, name=source_guid)
    feed_id         TEXT NOT NULL REFERENCES feeds(id) ON DELETE CASCADE,
    source_guid     TEXT NOT NULL,               -- original ID/GUID from the source
    title           TEXT NOT NULL,
    url             TEXT,                        -- link to original content
    author          TEXT,
    published_at    INTEGER NOT NULL,            -- Unix timestamp; always provided by normalization
    fetched_at      INTEGER NOT NULL,            -- when we fetched this item
    body_text       TEXT,                        -- cleaned plaintext (for search, rules)
    body_html       TEXT,                        -- sanitized HTML (reader view); relative URLs
                                                 -- resolved at ingestion, http media → https
    word_count      INTEGER,                     -- approximate, for reading time estimates

    -- Engagement metadata (source-specific, nullable)
    score           INTEGER,                     -- Reddit upvotes or HN points
    comment_count   INTEGER,
    comment_url     TEXT,                        -- direct link to comment thread

    -- Additional source-specific metadata (JSON blob)
    -- RSS: {"categories": [...], "enclosure_url": "..."}
    -- HN: {"type": "story"|"ask"|"show"|"job", "kids": [...]}
    -- Reddit: {"subreddit": "...", "is_self": true}
    -- Enrichment writes: enriched_at, og_title, og_description, og_image, is_image
    -- External crosspost link: external_url
    source_meta     TEXT NOT NULL DEFAULT '{}',

    UNIQUE(feed_id, source_guid)                -- prevents duplicates at DB level
);

CREATE INDEX idx_feed_items_feed_id ON feed_items(feed_id);
CREATE INDEX idx_feed_items_published_at ON feed_items(published_at DESC);
CREATE INDEX idx_feed_items_fetched_at ON feed_items(fetched_at DESC);
-- Composite index for timeline cursor pagination
CREATE INDEX idx_feed_items_timeline ON feed_items(published_at DESC, id);
```

Items are **immutable after insert** (`INSERT OR IGNORE`). The only post-insert writes are `source_meta` updates from enrichment (`DbCommand::EnrichItem`) and the relative-URL backfill (M0006 / `db fix-relative-urls`). Body-text enrichment only writes when the item currently has no `body_text`, keeping the FTS update trigger consistent.

### `item_states`

Separated from `feed_items` to keep the items table append-only. State is always user-initiated mutation.

```sql
CREATE TABLE item_states (
    item_id     TEXT PRIMARY KEY REFERENCES feed_items(id) ON DELETE CASCADE,
    is_read     INTEGER NOT NULL DEFAULT 0,
    is_saved    INTEGER NOT NULL DEFAULT 0,
    is_hidden   INTEGER NOT NULL DEFAULT 0,
    read_at     INTEGER,
    saved_at    INTEGER,
    hidden_at   INTEGER,
    note        TEXT,                            -- user annotation (M0003)
    updated_at  INTEGER NOT NULL
);

CREATE INDEX idx_item_states_is_saved ON item_states(is_saved) WHERE is_saved = 1;
CREATE INDEX idx_item_states_is_hidden ON item_states(is_hidden) WHERE is_hidden = 1;
```

A state row is inserted with defaults when a new `feed_item` is created. State writes are lazy-safe: `update_item_state` first does `INSERT ... ON CONFLICT(item_id) DO NOTHING`, so items synced before the fix still get a state row. Timestamps (`read_at`, `saved_at`, `hidden_at`) are only set when the flag transitions to 1 — unsaving preserves the original `saved_at`. The **saved view orders and month-groups by `saved_at`**, not `published_at`.

`note` supports user annotations on saved items (`set_item_note` / `NoteSheet`); a note is preserved across save toggles and cleared only when explicitly set to null.

### `ai_tags`

The rule-tag store. Every row is produced by the deterministic rule engine.

```sql
CREATE TABLE ai_tags (
    id              TEXT PRIMARY KEY,            -- UUIDv4
    item_id         TEXT NOT NULL REFERENCES feed_items(id) ON DELETE CASCADE,
    tag             TEXT NOT NULL,               -- e.g. "technical", "clickbait"
    confidence      REAL NOT NULL,               -- fixed rule confidence (0.0 to 1.0)
    tagger_source   TEXT NOT NULL,               -- always 'rule'
    rule_id         TEXT,                        -- which rule fired
    model_name      TEXT,                        -- LEGACY — always NULL
    model_version   TEXT,                        -- LEGACY — always NULL
    explanation     TEXT NOT NULL,               -- human-readable reason (matched text)
    created_at      INTEGER NOT NULL
);

CREATE INDEX idx_ai_tags_item_id ON ai_tags(item_id);
CREATE INDEX idx_ai_tags_tag ON ai_tags(tag);
CREATE UNIQUE INDEX idx_ai_tags_unique ON ai_tags(item_id, tag, tagger_source);
```

Writes use `ON CONFLICT(item_id, tag, tagger_source) DO UPDATE` (refresh confidence/explanation in place). `model_name`/`model_version` columns are vestigial from the pre-v0.6 ML stack and are always `NULL` — no model code touches them. The `tagger_source` enum in code still has a `Model` variant so a future BYO adapter can implement the `Tagger` trait without a schema change, but nothing currently emits it.

The tag filter / tag chips read the tag distribution from `get_tag_stats` (`COUNT(DISTINCT item_id)` + `COUNT(*) GROUP BY tag`).

### `user_tags`

User-assigned tags, separate from rule tags.

```sql
CREATE TABLE user_tags (
    item_id     TEXT NOT NULL REFERENCES feed_items(id) ON DELETE CASCADE,
    tag         TEXT NOT NULL,
    created_at  INTEGER NOT NULL,
    PRIMARY KEY (item_id, tag)
);

CREATE INDEX idx_user_tags_tag ON user_tags(tag);
```

Updated with full-set semantics (`replace_user_tags`: delete + reinsert, trimming/lowercasing tags and capping at 40 chars). Timeline and search queries expose both `ai_tags` and `user_tags` and match the tag filter against either.

### Legacy tables (created by M0001, unused by current code)

- **`ai_models`** — the ML model registry from the removed on-device stack (FastText/MiniLM/CLIP). No Rust code reads or writes it. Keep the table so old databases migrate cleanly, but nothing depends on it.
- **`filter_rules`** — a planned auto-hide/highlight rule engine. The schema exists but no code touches it; filtering is done by the timeline `tag`/`read`/`saved` predicates instead.

### FTS5 Virtual Table

```sql
CREATE VIRTUAL TABLE feed_items_fts USING fts5(
    id         UNINDEXED,       -- not searchable, used to join back to feed_items
    title,
    body_text,
    author,
    content    = 'feed_items',  -- external-content table
    content_rowid = 'rowid',
    tokenize   = 'unicode61 remove_diacritics 1'
);
```

Notes:

- **Column name is `id`, not `item_id`.** M0005 (`M0005_fts_content_fix.sql`) recreated the index because the original was created with a column named `item_id` that doesn't exist in the content table — external-content FTS resolves index columns by name, so any content-table access failed with `no such column: T.item_id`.
- FTS maintenance is **application-managed plus triggers**. New rows are inserted into `feed_items_fts` from the writer actor using the just-inserted `rowid` (only when `INSERT OR IGNORE` actually inserted). A `DELETE` trigger and an `AFTER UPDATE OF body_text, title, author` trigger (M0002, replaced in M0005) keep the index in sync.
- `VACUUM` renumbers implicit rowids and can silently corrupt the external-content mapping — `DbCommand::Vacuum` runs `INSERT INTO feed_items_fts(feed_items_fts) VALUES('rebuild')` afterwards.

### `schema_migrations`

Managed by `run_migrations` (`crates/pulse-core/src/storage/migrations.rs`):

```sql
CREATE TABLE schema_migrations (
    version     INTEGER PRIMARY KEY,
    applied_at  INTEGER NOT NULL
);
```

Migrations run in order at `PulseCore::init`, each in a transaction with the version insert (crash-safe). M0003/M0004 guard against duplicate-column crashes on legacy DBs. **M0006 is a programmatic data migration** (not an SQL file): it rewrites stored `body_html` so relative `src`/`href` become absolute and `http` media becomes `https` (same pass as `db fix-relative-urls`).

## ID Generation

### Feed IDs and Group IDs: UUIDv4

Random UUIDs generated at creation time.

### Feed Item IDs: UUIDv5 (Deterministic)

```
namespace = UUID_v5(NAMESPACE_URL, feed.url)
item_id   = UUID_v5(namespace, source_guid)
```

Guarantees: the same item always gets the same ID regardless of fetch time; `INSERT OR IGNORE` is sufficient for dedup (no SELECT-then-INSERT); item IDs survive database wipes and re-syncs. `UNIQUE(feed_id, source_guid)` is a secondary DB-level safety net.

## Feed Normalization

All three source types normalize to the same `FeedItem` struct.

### RSS/Atom (via `feed-rs`)

```
feed_rs::Entry.id            → source_guid (fallback: sha256 of the item URL)
feed_rs::Entry.title         → title
feed_rs::Entry.links[0].href → url
feed_rs::Entry.authors[0]    → author
feed_rs::Entry.published     → published_at (fallback: updated, then fetched_at)
feed_rs::Entry.summary       → body_text (HTML stripped)
feed_rs::Entry.content       → body_html (relative URLs resolved against the feed base)
```

### Hacker News (Firebase JSON API)

Fetch `/v0/{top|new|best|ask|show|job}stories.json` (or the HN home feed), then each item at `/v0/item/{id}.json`. Incremental sync compares against `source_config.last_seen_id` and only fetches newer IDs.

```
hn_item.id           → source_guid (stringified integer)
hn_item.title        → title
hn_item.url          → url (null for Ask HN → news.ycombinator.com/item?id={id})
hn_item.by           → author
hn_item.time         → published_at (Unix timestamp)
hn_item.text         → body_html
hn_item.score        → score
hn_item.descendants  → comment_count
hn_item.type         → source_meta.type ("story"|"ask"|"show"|"job")
```

### Reddit JSON API

Fetch `https://www.reddit.com/r/{subreddit}/{sort}.json?limit=100`. Optional OAuth2 script-app auth (`reddit_auth.rs`) raises the rate limit.

```
reddit_post.data.id           → source_guid
reddit_post.data.title        → title
reddit_post.data.url          → url (for link posts)
reddit_post.data.permalink    → comment_url
reddit_post.data.author       → author
reddit_post.data.created_utc  → published_at
reddit_post.data.selftext     → body_text
reddit_post.data.selftext_html→ body_html (relative URLs resolved)
reddit_post.data.score        → score
reddit_post.data.num_comments → comment_count
```

For self posts (`is_self=true`), `url` is set to the Reddit post URL.

### Relative-URL resolution (ingestion-time)

`resolve_relative_urls` (in `feeds/normalize.rs`) runs on every body_html at ingestion and rewrites:
- relative `src`/`href`/`srcset` → absolute against the item URL (fallback: feed URL)
- protocol-relative (`//host/…`) → `https://`
- `http:` media (img/source/embed) → `https:` — the app's CSP only allows `img-src https:`, so `http` images would be silently blocked

Items ingested before this pass are fixed by the M0006 backfill migration and the `db fix-relative-urls` CLI command.

## Migration Strategy

- Migrations are **append-only**; never edit an applied migration.
- Each runs in a transaction with its version insert; failure rolls back cleanly.
- SQL files live in `crates/pulse-core/migrations/` and are embedded via `include_str!`.
- M0006 is the first programmatic (data) migration, run from `run_migrations` directly.

## Key Query Patterns

### Timeline (cursor pagination)

```sql
-- Saved view orders by saved_at; everywhere else by published_at.
SELECT
    fi.id, fi.title, fi.url, fi.author, fi.published_at, fi.fetched_at,
    fi.word_count, fi.score, fi.comment_count, fi.comment_url, fi.body_text, fi.body_html,
    json_extract(fi.source_meta, '$.external_url') AS external_url,
    json_extract(fi.source_meta, '$.og_image') AS og_image,
    f.id AS feed_id, f.title AS feed_title, f.feed_type, f.url AS feed_url,
    f.group_id, fg.name AS group_name,
    ist.is_read, ist.is_saved, ist.is_hidden, ist.saved_at, ist.note,
    COALESCE(json_group_array(DISTINCT at.tag) FILTER (WHERE at.tag IS NOT NULL), '[]') AS ai_tags,
    COALESCE(json_group_array(DISTINCT ut.tag) FILTER (WHERE ut.tag IS NOT NULL), '[]') AS user_tags
FROM feed_items fi
JOIN feeds f ON fi.feed_id = f.id
LEFT JOIN feed_groups fg ON f.group_id = fg.id
JOIN item_states ist ON ist.item_id = fi.id
LEFT JOIN ai_tags at ON at.item_id = fi.id
LEFT JOIN user_tags ut ON ut.item_id = fi.id
WHERE ist.is_hidden = 0
  AND (COALESCE(ist.saved_at, 0) < ? OR (COALESCE(ist.saved_at, 0) = ? AND fi.id < ?))  -- saved mode
GROUP BY fi.id
ORDER BY COALESCE(ist.saved_at, 0) DESC, fi.id DESC
LIMIT ?;
```

The cursor is an opaque `(published_at | saved_at, id)` tuple returned as `nextCursor`. The tag filter matches either `ai_tags` or `user_tags` via `EXISTS`.

### Full-Text Search

```sql
SELECT fi.id, fi.title, fi.url, fi.published_at, f.title AS feed_title, rank
FROM feed_items_fts
JOIN feed_items fi ON fi.rowid = feed_items_fts.rowid
JOIN feeds f ON fi.feed_id = f.id
WHERE feed_items_fts MATCH ?
ORDER BY rank  -- or fi.published_at DESC/ASC for newest/oldest
LIMIT ?;
```

`rank` is FTS5's built-in BM25 ranking. Hidden items are excluded.

### Feed Health

```sql
SELECT
    id, title, url, feed_type,
    last_success_at, last_item_at, failure_streak,
    ROUND(CAST(total_fetches - total_failures AS REAL) / NULLIF(total_fetches, 0) * 100, 1) AS success_rate_pct,
    avg_latency_ms, next_fetch_at
FROM feeds;
```

## SQLite Configuration

Applied by `storage/connection.rs`:

```sql
-- writer pool: journal_mode=WAL, synchronous=FULL on Android / NORMAL on desktop,
-- foreign_keys=ON, busy_timeout=5000, cache_size=-8192, temp_store=MEMORY,
-- mmap_size=0 on Android / 268435456 on desktop
-- reader pool: read-only, cache_size=-16384, no journal mode (inherits WAL from writer)
```

`synchronous = NORMAL` with WAL is safe against OS crashes; Android uses `FULL` because process death is equivalent to power failure. `mmap` is disabled on Android (256MB mmap inflates VSZ and can trigger the OOM killer).
