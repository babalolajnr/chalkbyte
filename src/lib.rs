//! # Chalkbyte API
//!
//! A REST API built with Rust, Axum, and PostgreSQL that implements a hierarchical
//! role-based access control system for managing schools, administrators, teachers,
//! and students.
//!
//! ## Overview
//!
//! Chalkbyte provides a complete backend solution for educational institution management
//! with features including:
//!
//! - **Authentication**: JWT-based authentication with access and refresh tokens
//! - **Multi-Factor Authentication**: TOTP-based MFA with recovery codes
//! - **Role-Based Access Control**: Hierarchical roles with granular permissions
//! - **School Management**: Multi-tenant architecture with school scoping
//! - **User Management**: Create, update, and manage users across roles
//!
//! ## Architecture
//!
//! The codebase follows a modular architecture inspired by NestJS:
//!
//! ```text
//! src/
//! ├── cli/              # CLI commands (e.g., create-sysadmin)
//! ├── config/           # Configuration modules (JWT, database, CORS)
//! ├── middleware/       # Auth middleware and extractors
//! ├── modules/          # Feature modules
//! │   ├── auth/        # Authentication (login, MFA, password reset)
//! │   ├── schools/     # School management
//! │   ├── users/       # User management
//! │   ├── roles/       # Role and permission management
//! │   ├── levels/      # Educational levels
//! │   ├── branches/    # School branches
//! │   ├── students/    # Student-specific operations
//! │   └── mfa/         # Multi-factor authentication
//! └── utils/           # Shared utilities
//! ```
//!
//! Each feature module follows a consistent structure:
//!
//! - `mod.rs`: Module exports
//! - `controller.rs`: HTTP handlers (routes)
//! - `service.rs`: Business logic
//! - `model.rs`: Data models, DTOs, database structs
//! - `router.rs`: Axum router configuration
//!
//! ## Role Hierarchy
//!
//! The system implements a hierarchical role system:
//!
//! ```text
//! System Admin (CLI-created, no school scope)
//!     ↓ creates
//! Schools + School Admins (school_id assigned)
//!     ↓ create
//! Teachers + Students (inherit school_id)
//! ```
//!
//! ### System Roles
//!
//! | Role | Scope | Description |
//! |------|-------|-------------|
//! | System Admin | Global | Full system access, created via CLI only |
//! | Admin | School | School-scoped management |
//! | Teacher | School | Teaching-related permissions |
//! | Student | School | Basic read permissions |
//!
//! ## Authentication
//!
//! The API uses JWT tokens for authentication:
//!
//! - **Access Token**: Short-lived token (default: 1 hour) for API authentication
//! - **Refresh Token**: Long-lived token (default: 7 days) for obtaining new access tokens
//! - **MFA Temp Token**: 10-minute token for completing MFA verification
//!
//! ### Token Claims
//!
//! Access tokens include:
//! - User ID and email
//! - School ID (for school-scoped users)
//! - Role IDs
//! - Permission names
//!
//! ## Quick Start
//!
//! ### Environment Variables
//!
//! ```bash
//! DATABASE_URL=postgres://user:pass@localhost/chalkbyte
//! JWT_SECRET=your-secure-secret-key
//! JWT_ACCESS_EXPIRY=3600
//! JWT_REFRESH_EXPIRY=604800
//! ```
//!
//! ### Creating a System Admin
//!
//! System admins can only be created via CLI:
//!
//! ```bash
//! cargo run --bin chalkbyte-cli -- create-sysadmin
//! ```
//!
//! ### API Documentation
//!
//! When the server is running, API documentation is available at:
//!
//! - Swagger UI: `http://localhost:3000/swagger-ui`
//! - Scalar: `http://localhost:3000/scalar`
//!
//! ## Modules
//!
//! - [`cli`]: Command-line interface utilities
//! - [`config`]: Application configuration
//! - [`docs`]: OpenAPI documentation setup
//! - [`logging`]: Distributed tracing and logging
//! - [`metrics`]: Prometheus metrics endpoint
//! - [`middleware`]: Authentication and authorization middleware
//! - [`modules`]: Feature modules (auth, users, schools, etc.)
//! - [`router`]: Main application router
//! - [`state`]: Shared application state
//! - [`utils`]: Shared utilities (errors, JWT, password hashing)
//! - [`validator`]: Request validation utilities
//!
//! ## Security Considerations
//!
//! - Passwords are hashed using bcrypt
//! - JWT secrets should be cryptographically random
//! - School admins can only access their own school's data
//! - System admins cannot be created via API (CLI only)
//! - Rate limiting is configurable for API endpoints

pub mod cli;
pub mod config;
pub mod docs;
pub mod logging;
pub mod metrics;
pub mod middleware;
pub mod modules;
pub mod router;
pub mod state;
pub mod utils;
pub mod validator;

// Re-export workspace crates for convenience
pub use chalkbyte_auth;
pub use chalkbyte_config;
pub use chalkbyte_core;
pub use chalkbyte_db;
