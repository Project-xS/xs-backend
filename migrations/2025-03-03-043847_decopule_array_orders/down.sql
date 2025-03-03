-- This file should undo anything in `up.sql`

ALTER TABLE active_orders
    ADD items INTEGER[] NOT NULL DEFAULT '{}'
        CONSTRAINT active_orders_items_check
            CHECK (ARRAY_LENGTH(items, 1) > 0);

DROP TABLE IF EXISTS active_order_items;
