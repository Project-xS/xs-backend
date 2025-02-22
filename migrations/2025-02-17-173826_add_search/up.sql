-- Your SQL goes here
CREATE EXTENSION IF NOT EXISTS pg_trgm;

CREATE INDEX IF NOT EXISTS menu_items_trgm_idx
ON menu_items
USING GIN(name gin_trgm_ops);
