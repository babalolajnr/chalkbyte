-- Add school logo support
-- Allows schools to upload and store logos

ALTER TABLE schools ADD COLUMN logo_path TEXT UNIQUE;

-- Create index for faster lookups
CREATE INDEX idx_schools_logo_path ON schools(logo_path) WHERE logo_path IS NOT NULL;
