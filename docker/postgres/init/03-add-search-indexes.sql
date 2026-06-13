CREATE EXTENSION IF NOT EXISTS pg_trgm;
CREATE INDEX IF NOT EXISTS idx_urls_original_url_trgm ON urls USING GIN (original_url gin_trgm_ops);
