ALTER TABLE canteens
    ADD COLUMN opening_time TIME,
    ADD COLUMN closing_time TIME,
    ADD COLUMN is_open BOOLEAN NOT NULL DEFAULT TRUE,
    ADD COLUMN last_opened_at TIMESTAMPTZ;
