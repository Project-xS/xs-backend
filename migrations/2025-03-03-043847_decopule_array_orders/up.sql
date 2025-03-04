-- Your SQL goes here
ALTER TABLE active_orders
    DROP CONSTRAINT active_orders_items_check;

ALTER TABLE active_orders
    DROP COLUMN items;

CREATE TABLE active_order_items
(
    order_id INTEGER
        CONSTRAINT active_order_items_active_orders_order_id_fk
            REFERENCES active_orders ON DELETE CASCADE,
    item_id  INTEGER
        CONSTRAINT active_order_items_menu_items_item_id_fk
            REFERENCES menu_items,
    quantity SMALLINT NOT NULL DEFAULT 1,
    CONSTRAINT active_order_items_pk
        PRIMARY KEY (order_id, item_id)
);

CREATE INDEX ON active_order_items USING btree (item_id);
