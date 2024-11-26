-- Your SQL goes here

-- Create users table
CREATE TABLE users
(
    user_id    SERIAL PRIMARY KEY,
    rfid       VARCHAR UNIQUE NOT NULL,
    name       VARCHAR NOT NULL,
    email      VARCHAR UNIQUE NOT NULL,
    created_at TIMESTAMP(0) WITH TIME ZONE NOT NULL DEFAULT (NOW() AT TIME ZONE 'UTC')
);

-- Create canteens table
CREATE TABLE canteens
(
    canteen_id   SERIAL PRIMARY KEY,
    canteen_name VARCHAR NOT NULL,
    location     VARCHAR NOT NULL
);
CREATE INDEX idx_canteens_name ON canteens (canteen_name);

-- Create menu_items table
CREATE TABLE menu_items
(
    item_id      SERIAL PRIMARY KEY,
    canteen_id   INTEGER NOT NULL REFERENCES canteens (canteen_id),
    name         VARCHAR NOT NULL,
    is_veg       BOOLEAN NOT NULL DEFAULT false,
    price        FLOAT   NOT NULL DEFAULT 0.0,
    stock        INTEGER NOT NULL DEFAULT 0,
    is_available BOOLEAN NOT NULL DEFAULT false,
    list         BOOLEAN NOT NULL DEFAULT true,
    pic_link     VARCHAR,
    description  VARCHAR
);
CREATE INDEX idx_menu_items_canteen_id ON menu_items (canteen_id);
CREATE INDEX idx_menu_items_name ON menu_items (name);
CREATE INDEX idx_menu_items_is_available ON menu_items (is_available);

-- Create active_orders table
CREATE TABLE active_orders
(
    order_id   VARCHAR PRIMARY KEY,
    user_id    INTEGER NOT NULL REFERENCES users (user_id),
    items      INTEGER[] NOT NULL CHECK (array_length(items, 1) > 0),
    ordered_at TIMESTAMP(0) WITH TIME ZONE NOT NULL DEFAULT (NOW() AT TIME ZONE 'UTC')
);
CREATE INDEX idx_active_orders_user_id ON active_orders (user_id);
CREATE INDEX idx_active_orders_ordered_at ON active_orders (ordered_at);

-- Create past_orders table
CREATE TABLE past_orders
(
    order_id     VARCHAR PRIMARY KEY,
    user_id      INTEGER NOT NULL REFERENCES users (user_id),
    items        INTEGER[] NOT NULL CHECK (array_length(items, 1) > 0),
    order_status BOOLEAN NOT NULL DEFAULT true,
    ordered_at   TIMESTAMP(0) WITH TIME ZONE NOT NULL
);
CREATE INDEX idx_past_orders_user_id ON past_orders (user_id);
CREATE INDEX idx_past_orders_ordered_at ON past_orders (ordered_at);

-- Create item_count table
CREATE TABLE item_count
(
    item_id     INTEGER PRIMARY KEY REFERENCES menu_items (item_id),
    num_ordered INTEGER NOT NULL DEFAULT 0
);

-- Create cart table
CREATE TABLE cart
(
    user_id INTEGER PRIMARY KEY REFERENCES users (user_id),
    items   INTEGER[] NOT NULL CHECK (array_length(items, 1) > 0)
);
