//! # Chalkbyte Models
//!
//! Domain models and DTOs for the Chalkbyte API.
//!
//! This crate provides all data structures used throughout the Chalkbyte application,
//! including database entities, request/response DTOs, and validation schemas.
//!
//! # Modules
//!
//! - [`auth`]: Authentication models (login, MFA, password reset)
//! - [`branches`]: School branch models
//! - [`ids`]: Strongly-typed ID newtypes for type safety
//! - [`levels`]: Educational level models
//! - [`mfa`]: Multi-factor authentication models
//! - [`roles`]: Role and permission models
//! - [`students`]: Student-specific models
//! - [`users`]: User models and system roles
//!
//! # Example
//!
//! ```ignore
//! use chalkbyte_models::users::{User, CreateUserDto, system_roles};
//! use chalkbyte_models::auth::{LoginRequest, LoginResponse};
//! use chalkbyte_models::roles::{Role, Permission};
//! use chalkbyte_models::ids::{UserId, SchoolId, RoleId};
//!
//! // NewType IDs prevent mixing up different ID types
//! fn get_user(id: UserId) { /* ... */ }
//! fn get_school(id: SchoolId) { /* ... */ }
//!
//! let user_id = UserId::new();
//! let school_id = SchoolId::new();
//!
//! get_user(user_id);    // OK
//! // get_user(school_id); // Compile error! Type mismatch.
//!
//! // Check if a role is a system role
//! if system_roles::is_system_role(&system_roles::ADMIN) {
//!     println!("This is a system role");
//! }
//! ```

pub mod auth;
pub mod branches;
pub mod ids;
pub mod levels;
pub mod mfa;
pub mod roles;
pub mod students;
pub mod users;

// Re-export ID types at crate root for convenience
pub use ids::{
    BranchId, LevelId, PermissionId, RoleId, RolePermissionId, SchoolId, UserId, UserRoleId,
};

// Re-export commonly used types at crate root for convenience
pub use auth::{
    Claims, ForgotPasswordRequest, LoginRequest, LoginResponse, LoginUser, MessageResponse,
    MfaRecoveryLoginRequest, MfaRequiredResponse, MfaTempClaims, MfaVerifyLoginRequest,
    RefreshTokenClaims, RefreshTokenRequest, ResetPasswordRequest,
};

pub use roles::{
    AssignPermissionsDto, AssignRoleToUserDto, CreateRoleDto, PaginatedPermissionsResponse,
    PaginatedRolesResponse, Permission, PermissionFilterParams, Role, RoleAssignmentResponse,
    RoleFilterParams, RolePermission, RoleWithPermissions, UpdateRoleDto, UserRole, UserWithRoles,
    generate_slug,
};

pub use users::{
    BranchInfo, ChangePasswordDto, CreateSchoolDto, CreateUserDto, LevelInfo,
    PaginatedBasicUsersResponse, PaginatedSchoolsResponse, PaginatedUsersResponse, RoleInfo,
    School, SchoolFilterParams, SchoolFullInfo, SchoolInfo, UpdateProfileDto, User,
    UserFilterParams, UserWithRelations, UserWithSchool, system_roles,
};

pub use levels::{
    AssignStudentsToLevelDto, BulkAssignResponse as LevelBulkAssignResponse, CreateLevelDto, Level,
    LevelFilterParams, LevelWithStats, MoveStudentToLevelDto, PaginatedLevelsResponse,
    UpdateLevelDto,
};

pub use branches::{
    AssignStudentsToBranchDto, Branch, BranchFilterParams, BranchWithStats,
    BulkAssignResponse as BranchBulkAssignResponse, CreateBranchDto, MoveStudentToBranchDto,
    PaginatedBranchesResponse, UpdateBranchDto,
};

pub use mfa::{
    DisableMfaRequest, EnableMfaResponse, MfaStatusResponse, RegenerateMfaRecoveryCodesResponse,
    VerifyMfaRequest,
};

pub use students::{
    CreateStudentDto, PaginatedStudentsResponse, QueryParams as StudentQueryParams, Student,
    UpdateStudentDto,
};
