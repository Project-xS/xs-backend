-- This file should undo anything in `up.sql`
ALTER TABLE canteens
    DROP COLUMN pic_link;

ALTER TABLE canteens
    ADD COLUMN pic_link VARCHAR;

ALTER TABLE menu_items
    DROP COLUMN pic_link;

ALTER TABLE menu_items
    ADD COLUMN pic_link VARCHAR;
