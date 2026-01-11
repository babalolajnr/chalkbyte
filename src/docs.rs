use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi};

use crate::modules::auth::controller::ErrorResponse;
use crate::modules::auth::model::{
    ForgotPasswordRequest, LoginRequest, LoginResponse, LoginUser, MessageResponse,
    MfaRecoveryLoginRequest, MfaRequiredResponse, MfaVerifyLoginRequest, RefreshTokenRequest,
    ResetPasswordRequest,
};
use crate::modules::branches::model::{
    AssignStudentsToBranchDto, Branch, BranchFilterParams, BranchWithStats, CreateBranchDto,
    MoveStudentToBranchDto, PaginatedBranchesResponse, UpdateBranchDto,
};
use crate::modules::levels::model::{
    AssignStudentsToLevelDto, BulkAssignResponse, CreateLevelDto, Level, LevelFilterParams,
    LevelWithStats, MoveStudentToLevelDto, PaginatedLevelsResponse, UpdateLevelDto,
};
use crate::modules::mfa::model::{
    DisableMfaRequest, EnableMfaResponse, MfaStatusResponse, RegenerateMfaRecoveryCodesResponse,
    VerifyMfaRequest,
};
use crate::modules::roles::model::{
    AssignPermissionsDto, AssignRoleToUserDto, CreateRoleDto, PaginatedPermissionsResponse,
    PaginatedRolesResponse, Permission, PermissionFilterParams, Role, RoleAssignmentResponse,
    RoleFilterParams, RoleWithPermissions, UpdateRoleDto, UserRole,
};
use crate::modules::students::model::{CreateStudentDto, Student, UpdateStudentDto};
use crate::modules::users::controller::ProfileResponse;
use crate::modules::users::model::{
    ChangePasswordDto, CreateSchoolDto, CreateUserDto, PaginatedSchoolsResponse,
    PaginatedUsersResponse, School, SchoolFilterParams, SchoolFullInfo, UpdateProfileDto, User,
    UserFilterParams,
};
use chalkbyte_core::{PaginationMeta, PaginationParams};

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::modules::auth::controller::login_user,
        crate::modules::auth::controller::verify_mfa_login,
        crate::modules::auth::controller::verify_mfa_recovery_login,
        crate::modules::auth::controller::forgot_password,
        crate::modules::auth::controller::reset_password,
        crate::modules::auth::controller::refresh_token,
        crate::modules::auth::controller::logout,
        crate::modules::mfa::controller::get_mfa_status,
        crate::modules::mfa::controller::enable_mfa,
        crate::modules::mfa::controller::verify_mfa,
        crate::modules::mfa::controller::disable_mfa,
        crate::modules::mfa::controller::regenerate_recovery_codes,
        crate::modules::users::controller::create_user,
        crate::modules::users::controller::get_users,
        crate::modules::users::controller::get_profile,
        crate::modules::users::controller::update_profile,
        crate::modules::users::controller::change_password,
        crate::modules::schools::controller::create_school,
        crate::modules::schools::controller::get_all_schools,
        crate::modules::schools::controller::get_school,
        crate::modules::schools::controller::delete_school,
        crate::modules::schools::controller::get_school_students,
        crate::modules::schools::controller::get_school_admins,
        crate::modules::schools::controller::get_school_full_info,
        crate::modules::schools::controller::get_school_levels,
        crate::modules::schools::controller::get_school_level_branches,
        crate::modules::students::controller::create_student,
        crate::modules::students::controller::get_students,
        crate::modules::students::controller::get_student,
        crate::modules::students::controller::update_student,
        crate::modules::students::controller::delete_student,
        crate::modules::levels::controller::create_level,
        crate::modules::levels::controller::get_levels,
        crate::modules::levels::controller::get_level_by_id,
        crate::modules::levels::controller::update_level,
        crate::modules::levels::controller::delete_level,
        crate::modules::levels::controller::assign_students_to_level,
        crate::modules::levels::controller::get_students_in_level,
        crate::modules::levels::controller::move_student_to_level,
        crate::modules::levels::controller::remove_student_from_level,
        crate::modules::branches::controller::create_branch,
        crate::modules::branches::controller::get_branches,
        crate::modules::branches::controller::get_branch_by_id,
        crate::modules::branches::controller::update_branch,
        crate::modules::branches::controller::delete_branch,
        crate::modules::branches::controller::assign_students_to_branch,
        crate::modules::branches::controller::get_students_in_branch,
        crate::modules::branches::controller::move_student_to_branch,
        crate::modules::branches::controller::remove_student_from_branch,
        crate::modules::roles::controller::get_permissions,
        crate::modules::roles::controller::get_permission_by_id,
        crate::modules::roles::controller::create_role,
        crate::modules::roles::controller::get_roles,
        crate::modules::roles::controller::get_role_by_id,
        crate::modules::roles::controller::update_role,
        crate::modules::roles::controller::delete_role,
        crate::modules::roles::controller::assign_permissions,
        crate::modules::roles::controller::remove_permission,
        crate::modules::roles::controller::assign_role_to_user,
        crate::modules::roles::controller::remove_role_from_user,
        crate::modules::roles::controller::get_user_roles,
        crate::modules::roles::controller::get_user_permissions,
    ),
    components(
        schemas(
            User,
            CreateUserDto,
            UpdateProfileDto,
            ChangePasswordDto,
            School,
            CreateSchoolDto,
            LoginRequest,
            LoginResponse,
            LoginUser,
            MfaRequiredResponse,
            MfaVerifyLoginRequest,
            MfaRecoveryLoginRequest,
            ForgotPasswordRequest,
            ResetPasswordRequest,
            RefreshTokenRequest,
            MessageResponse,
            MfaStatusResponse,
            EnableMfaResponse,
            VerifyMfaRequest,
            DisableMfaRequest,
            RegenerateMfaRecoveryCodesResponse,
            ProfileResponse,
            ErrorResponse,
            Student,
            CreateStudentDto,
            UpdateStudentDto,
            PaginationMeta,
            PaginationParams,
            SchoolFilterParams,
            PaginatedSchoolsResponse,
            UserFilterParams,
            PaginatedUsersResponse,
            SchoolFullInfo,
            Level,
            LevelWithStats,
            CreateLevelDto,
            UpdateLevelDto,
            AssignStudentsToLevelDto,
            MoveStudentToLevelDto,
            BulkAssignResponse,
            LevelFilterParams,
            PaginatedLevelsResponse,
            Branch,
            BranchWithStats,
            CreateBranchDto,
            UpdateBranchDto,
            AssignStudentsToBranchDto,
            MoveStudentToBranchDto,
            BranchFilterParams,
            PaginatedBranchesResponse,
            Permission,
            Role,
            RoleWithPermissions,
            UserRole,
            CreateRoleDto,
            UpdateRoleDto,
            AssignPermissionsDto,
            AssignRoleToUserDto,
            RoleFilterParams,
            PermissionFilterParams,
            PaginatedRolesResponse,
            PaginatedPermissionsResponse,
            RoleAssignmentResponse,
        )
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "Authentication", description = "User authentication endpoints"),
        (name = "MFA", description = "Multi-factor authentication management"),
        (name = "Users", description = "User management endpoints"),
        (name = "Schools", description = "School management endpoints"),
        (name = "Students", description = "Student management endpoints"),
        (name = "Levels", description = "Level/Grade management endpoints"),
        (name = "Branches", description = "Branch management endpoints"),
        (name = "Roles", description = "Custom roles and permissions management")
    ),
    info(
        title = "Chalkbyte API",
        version = "0.1.0",
        description = "A modern REST API built with Rust, Axum, and PostgreSQL featuring JWT-based authentication.",
        contact(
            name = "API Support",
            email = "support@chalkbyte.com"
        ),
        license(
            name = "MIT"
        )
    )
)]
pub struct ApiDoc;

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .build(),
                ),
            )
        }
    }
}
