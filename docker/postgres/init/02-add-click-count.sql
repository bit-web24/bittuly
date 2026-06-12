-- Live migration: add click_count to urls table
-- Run this against the existing database if you're not doing a fresh init.
ALTER TABLE urls ADD COLUMN IF NOT EXISTS click_count BIGINT NOT NULL DEFAULT 0;
