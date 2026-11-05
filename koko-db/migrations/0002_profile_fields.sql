-- Add public profile metadata without changing existing accounts.
ALTER TABLE users ADD COLUMN bio TEXT NOT NULL DEFAULT '';
ALTER TABLE users ADD COLUMN avatar_media_id TEXT;
