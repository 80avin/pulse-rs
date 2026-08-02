-- M0005_fts_content_fix.sql
-- The FTS5 external-content table was created with a column named `item_id`,
-- which does not exist in the content table `feed_items` (it has `id`).
-- External-content FTS resolves index columns by NAME against the content table,
-- so any content-table access (DELETE-all, `rebuild`, implicit reads) failed with
-- "no such column: T.item_id". Recreate the index with a matching column name.

DROP TRIGGER IF EXISTS feed_items_fts_after_update;
DROP TRIGGER IF EXISTS feed_items_fts_delete;
DROP TABLE IF EXISTS feed_items_fts;

CREATE VIRTUAL TABLE feed_items_fts USING fts5(
    id         UNINDEXED,
    title,
    body_text,
    author,
    content    = 'feed_items',
    content_rowid = 'rowid',
    tokenize   = 'unicode61 remove_diacritics 1'
);

CREATE TRIGGER feed_items_fts_delete AFTER DELETE ON feed_items BEGIN
    INSERT INTO feed_items_fts(feed_items_fts, rowid, id, title, body_text, author)
    VALUES ('delete', old.rowid, old.id, old.title, old.body_text, old.author);
END;

CREATE TRIGGER feed_items_fts_after_update
AFTER UPDATE OF body_text, title, author ON feed_items BEGIN
    INSERT INTO feed_items_fts(feed_items_fts, rowid, id, title, body_text, author)
    VALUES ('delete', old.rowid, old.id, old.title, old.body_text, old.author);
    INSERT INTO feed_items_fts(rowid, id, title, body_text, author)
    VALUES (new.rowid, new.id, new.title, new.body_text, new.author);
END;

INSERT INTO feed_items_fts(rowid, id, title, body_text, author)
SELECT rowid, id, title, body_text, author FROM feed_items;
