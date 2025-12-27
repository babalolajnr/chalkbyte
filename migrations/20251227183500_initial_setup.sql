-- Chalkbyte API Initial Database Setup
-- This migration creates all tables, indexes, and seeds initial data

-- ============================================
-- Extensions
-- ============================================
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- ============================================
-- Schools Table
-- ============================================
CREATE TABLE schools (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(255) NOT NULL UNIQUE,
    address TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_schools_name ON schools(name);

-- ============================================
-- Users Table
-- ============================================
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    first_name VARCHAR(100) NOT NULL,
    last_name VARCHAR(100) NOT NULL,
    email VARCHAR(255) NOT NULL UNIQUE,
    password VARCHAR(255),
    school_id UUID REFERENCES schools(id) ON DELETE SET NULL,
    date_of_birth DATE,
    grade_level VARCHAR(10),
    mfa_enabled BOOLEAN NOT NULL DEFAULT FALSE,
    mfa_secret TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_school_id ON users(school_id);
CREATE INDEX idx_users_created_at ON users(created_at);
CREATE INDEX idx_users_date_of_birth ON users(date_of_birth);
CREATE INDEX idx_users_grade_level ON users(grade_level);

-- ============================================
-- Levels Table (School Grades/Classes)
-- ============================================
CREATE TABLE levels (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(100) NOT NULL,
    description TEXT,
    school_id UUID NOT NULL REFERENCES schools(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT unique_level_name_per_school UNIQUE (name, school_id)
);

CREATE INDEX idx_levels_school_id ON levels(school_id);
CREATE INDEX idx_levels_name ON levels(name);

-- Add level_id to users
ALTER TABLE users ADD COLUMN level_id UUID REFERENCES levels(id) ON DELETE SET NULL;
CREATE INDEX idx_users_level_id ON users(level_id);

-- ============================================
-- Branches Table (Class Sections)
-- ============================================
CREATE TABLE branches (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(100) NOT NULL,
    description TEXT,
    level_id UUID NOT NULL REFERENCES levels(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT unique_branch_name_per_level UNIQUE (name, level_id)
);

CREATE INDEX idx_branches_level_id ON branches(level_id);
CREATE INDEX idx_branches_name ON branches(name);

-- Add branch_id to users
ALTER TABLE users ADD COLUMN branch_id UUID REFERENCES branches(id) ON DELETE SET NULL;
CREATE INDEX idx_users_branch_id ON users(branch_id);

-- ============================================
-- Permissions Table
-- ============================================
CREATE TABLE permissions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(100) NOT NULL UNIQUE,
    description TEXT,
    category VARCHAR(50) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_permissions_category ON permissions(category);
CREATE INDEX idx_permissions_name ON permissions(name);

-- ============================================
-- Roles Table
-- ============================================
CREATE TABLE roles (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(100) NOT NULL,
    description TEXT,
    school_id UUID REFERENCES schools(id) ON DELETE CASCADE,
    is_system_role BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT unique_role_name_per_scope UNIQUE (name, school_id),
    CONSTRAINT system_role_no_school CHECK (
        (is_system_role = TRUE AND school_id IS NULL) OR
        (is_system_role = FALSE)
    )
);

CREATE INDEX idx_roles_school_id ON roles(school_id);
CREATE INDEX idx_roles_is_system_role ON roles(is_system_role);
CREATE INDEX idx_roles_name ON roles(name);

-- ============================================
-- Role Permissions Junction Table
-- ============================================
CREATE TABLE role_permissions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    role_id UUID NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    permission_id UUID NOT NULL REFERENCES permissions(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT unique_role_permission UNIQUE (role_id, permission_id)
);

CREATE INDEX idx_role_permissions_role_id ON role_permissions(role_id);
CREATE INDEX idx_role_permissions_permission_id ON role_permissions(permission_id);

-- ============================================
-- User Roles Assignment Table
-- ============================================
CREATE TABLE user_roles (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role_id UUID NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    assigned_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    assigned_by UUID REFERENCES users(id) ON DELETE SET NULL,
    CONSTRAINT unique_user_role UNIQUE (user_id, role_id)
);

CREATE INDEX idx_user_roles_user_id ON user_roles(user_id);
CREATE INDEX idx_user_roles_role_id ON user_roles(role_id);

-- ============================================
-- Password Reset Tokens Table
-- ============================================
CREATE TABLE password_reset_tokens (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token TEXT NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    used BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_password_reset_tokens_user_id ON password_reset_tokens(user_id);
CREATE INDEX idx_password_reset_tokens_token ON password_reset_tokens(token);
CREATE INDEX idx_password_reset_tokens_expires_at ON password_reset_tokens(expires_at);

-- ============================================
-- MFA Recovery Codes Table
-- ============================================
CREATE TABLE mfa_recovery_codes (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    code_hash TEXT NOT NULL,
    used BOOLEAN NOT NULL DEFAULT FALSE,
    used_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_mfa_recovery_codes_user_id ON mfa_recovery_codes(user_id);
CREATE INDEX idx_mfa_recovery_codes_used ON mfa_recovery_codes(used);

-- ============================================
-- Refresh Tokens Table
-- ============================================
CREATE TABLE refresh_tokens (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token TEXT NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    revoked BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_refresh_tokens_user_id ON refresh_tokens(user_id);
CREATE INDEX idx_refresh_tokens_token ON refresh_tokens(token);
CREATE INDEX idx_refresh_tokens_expires_at ON refresh_tokens(expires_at);
CREATE INDEX idx_refresh_tokens_revoked ON refresh_tokens(revoked);

-- ============================================
-- Triggers for updated_at
-- ============================================

-- Roles updated_at trigger
CREATE OR REPLACE FUNCTION update_roles_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_update_roles_updated_at
    BEFORE UPDATE ON roles
    FOR EACH ROW
    EXECUTE FUNCTION update_roles_updated_at();

-- Permissions updated_at trigger
CREATE OR REPLACE FUNCTION update_permissions_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_update_permissions_updated_at
    BEFORE UPDATE ON permissions
    FOR EACH ROW
    EXECUTE FUNCTION update_permissions_updated_at();

-- ============================================
-- Seed Default Permissions
-- ============================================
INSERT INTO permissions (name, description, category) VALUES
    -- User permissions
    ('users:create', 'Create new users', 'users'),
    ('users:read', 'View user information', 'users'),
    ('users:update', 'Update user information', 'users'),
    ('users:delete', 'Delete users', 'users'),

    -- School permissions
    ('schools:create', 'Create new schools', 'schools'),
    ('schools:read', 'View school information', 'schools'),
    ('schools:update', 'Update school information', 'schools'),
    ('schools:delete', 'Delete schools', 'schools'),

    -- Student permissions
    ('students:create', 'Create new students', 'students'),
    ('students:read', 'View student information', 'students'),
    ('students:update', 'Update student information', 'students'),
    ('students:delete', 'Delete students', 'students'),

    -- Level permissions
    ('levels:create', 'Create new levels', 'levels'),
    ('levels:read', 'View level information', 'levels'),
    ('levels:update', 'Update level information', 'levels'),
    ('levels:delete', 'Delete levels', 'levels'),
    ('levels:assign_students', 'Assign students to levels', 'levels'),

    -- Branch permissions
    ('branches:create', 'Create new branches', 'branches'),
    ('branches:read', 'View branch information', 'branches'),
    ('branches:update', 'Update branch information', 'branches'),
    ('branches:delete', 'Delete branches', 'branches'),
    ('branches:assign_students', 'Assign students to branches', 'branches'),

    -- Role management permissions
    ('roles:create', 'Create custom roles', 'roles'),
    ('roles:read', 'View roles', 'roles'),
    ('roles:update', 'Update roles', 'roles'),
    ('roles:delete', 'Delete roles', 'roles'),
    ('roles:assign', 'Assign roles to users', 'roles'),

    -- Reports permissions
    ('reports:view', 'View reports and analytics', 'reports'),
    ('reports:export', 'Export reports', 'reports'),

    -- Settings permissions
    ('settings:read', 'View settings', 'settings'),
    ('settings:update', 'Update settings', 'settings');

-- ============================================
-- Seed Default System Roles
-- ============================================
INSERT INTO roles (id, name, description, school_id, is_system_role) VALUES
    ('00000000-0000-0000-0000-000000000001', 'System Admin', 'Full system access with all permissions', NULL, TRUE),
    ('00000000-0000-0000-0000-000000000002', 'Admin', 'School administrator with school-scoped management permissions', NULL, TRUE),
    ('00000000-0000-0000-0000-000000000003', 'Teacher', 'School staff with teaching-related permissions', NULL, TRUE),
    ('00000000-0000-0000-0000-000000000004', 'Student', 'Default role with basic read permissions', NULL, TRUE);

-- ============================================
-- Assign Permissions to System Roles
-- ============================================

-- System Admin gets ALL permissions
INSERT INTO role_permissions (role_id, permission_id)
SELECT '00000000-0000-0000-0000-000000000001', id FROM permissions;

-- Admin role permissions
INSERT INTO role_permissions (role_id, permission_id)
SELECT '00000000-0000-0000-0000-000000000002', id FROM permissions
WHERE name IN (
    'users:create', 'users:read', 'users:update', 'users:delete',
    'schools:read', 'schools:update',
    'students:create', 'students:read', 'students:update', 'students:delete',
    'levels:create', 'levels:read', 'levels:update', 'levels:delete', 'levels:assign_students',
    'branches:create', 'branches:read', 'branches:update', 'branches:delete', 'branches:assign_students',
    'roles:create', 'roles:read', 'roles:update', 'roles:delete', 'roles:assign',
    'reports:view', 'reports:export',
    'settings:read', 'settings:update'
);

-- Teacher role permissions
INSERT INTO role_permissions (role_id, permission_id)
SELECT '00000000-0000-0000-0000-000000000003', id FROM permissions
WHERE name IN (
    'users:read',
    'schools:read',
    'students:read', 'students:update',
    'levels:read',
    'branches:read',
    'roles:read',
    'reports:view'
);

-- Student role permissions
INSERT INTO role_permissions (role_id, permission_id)
SELECT '00000000-0000-0000-0000-000000000004', id FROM permissions
WHERE name IN (
    'schools:read',
    'levels:read',
    'branches:read'
);
