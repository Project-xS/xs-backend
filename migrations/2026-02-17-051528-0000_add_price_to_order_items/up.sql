ALTER TABLE active_order_items
    ADD COLUMN price INTEGER NOT NULL DEFAULT 0;

UPDATE active_order_items a
SET price = m.price
FROM menu_items m
WHERE a.item_id = m.item_id;

ALTER TABLE active_order_items
    ALTER COLUMN price DROP DEFAULT;

ALTER TABLE held_order_items
    ADD COLUMN price INTEGER NOT NULL DEFAULT 0;

UPDATE held_order_items h
SET price = m.price
FROM menu_items m
WHERE h.item_id = m.item_id;

ALTER TABLE held_order_items
    ALTER COLUMN price DROP DEFAULT;
