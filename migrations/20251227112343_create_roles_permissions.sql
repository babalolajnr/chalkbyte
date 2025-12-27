-- Permissions table: stores available permission types
CREATE TABLE permissions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(100) NOT NULL UNIQUE,
    description TEXT,
    category VARCHAR(50) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Custom roles table: stores custom roles (system-wide or school-scoped)
CREATE TABLE custom_roles (
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

-- Role permissions junction table
CREATE TABLE role_permissions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    role_id UUID NOT NULL REFERENCES custom_roles(id) ON DELETE CASCADE,
    permission_id UUID NOT NULL REFERENCES permissions(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT unique_role_permission UNIQUE (role_id, permission_id)
);

-- User custom roles assignment table
CREATE TABLE user_custom_roles (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role_id UUID NOT NULL REFERENCES custom_roles(id) ON DELETE CASCADE,
    assigned_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    assigned_by UUID REFERENCES users(id) ON DELETE SET NULL,
    CONSTRAINT unique_user_role UNIQUE (user_id, role_id)
);

-- Indexes for better query performance
CREATE INDEX idx_custom_roles_school_id ON custom_roles(school_id);
CREATE INDEX idx_custom_roles_is_system_role ON custom_roles(is_system_role);
CREATE INDEX idx_role_permissions_role_id ON role_permissions(role_id);
CREATE INDEX idx_role_permissions_permission_id ON role_permissions(permission_id);
CREATE INDEX idx_user_custom_roles_user_id ON user_custom_roles(user_id);
CREATE INDEX idx_user_custom_roles_role_id ON user_custom_roles(role_id);
CREATE INDEX idx_permissions_category ON permissions(category);

-- Seed default permissions
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

-- Trigger to update updated_at on custom_roles
CREATE OR REPLACE FUNCTION update_custom_roles_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_update_custom_roles_updated_at
    BEFORE UPDATE ON custom_roles
    FOR EACH ROW
    EXECUTE FUNCTION update_custom_roles_updated_at();

-- Trigger to update updated_at on permissions
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
