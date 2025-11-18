use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi};

use crate::modules::auth::controller::ErrorResponse;
use crate::modules::auth::model::{
    ForgotPasswordRequest, LoginRequest, LoginResponse, MessageResponse, MfaRecoveryLoginRequest,
    MfaRequiredResponse, MfaVerifyLoginRequest, ResetPasswordRequest,
};
use crate::modules::mfa::model::{
    DisableMfaRequest, EnableMfaResponse, MfaStatusResponse, RegenerateMfaRecoveryCodesResponse,
    VerifyMfaRequest,
};
use crate::modules::students::model::{CreateStudentDto, Student, UpdateStudentDto};
use crate::modules::users::controller::ProfileResponse;
use crate::modules::users::model::{
    CreateSchoolDto, CreateUserDto, PaginatedSchoolsResponse, School, SchoolFilterParams, User,
    UserRole,
};
use crate::utils::pagination::{PaginationMeta, PaginationParams};

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::modules::auth::controller::login_user,
        crate::modules::auth::controller::verify_mfa_login,
        crate::modules::auth::controller::verify_mfa_recovery_login,
        crate::modules::auth::controller::forgot_password,
        crate::modules::auth::controller::reset_password,
        crate::modules::mfa::controller::get_mfa_status,
        crate::modules::mfa::controller::enable_mfa,
        crate::modules::mfa::controller::verify_mfa,
        crate::modules::mfa::controller::disable_mfa,
        crate::modules::mfa::controller::regenerate_recovery_codes,
        crate::modules::users::controller::create_user,
        crate::modules::users::controller::get_users,
        crate::modules::users::controller::get_profile,
        crate::modules::schools::controller::create_school,
        crate::modules::schools::controller::get_all_schools,
        crate::modules::schools::controller::get_school,
        crate::modules::schools::controller::delete_school,
        crate::modules::students::controller::create_student,
        crate::modules::students::controller::get_students,
        crate::modules::students::controller::get_student,
        crate::modules::students::controller::update_student,
        crate::modules::students::controller::delete_student,
    ),
    components(
        schemas(
            User,
            UserRole,
            CreateUserDto,
            School,
            CreateSchoolDto,
            LoginRequest,
            LoginResponse,
            MfaRequiredResponse,
            MfaVerifyLoginRequest,
            MfaRecoveryLoginRequest,
            ForgotPasswordRequest,
            ResetPasswordRequest,
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
        )
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "Authentication", description = "User authentication endpoints"),
        (name = "MFA", description = "Multi-factor authentication management"),
        (name = "Users", description = "User management endpoints"),
        (name = "Schools", description = "School management endpoints"),
        (name = "Students", description = "Student management endpoints")
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
