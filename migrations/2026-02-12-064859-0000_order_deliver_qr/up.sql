-- Create held_orders table for order reservation during payment
CREATE TABLE held_orders (
    hold_id     SERIAL PRIMARY KEY,
    user_id     INTEGER NOT NULL REFERENCES users(user_id),
    canteen_id  INTEGER NOT NULL REFERENCES canteens(canteen_id),
    total_price INTEGER NOT NULL,
    deliver_at  time_band,
    held_at     TIMESTAMP(0) WITH TIME ZONE NOT NULL DEFAULT (NOW() AT TIME ZONE 'UTC'),
    expires_at  TIMESTAMP(0) WITH TIME ZONE NOT NULL
);
CREATE INDEX idx_held_orders_user_id ON held_orders(user_id);
CREATE INDEX idx_held_orders_expires_at ON held_orders(expires_at);

-- Create held_order_items table (mirrors active_order_items)
CREATE TABLE held_order_items (
    hold_id  INTEGER NOT NULL REFERENCES held_orders(hold_id) ON DELETE CASCADE,
    item_id  INTEGER NOT NULL REFERENCES menu_items(item_id),
    quantity SMALLINT NOT NULL,
    PRIMARY KEY (hold_id, item_id)
);
