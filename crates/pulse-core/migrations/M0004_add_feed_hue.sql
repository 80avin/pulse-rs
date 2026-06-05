-- M0004_add_feed_hue.sql
-- Add hue column for user-customizable source colour
ALTER TABLE feeds ADD COLUMN hue INTEGER;
