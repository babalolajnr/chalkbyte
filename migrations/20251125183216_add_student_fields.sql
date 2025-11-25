ALTER TABLE users
ADD COLUMN date_of_birth DATE,
ADD COLUMN grade_level VARCHAR(10);

CREATE INDEX idx_users_grade_level ON users(grade_level) WHERE role = 'student';
CREATE INDEX idx_users_date_of_birth ON users(date_of_birth) WHERE role = 'student';
