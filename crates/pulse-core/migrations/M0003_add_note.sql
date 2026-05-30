-- M0003_add_note.sql
-- Add note column to item_states for user annotations on saved items

ALTER TABLE item_states ADD COLUMN note TEXT;
