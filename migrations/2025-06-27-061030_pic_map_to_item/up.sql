-- Your SQL goes here
ALTER TABLE menu_items
    DROP COLUMN pic_link;

ALTER TABLE menu_items
    ADD COLUMN pic_link bool DEFAULT false NOT NULL;

ALTER TABLE canteens
    DROP COLUMN pic_link;

ALTER TABLE canteens
    ADD COLUMN pic_link bool DEFAULT false NOT NULL;
