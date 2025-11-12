-- Create user_role enum type
CREATE TYPE user_role AS ENUM ('admin', 'teacher', 'student');

-- Add role column to users table with default value 'student'
ALTER TABLE users ADD COLUMN role user_role NOT NULL DEFAULT 'student';

-- Create index on role for faster lookups
CREATE INDEX idx_users_role ON users(role);
