-- Rename boolean column `pic_link` to `has_pic` on both tables
ALTER TABLE public.menu_items
    RENAME COLUMN pic_link TO has_pic;

ALTER TABLE public.canteens
    RENAME COLUMN pic_link TO has_pic;
