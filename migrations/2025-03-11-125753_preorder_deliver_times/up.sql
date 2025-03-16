-- Your SQL goes here

CREATE TYPE time_band AS ENUM ('ElevenAM', 'TwelvePM');

ALTER TABLE public.active_orders
    ADD IF NOT EXISTS canteen_id INTEGER NOT NULL
        CONSTRAINT active_orders_canteens_canteen_id_fk
            REFERENCES public.canteens
            ON UPDATE RESTRICT ON DELETE RESTRICT;

ALTER TABLE public.active_orders
    ADD deliver_at time_band;

ALTER TABLE active_orders
    RENAME COLUMN price TO total_price;

ALTER TABLE active_orders
    ADD CONSTRAINT active_orders_pk
        UNIQUE (order_id, user_id, canteen_id);

CREATE INDEX active_orders_canteen_id_index
    ON public.active_orders (canteen_id);

CREATE INDEX active_orders_deliver_at_index
    ON public.active_orders (deliver_at);
