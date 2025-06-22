-- Add authentication fields to canteens table
CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE OR REPLACE FUNCTION gen_username_pass_hash()
    RETURNS TRIGGER AS $$
DECLARE
    generated_username VARCHAR := (
        LOWER(REPLACE(new.canteen_name, ' ', '_'))
        );
    generated_password VARCHAR := (
        generated_username
            || '@'
            || LPAD(new.canteen_id::TEXT, 2, '0')
        );
BEGIN
    IF TG_OP = 'INSERT' THEN
        NEW.password = crypt(generated_password, gen_salt('bf', 10));
        NEW.username = generated_username;
        -- Hash the password with a randomly generated salt and a cost factor of 10
    ELSE
        IF (TG_OP = 'UPDATE' AND NEW.password IS DISTINCT FROM OLD.password) THEN
            NEW.password = crypt(NEW.password, gen_salt('bf', 10));
        END IF;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- First add the columns as nullable
ALTER TABLE canteens
    ADD COLUMN username VARCHAR,
    ADD COLUMN password VARCHAR;

CREATE TRIGGER trigger_gen_username_pass
    BEFORE INSERT OR UPDATE ON canteens
    FOR EACH ROW
EXECUTE FUNCTION gen_username_pass_hash();

UPDATE canteens
SET username = (
    LOWER(REPLACE(canteen_name, ' ', '_'))
),
password = (
    LOWER(REPLACE(canteen_name, ' ', '_'))
        || '@'
        || LPAD(canteen_id::TEXT, 2, '0')
    );

-- Then make them NOT NULL
ALTER TABLE canteens
    ALTER COLUMN username SET NOT NULL,
    ALTER COLUMN password SET NOT NULL;

-- Finally add the unique constraint
ALTER TABLE canteens
    ADD CONSTRAINT canteens_username_key UNIQUE (username);
