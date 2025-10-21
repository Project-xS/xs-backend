-- Alter users table for Firebase auth
-- 1) Make rfid nullable
ALTER TABLE users
    ALTER COLUMN rfid DROP NOT NULL;

-- 2) Add firebase auth related columns
ALTER TABLE users
    ADD COLUMN IF NOT EXISTS firebase_uid TEXT,
    ADD COLUMN IF NOT EXISTS auth_provider TEXT NOT NULL DEFAULT 'google',
    ADD COLUMN IF NOT EXISTS email_verified BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS display_name TEXT,
    ADD COLUMN IF NOT EXISTS photo_url TEXT;

-- 3) Populate firebase_uid for existing rows if any
UPDATE users
SET firebase_uid = 'legacy:' || user_id
WHERE firebase_uid IS NULL;

-- 4) Enforce constraints
ALTER TABLE users
    ALTER COLUMN firebase_uid SET NOT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS idx_users_firebase_uid ON users(firebase_uid);
