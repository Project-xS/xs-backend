ALTER TABLE active_order_items
    DROP COLUMN IF EXISTS price;

ALTER TABLE held_order_items
    DROP COLUMN IF EXISTS price;
