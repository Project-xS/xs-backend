-- This file should undo anything in `up.sql`
DROP EXTENSION IF EXISTS pg_trgm;

DROP INDEX IF EXISTS menu_items_trgm_idx;
