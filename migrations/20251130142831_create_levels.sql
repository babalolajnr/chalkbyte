-- Create levels table for school levels/grades
CREATE TABLE levels (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(100) NOT NULL,
    description TEXT,
    school_id UUID NOT NULL REFERENCES schools(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT unique_level_name_per_school UNIQUE (name, school_id)
);

-- Add level_id to users table for students
ALTER TABLE users ADD COLUMN level_id UUID REFERENCES levels(id) ON DELETE SET NULL;

-- Create indexes
CREATE INDEX idx_levels_school_id ON levels(school_id);
CREATE INDEX idx_users_level_id ON users(level_id);
CREATE INDEX idx_levels_name ON levels(name);
