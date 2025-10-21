-- Revert Firebase auth changes

-- 1) Drop unique index and new columns
DROP INDEX IF EXISTS idx_users_firebase_uid;

ALTER TABLE users
    DROP COLUMN IF EXISTS firebase_uid,
    DROP COLUMN IF EXISTS auth_provider,
    DROP COLUMN IF EXISTS email_verified,
    DROP COLUMN IF EXISTS display_name,
    DROP COLUMN IF EXISTS photo_url;

-- 2) Make rfid NOT NULL again
ALTER TABLE users
    ALTER COLUMN rfid SET NOT NULL;
