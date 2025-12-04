-- Create branches table
CREATE TABLE branches (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(100) NOT NULL,
    description TEXT,
    level_id UUID NOT NULL REFERENCES levels(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT unique_branch_name_per_level UNIQUE (name, level_id)
);

-- Add branch_id to users table for students
ALTER TABLE users ADD COLUMN branch_id UUID REFERENCES branches(id) ON DELETE SET NULL;

-- Create indexes
CREATE INDEX idx_branches_level_id ON branches(level_id);
CREATE INDEX idx_users_branch_id ON users(branch_id);
CREATE INDEX idx_branches_name ON branches(name);
