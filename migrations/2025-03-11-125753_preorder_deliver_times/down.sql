-- This file should undo anything in `up.sql`

ALTER TABLE active_orders
    DROP CONSTRAINT active_orders_pk;

ALTER TABLE public.active_orders
    DROP COLUMN canteen_id;

ALTER TABLE public.active_orders
    DROP COLUMN deliver_at;

ALTER TABLE active_orders
    RENAME COLUMN total_price TO price;

DROP TYPE time_band;
