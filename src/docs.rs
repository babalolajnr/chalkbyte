use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi};

use crate::modules::auth::controller::ErrorResponse;
use crate::modules::auth::model::{LoginRequest, LoginResponse, RegisterRequestDto};
use crate::modules::users::controller::ProfileResponse;
use crate::modules::users::model::{CreateUserDto, User};

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::modules::auth::controller::register_user,
        crate::modules::auth::controller::login_user,
        crate::modules::users::controller::create_user,
        crate::modules::users::controller::get_users,
        crate::modules::users::controller::get_profile,
    ),
    components(
        schemas(
            User,
            CreateUserDto,
            RegisterRequestDto,
            LoginRequest,
            LoginResponse,
            ProfileResponse,
            ErrorResponse,
        )
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "Authentication", description = "User authentication endpoints"),
        (name = "Users", description = "User management endpoints")
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
