-- Add commit_hash and supersedes fields from metadata v0.5.0
ALTER TABLE template_metadata ADD COLUMN commit_hash TEXT;
ALTER TABLE template_metadata ADD COLUMN supersedes TEXT;
