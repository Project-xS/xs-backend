-- This file should undo anything in `up.sql`
ALTER TABLE public.menu_items
    DROP pic_etag;

ALTER TABLE public.canteens
    DROP pic_etag;
