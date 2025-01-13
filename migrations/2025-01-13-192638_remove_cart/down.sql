-- This file should undo anything in `up.sql`
-- Create cart table
CREATE TABLE cart
(
    user_id INTEGER PRIMARY KEY REFERENCES users (user_id),
    items   INTEGER[] NOT NULL CHECK (array_length(items, 1) > 0)
);
