-- This file should undo anything in `up.sql`
ALTER TABLE menu_items
    ADD list BOOLEAN DEFAULT TRUE NOT NULL;
