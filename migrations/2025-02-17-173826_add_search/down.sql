-- This file should undo anything in `up.sql`
DROP INDEX IF EXISTS menu_items_trgm_idx;

DROP EXTENSION IF EXISTS pg_trgm;
