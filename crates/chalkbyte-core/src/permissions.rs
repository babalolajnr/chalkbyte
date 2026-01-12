//! Permission constants for the Chalkbyte API.
//!
//! This module provides centralized permission string constants for use across
//! the codebase. Using these constants instead of string literals ensures
//! consistency and makes refactoring easier.
//!
//! # Example
//!
//! ```ignore
//! use chalkbyte_core::permissions;
//!
//! if auth_user.has_permission(permissions::USERS_CREATE) {
//!     // Create user
//! }
//!
//! if auth_user.has_any_permission(&[permissions::ADMIN_READ, permissions::SUPER_READ]) {
//!     // Access granted
//! }
//! ```

// =============================================================================
// Users permissions
// =============================================================================

/// Permission to create users
pub const USERS_CREATE: &str = "users:create";
/// Permission to read users
pub const USERS_READ: &str = "users:read";
/// Permission to update users
pub const USERS_UPDATE: &str = "users:update";
/// Permission to delete users
pub const USERS_DELETE: &str = "users:delete";

// =============================================================================
// Schools permissions
// =============================================================================

/// Permission to create schools
pub const SCHOOLS_CREATE: &str = "schools:create";
/// Permission to read schools
pub const SCHOOLS_READ: &str = "schools:read";
/// Permission to update schools
pub const SCHOOLS_UPDATE: &str = "schools:update";
/// Permission to delete schools
pub const SCHOOLS_DELETE: &str = "schools:delete";

// =============================================================================
// Students permissions
// =============================================================================

/// Permission to create students
pub const STUDENTS_CREATE: &str = "students:create";
/// Permission to read students
pub const STUDENTS_READ: &str = "students:read";
/// Permission to update students
pub const STUDENTS_UPDATE: &str = "students:update";
/// Permission to delete students
pub const STUDENTS_DELETE: &str = "students:delete";

// =============================================================================
// Levels permissions
// =============================================================================

/// Permission to create levels
pub const LEVELS_CREATE: &str = "levels:create";
/// Permission to read levels
pub const LEVELS_READ: &str = "levels:read";
/// Permission to update levels
pub const LEVELS_UPDATE: &str = "levels:update";
/// Permission to delete levels
pub const LEVELS_DELETE: &str = "levels:delete";
/// Permission to assign students to levels
pub const LEVELS_ASSIGN_STUDENTS: &str = "levels:assign_students";

// =============================================================================
// Branches permissions
// =============================================================================

/// Permission to create branches
pub const BRANCHES_CREATE: &str = "branches:create";
/// Permission to read branches
pub const BRANCHES_READ: &str = "branches:read";
/// Permission to update branches
pub const BRANCHES_UPDATE: &str = "branches:update";
/// Permission to delete branches
pub const BRANCHES_DELETE: &str = "branches:delete";
/// Permission to assign students to branches
pub const BRANCHES_ASSIGN_STUDENTS: &str = "branches:assign_students";

// =============================================================================
// Roles permissions
// =============================================================================

/// Permission to create roles
pub const ROLES_CREATE: &str = "roles:create";
/// Permission to read roles
pub const ROLES_READ: &str = "roles:read";
/// Permission to update roles
pub const ROLES_UPDATE: &str = "roles:update";
/// Permission to delete roles
pub const ROLES_DELETE: &str = "roles:delete";
/// Permission to assign roles to users
pub const ROLES_ASSIGN: &str = "roles:assign";

// =============================================================================
// Reports permissions
// =============================================================================

/// Permission to view reports
pub const REPORTS_VIEW: &str = "reports:view";
/// Permission to export reports
pub const REPORTS_EXPORT: &str = "reports:export";

// =============================================================================
// Settings permissions
// =============================================================================

/// Permission to read settings
pub const SETTINGS_READ: &str = "settings:read";
/// Permission to update settings
pub const SETTINGS_UPDATE: &str = "settings:update";
