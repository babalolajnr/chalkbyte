-- Add unique constraint to school name
ALTER TABLE schools ADD CONSTRAINT schools_name_unique UNIQUE (name);
