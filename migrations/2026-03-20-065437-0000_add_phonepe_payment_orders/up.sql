CREATE TABLE payment_orders (
    payment_id SERIAL PRIMARY KEY,
    hold_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL REFERENCES users(user_id),
    merchant_order_id VARCHAR NOT NULL,
    phonepe_order_id VARCHAR NOT NULL,
    sdk_token TEXT NOT NULL,
    amount INTEGER NOT NULL,
    payment_state VARCHAR NOT NULL CHECK (
        payment_state IN ('CREATED', 'PENDING', 'COMPLETED', 'FAILED')
    ),
    phonepe_expires_at TIMESTAMP(0) WITH TIME ZONE,
    app_order_id INTEGER,
    created_at TIMESTAMP(0) WITH TIME ZONE NOT NULL DEFAULT (NOW() AT TIME ZONE 'UTC'),
    updated_at TIMESTAMP(0) WITH TIME ZONE NOT NULL DEFAULT (NOW() AT TIME ZONE 'UTC')
);

CREATE UNIQUE INDEX idx_payment_orders_hold_id ON payment_orders(hold_id);
CREATE UNIQUE INDEX idx_payment_orders_merchant_order_id ON payment_orders(merchant_order_id);
CREATE INDEX idx_payment_orders_user_hold ON payment_orders(user_id, hold_id);
CREATE INDEX idx_payment_orders_state ON payment_orders(payment_state);
