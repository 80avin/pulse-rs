-- M0002_fts_update_trigger.sql
-- Replace the stub FTS update trigger with a working one.
-- Required before enrichment can update body_text and have the search index stay in sync.

DROP TRIGGER IF EXISTS feed_items_fts_update;

CREATE TRIGGER feed_items_fts_after_update
AFTER UPDATE OF body_text, title, author ON feed_items BEGIN
    -- Remove old tokens from the index
    INSERT INTO feed_items_fts(feed_items_fts, rowid, item_id, title, body_text, author)
    VALUES ('delete', old.rowid, old.id, old.title, old.body_text, old.author);
    -- Add updated tokens
    INSERT INTO feed_items_fts(rowid, item_id, title, body_text, author)
    VALUES (new.rowid, new.id, new.title, new.body_text, new.author);
END;
