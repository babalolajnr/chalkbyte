//! Middleware modules for request processing.
//!
//! This module contains middleware and extractors for handling cross-cutting
//! concerns like authentication, authorization, and role checking.
//!
//! # Modules
//!
//! - [`auth`]: Authentication extractors and permission-based access control
//! - [`role`]: Role checking utilities and system role helpers
//!
//! # Authentication Flow
//!
//! 1. Client sends request with `Authorization: Bearer <token>` header
//! 2. `AuthUser` extractor validates the JWT and extracts claims
//! 3. Permission extractors check if the user has required permissions
//! 4. Handler executes if all checks pass
//!
//! # Example
//!
//! ```ignore
//! use crate::middleware::auth::{AuthUser, RequireUsersCreate};
//!
//! // Basic authentication (any valid token)
//! async fn get_profile(auth_user: AuthUser) -> impl IntoResponse {
//!     let user_id = auth_user.user_id()?;
//!     // ...
//! }
//!
//! // Permission-based access control
//! async fn create_user(
//!     RequireUsersCreate(auth_user): RequireUsersCreate,
//! ) -> impl IntoResponse {
//!     // Only executes if user has "users:create" permission
//! }
//! ```
//!
//! # Role Checking
//!
//! For checking system roles (system_admin, admin, teacher, student):
//!
//! ```ignore
//! use crate::middleware::role::is_system_admin_jwt;
//!
//! async fn admin_only_handler(auth_user: AuthUser) -> Result<impl IntoResponse, AppError> {
//!     if !is_system_admin_jwt(&auth_user) {
//!         return Err(AppError::forbidden("System admin required".to_string()));
//!     }
//!     // Proceed with admin operation
//! }
//! ```

pub mod auth;
pub mod role;
#[cfg(not(feature = "observability"))]
pub mod observability_stubs;
