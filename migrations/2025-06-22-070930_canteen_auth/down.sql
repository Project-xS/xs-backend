-- This file should undo anything in `up.sql`
ALTER TABLE canteens
    DROP COLUMN IF EXISTS username;
ALTER TABLE canteens
    DROP COLUMN IF EXISTS password;

DROP TRIGGER IF EXISTS trigger_gen_username_pass ON canteens;

DROP FUNCTION IF EXISTS gen_username_pass_hash;

DROP EXTENSION IF EXISTS pgcrypto;
