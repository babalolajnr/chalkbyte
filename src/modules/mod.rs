//! Feature modules for the Chalkbyte API.
//!
//! This module contains all feature-specific modules, each implementing a
//! distinct domain area of the application. Each module follows a consistent
//! structure inspired by NestJS:
//!
//! - `mod.rs`: Module exports
//! - `controller.rs`: HTTP handlers (routes)
//! - `service.rs`: Business logic
//! - `model.rs`: Data models, DTOs, database structs
//! - `router.rs`: Axum router configuration
//!
//! # Modules
//!
//! ## Core Modules
//!
//! - [`auth`] - Authentication (login, logout, token refresh, password reset)
//! - [`users`] - User management and profile operations
//! - [`schools`] - School CRUD operations
//! - [`roles`] - Role and permission management
//!
//! ## Education Modules
//!
//! - [`levels`] - Educational levels (e.g., Grade 1, Grade 2)
//! - [`branches`] - School branches or departments
//! - [`students`] - Student-specific operations
//!
//! ## Security Modules
//!
//! - [`mfa`] - Multi-factor authentication (TOTP setup, verification, recovery)
//!
//! # Module Architecture
//!
//! Each module is self-contained and follows these principles:
//!
//! 1. **Controller**: Handles HTTP requests, extracts parameters, validates input,
//!    and delegates to services. Controllers use Axum extractors for type-safe
//!    parameter handling.
//!
//! 2. **Service**: Contains business logic and database operations. Services are
//!    stateless and receive the database pool as a parameter.
//!
//! 3. **Model**: Defines DTOs (Data Transfer Objects) for request/response,
//!    database models, and validation rules.
//!
//! 4. **Router**: Configures Axum routes and applies middleware.
//!
//! # Example Module Structure
//!
//! ```text
//! modules/users/
//! ├── mod.rs           # pub use controller::*; pub use service::*; etc.
//! ├── controller.rs    # HTTP handlers with #[utoipa::path] annotations
//! ├── service.rs       # UserService with database operations
//! ├── model.rs         # User, CreateUserDto, UserWithRelations, etc.
//! └── router.rs        # init_module_router() -> Router<AppState>
//! ```
//!
//! # Authorization
//!
//! Most endpoints require authentication and specific permissions. Use the
//! permission extractors from [`crate::middleware::auth`]:
//!
//! ```ignore
//! use crate::middleware::auth::RequireUsersCreate;
//!
//! async fn create_user(
//!     RequireUsersCreate(auth_user): RequireUsersCreate,
//!     // ...
//! ) -> Result<Json<User>, AppError> {
//!     // Handler only executes if user has "users:create" permission
//! }
//! ```
//!
//! # School Scoping
//!
//! Many operations are scoped to a specific school. School admins can only
//! access data within their own school, while system admins have global access.
//!
//! ```ignore
//! let school_id_filter = if is_system_admin(&auth_user) {
//!     None // System admin: no filtering
//! } else {
//!     Some(get_admin_school_id(&db, &auth_user).await?)
//! };
//! ```

pub mod auth;
pub mod branches;
pub mod levels;
pub mod mfa;
pub mod roles;
pub mod schools;
pub mod students;
pub mod users;
