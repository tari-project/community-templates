-- Invalidate JWTs upon password change or account revocation (#27)
ALTER TABLE admins ADD COLUMN token_version INTEGER NOT NULL DEFAULT 1;
