-- M0001_initial.sql
-- Initial schema for Pulse

CREATE TABLE feed_groups (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL UNIQUE,
    description TEXT,
    color       TEXT,
    sort_order  INTEGER NOT NULL DEFAULT 0,
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL
);

CREATE TABLE feeds (
    id                  TEXT PRIMARY KEY,
    url                 TEXT NOT NULL UNIQUE,
    feed_type           TEXT NOT NULL,
    title               TEXT,
    description         TEXT,
    site_url            TEXT,
    icon_url            TEXT,
    group_id            TEXT REFERENCES feed_groups(id) ON DELETE SET NULL,
    poll_interval_secs  INTEGER NOT NULL DEFAULT 3600,
    is_enabled          INTEGER NOT NULL DEFAULT 1,
    etag                TEXT,
    last_modified       TEXT,
    last_fetched_at     INTEGER,
    last_success_at     INTEGER,
    last_item_at        INTEGER,
    failure_streak      INTEGER NOT NULL DEFAULT 0,
    total_fetches       INTEGER NOT NULL DEFAULT 0,
    total_failures      INTEGER NOT NULL DEFAULT 0,
    avg_latency_ms      REAL,
    next_fetch_at       INTEGER,
    source_config       TEXT NOT NULL DEFAULT '{}',
    language            TEXT,
    created_at          INTEGER NOT NULL,
    updated_at          INTEGER NOT NULL
);

CREATE INDEX idx_feeds_group_id ON feeds(group_id);
CREATE INDEX idx_feeds_next_fetch_at ON feeds(next_fetch_at) WHERE is_enabled = 1;

CREATE TABLE feed_items (
    id              TEXT PRIMARY KEY,
    feed_id         TEXT NOT NULL REFERENCES feeds(id) ON DELETE CASCADE,
    source_guid     TEXT NOT NULL,
    title           TEXT NOT NULL,
    url             TEXT,
    author          TEXT,
    published_at    INTEGER NOT NULL,
    fetched_at      INTEGER NOT NULL,
    body_text       TEXT,
    body_html       TEXT,
    word_count      INTEGER,
    score           INTEGER,
    comment_count   INTEGER,
    comment_url     TEXT,
    source_meta     TEXT NOT NULL DEFAULT '{}',
    UNIQUE(feed_id, source_guid)
);

CREATE INDEX idx_feed_items_feed_id ON feed_items(feed_id);
CREATE INDEX idx_feed_items_published_at ON feed_items(published_at DESC);
CREATE INDEX idx_feed_items_fetched_at ON feed_items(fetched_at DESC);
CREATE INDEX idx_feed_items_timeline ON feed_items(published_at DESC, id);

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

CREATE TABLE ai_tags (
    id              TEXT PRIMARY KEY,
    item_id         TEXT NOT NULL REFERENCES feed_items(id) ON DELETE CASCADE,
    tag             TEXT NOT NULL,
    confidence      REAL NOT NULL,
    tagger_source   TEXT NOT NULL,
    rule_id         TEXT,
    model_name      TEXT,
    model_version   TEXT,
    explanation     TEXT NOT NULL,
    created_at      INTEGER NOT NULL
);

CREATE INDEX idx_ai_tags_item_id ON ai_tags(item_id);
CREATE INDEX idx_ai_tags_tag ON ai_tags(tag);
CREATE UNIQUE INDEX idx_ai_tags_unique ON ai_tags(item_id, tag, tagger_source);

CREATE TABLE ai_models (
    id                  TEXT PRIMARY KEY,
    display_name        TEXT NOT NULL,
    model_url           TEXT NOT NULL,
    tokenizer_url       TEXT,
    file_path           TEXT,
    file_size_bytes     INTEGER,
    sha256              TEXT,
    is_downloaded       INTEGER NOT NULL DEFAULT 0,
    is_active           INTEGER NOT NULL DEFAULT 0,
    download_progress   REAL,
    created_at          INTEGER NOT NULL,
    downloaded_at       INTEGER
);

CREATE TABLE user_tags (
    item_id     TEXT NOT NULL REFERENCES feed_items(id) ON DELETE CASCADE,
    tag         TEXT NOT NULL,
    created_at  INTEGER NOT NULL,
    PRIMARY KEY (item_id, tag)
);

CREATE INDEX idx_user_tags_tag ON user_tags(tag);

CREATE TABLE filter_rules (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    scope_type  TEXT NOT NULL DEFAULT 'global',
    scope_id    TEXT,
    action      TEXT NOT NULL,
    conditions  TEXT NOT NULL,
    is_enabled  INTEGER NOT NULL DEFAULT 1,
    sort_order  INTEGER NOT NULL DEFAULT 0,
    created_at  INTEGER NOT NULL
);

CREATE VIRTUAL TABLE feed_items_fts USING fts5(
    item_id    UNINDEXED,
    title,
    body_text,
    author,
    content    = 'feed_items',
    content_rowid = 'rowid',
    tokenize   = 'unicode61 remove_diacritics 1'
);

CREATE TRIGGER feed_items_fts_delete AFTER DELETE ON feed_items BEGIN
    INSERT INTO feed_items_fts(feed_items_fts, rowid, item_id, title, body_text, author)
    VALUES ('delete', old.rowid, old.id, old.title, old.body_text, old.author);
END;

CREATE TRIGGER feed_items_fts_update AFTER UPDATE OF body_text, title, author ON feed_items BEGIN
    SELECT RAISE(FAIL, 'feed_items FTS UPDATE trigger not implemented; run fts5 rebuild after implementing');
END;
