-- This file should undo anything in `up.sql`
alter table public.menu_items
    drop pic_etag;
