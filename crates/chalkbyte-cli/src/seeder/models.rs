//! Data models for database seeding configuration.
//!
//! This module contains configuration structures for controlling how
//! test data is generated during seeding operations.

use chalkbyte_models::{BranchId, LevelId, RoleId, SchoolId};

/// Seed data for creating a school.
pub struct SchoolSeed {
    pub name: String,
    pub address: String,
}

/// Seed data for creating a level (grade).
pub struct LevelSeed {
    pub name: String,
    pub description: Option<String>,
    pub school_id: SchoolId,
}

/// Seed data for creating a branch (section).
pub struct BranchSeed {
    pub name: String,
    pub description: Option<String>,
    pub level_id: LevelId,
}

/// Seed data for creating a user.
pub struct UserSeed {
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub password_hash: String,
    pub role_id: RoleId,
    pub school_id: Option<SchoolId>,
    pub level_id: Option<LevelId>,
    pub branch_id: Option<BranchId>,
}

/// Configuration for number of staff users per school.
#[derive(Clone)]
pub struct UsersPerSchool {
    pub admins: usize,
    pub teachers: usize,
}

impl Default for UsersPerSchool {
    fn default() -> Self {
        Self {
            admins: 2,
            teachers: 5,
        }
    }
}

/// Configuration for educational levels per school.
#[derive(Clone)]
pub struct LevelsPerSchool {
    pub count: usize,
    pub branches_per_level: usize,
    pub students_per_branch: usize,
}

impl Default for LevelsPerSchool {
    fn default() -> Self {
        Self {
            count: 6,              // e.g., Grade 1-6
            branches_per_level: 3, // e.g., A, B, C
            students_per_branch: 25,
        }
    }
}

/// Complete configuration for database seeding.
#[derive(Clone, Default)]
pub struct SeedConfig {
    pub num_schools: usize,
    pub users_per_school: UsersPerSchool,
    pub levels_per_school: LevelsPerSchool,
}

impl SeedConfig {
    /// Creates a new seed configuration with the specified number of schools.
    pub fn new(num_schools: usize) -> Self {
        Self {
            num_schools,
            ..Default::default()
        }
    }

    /// Sets the users per school configuration.
    pub fn with_users(mut self, users: UsersPerSchool) -> Self {
        self.users_per_school = users;
        self
    }

    /// Sets the levels per school configuration.
    pub fn with_levels(mut self, levels: LevelsPerSchool) -> Self {
        self.levels_per_school = levels;
        self
    }

    /// Calculates total students per school.
    pub fn total_students_per_school(&self) -> usize {
        self.levels_per_school.count
            * self.levels_per_school.branches_per_level
            * self.levels_per_school.students_per_branch
    }

    /// Calculates total users per school (staff + students).
    pub fn total_users_per_school(&self) -> usize {
        self.users_per_school.admins
            + self.users_per_school.teachers
            + self.total_students_per_school()
    }
}
