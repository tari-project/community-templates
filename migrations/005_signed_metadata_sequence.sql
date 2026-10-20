-- Add signed metadata replay protection sequence counter (#25)
ALTER TABLE template_metadata ADD COLUMN last_signed_sequence INTEGER NOT NULL DEFAULT 0;
