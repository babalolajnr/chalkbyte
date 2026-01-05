-- Add slug column to roles table
ALTER TABLE roles ADD COLUMN slug VARCHAR(50);

-- Populate slug for existing system roles
UPDATE roles SET slug = 'system_admin' WHERE id = '00000000-0000-0000-0000-000000000001';
UPDATE roles SET slug = 'admin' WHERE id = '00000000-0000-0000-0000-000000000002';
UPDATE roles SET slug = 'teacher' WHERE id = '00000000-0000-0000-0000-000000000003';
UPDATE roles SET slug = 'student' WHERE id = '00000000-0000-0000-0000-000000000004';

-- Make slug NOT NULL after populating existing data
-- For any custom roles without slug, generate from name
UPDATE roles SET slug = LOWER(REPLACE(REPLACE(name, ' ', '_'), '-', '_')) WHERE slug IS NULL;

ALTER TABLE roles ALTER COLUMN slug SET NOT NULL;

-- Add unique constraint for slug within scope (school_id)
-- System roles (school_id IS NULL) must have unique slugs globally
-- School roles must have unique slugs within their school
ALTER TABLE roles ADD CONSTRAINT unique_role_slug_per_scope UNIQUE (slug, school_id);

-- Create index for faster lookups by slug
CREATE INDEX idx_roles_slug ON roles(slug);
