-- Academic Sessions and Terms Migration
-- Allows schools to define their own academic calendars with flexible terms

-- ============================================
-- New Permissions
-- ============================================
INSERT INTO permissions (name, description, category) VALUES
    -- Academic Session permissions
    ('academic_sessions:create', 'Create academic sessions', 'academic_sessions'),
    ('academic_sessions:read', 'View academic sessions', 'academic_sessions'),
    ('academic_sessions:update', 'Update academic sessions', 'academic_sessions'),
    ('academic_sessions:delete', 'Delete academic sessions', 'academic_sessions'),
    -- Term permissions
    ('terms:create', 'Create terms', 'terms'),
    ('terms:read', 'View terms', 'terms'),
    ('terms:update', 'Update terms', 'terms'),
    ('terms:delete', 'Delete terms', 'terms');

-- ============================================
-- Academic Sessions Table
-- ============================================
CREATE TABLE academic_sessions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(100) NOT NULL,
    description TEXT,
    school_id UUID NOT NULL REFERENCES schools(id) ON DELETE CASCADE,
    start_date DATE NOT NULL,
    end_date DATE NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT unique_session_name_per_school UNIQUE (name, school_id),
    CONSTRAINT valid_session_dates CHECK (start_date < end_date)
);

CREATE INDEX idx_academic_sessions_school_id ON academic_sessions(school_id);
CREATE INDEX idx_academic_sessions_is_active ON academic_sessions(is_active);
CREATE INDEX idx_academic_sessions_dates ON academic_sessions(start_date, end_date);

-- Unique partial index: only one active session per school
CREATE UNIQUE INDEX idx_one_active_session_per_school 
    ON academic_sessions(school_id) 
    WHERE is_active = TRUE;

-- ============================================
-- Terms Table
-- ============================================
CREATE TABLE terms (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(100) NOT NULL,
    description TEXT,
    academic_session_id UUID NOT NULL REFERENCES academic_sessions(id) ON DELETE CASCADE,
    start_date DATE NOT NULL,
    end_date DATE NOT NULL,
    sequence INT NOT NULL DEFAULT 1,
    is_current BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT unique_term_name_per_session UNIQUE (name, academic_session_id),
    CONSTRAINT valid_term_dates CHECK (start_date < end_date),
    CONSTRAINT unique_term_sequence_per_session UNIQUE (sequence, academic_session_id)
);

CREATE INDEX idx_terms_academic_session_id ON terms(academic_session_id);
CREATE INDEX idx_terms_is_current ON terms(is_current);
CREATE INDEX idx_terms_dates ON terms(start_date, end_date);
CREATE INDEX idx_terms_sequence ON terms(sequence);

-- ============================================
-- Triggers for updated_at
-- ============================================
CREATE OR REPLACE FUNCTION update_academic_sessions_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_update_academic_sessions_updated_at
    BEFORE UPDATE ON academic_sessions
    FOR EACH ROW
    EXECUTE FUNCTION update_academic_sessions_updated_at();

CREATE OR REPLACE FUNCTION update_terms_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_update_terms_updated_at
    BEFORE UPDATE ON terms
    FOR EACH ROW
    EXECUTE FUNCTION update_terms_updated_at();

-- ============================================
-- Assign Permissions to System Admin
-- (System Admin gets ALL permissions)
-- ============================================
INSERT INTO role_permissions (role_id, permission_id)
SELECT '00000000-0000-0000-0000-000000000001', id FROM permissions
WHERE name LIKE 'academic_sessions:%' OR name LIKE 'terms:%';

-- ============================================
-- Assign Permissions to School Admin
-- ============================================
INSERT INTO role_permissions (role_id, permission_id)
SELECT '00000000-0000-0000-0000-000000000002', id FROM permissions
WHERE name IN (
    'academic_sessions:create', 'academic_sessions:read', 
    'academic_sessions:update', 'academic_sessions:delete',
    'terms:create', 'terms:read', 'terms:update', 'terms:delete'
);
