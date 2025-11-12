-- Add system_admin to user_role enum
ALTER TYPE user_role ADD VALUE 'system_admin';

-- Create schools table
CREATE TABLE schools (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR NOT NULL,
    address TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Add school_id to users table
ALTER TABLE users ADD COLUMN school_id UUID REFERENCES schools(id) ON DELETE SET NULL;

-- Create index on school_id for faster lookups
CREATE INDEX idx_users_school_id ON users(school_id);
