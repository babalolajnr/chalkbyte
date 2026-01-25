
# GitHub Copilot Instructions for Chalkbyte

---
description: 'Rust programming language coding conventions and best practices'
applyTo: '**/*.rs'
---

# Rust Coding Conventions and Best Practices

Follow idiomatic Rust practices and community standards when writing Rust code. 

These instructions are based on [The Rust Book](https://doc.rust-lang.org/book/), [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/), [RFC 430 naming conventions](https://github.com/rust-lang/rfcs/blob/master/text/0430-finalizing-naming-conventions.md), and the broader Rust community at [users.rust-lang.org](https://users.rust-lang.org).

## General Instructions

- Always prioritize readability, safety, and maintainability.
- Use strong typing and leverage Rust's ownership system for memory safety.
- Break down complex functions into smaller, more manageable functions.
- For algorithm-related code, include explanations of the approach used.
- Write code with good maintainability practices, including comments on why certain design decisions were made.
- Handle errors gracefully using `Result<T, E>` and provide meaningful error messages.
- For external dependencies, mention their usage and purpose in documentation.
- Use consistent naming conventions following [RFC 430](https://github.com/rust-lang/rfcs/blob/master/text/0430-finalizing-naming-conventions.md).
- Write idiomatic, safe, and efficient Rust code that follows the borrow checker's rules.
- Ensure code compiles without warnings.

## Patterns to Follow

- Use modules (`mod`) and public interfaces (`pub`) to encapsulate logic.
- Handle errors properly using `?`, `match`, or `if let`.
- Use `serde` for serialization and `thiserror` or `anyhow` for custom errors.
- Implement traits to abstract services or external dependencies.
- Structure async code using `async/await` and `tokio` or `async-std`.
- Prefer enums over flags and states for type safety.
- Use builders for complex object creation.
- Split binary and library code (`main.rs` vs `lib.rs`) for testability and reuse.
- Use `rayon` for data parallelism and CPU-bound tasks.
- Use iterators instead of index-based loops as they're often faster and safer.
- Use `&str` instead of `String` for function parameters when you don't need ownership.
- Prefer borrowing and zero-copy operations to avoid unnecessary allocations.

### Ownership, Borrowing, and Lifetimes

- Prefer borrowing (`&T`) over cloning unless ownership transfer is necessary.
- Use `&mut T` when you need to modify borrowed data.
- Explicitly annotate lifetimes when the compiler cannot infer them.
- Use `Rc<T>` for single-threaded reference counting and `Arc<T>` for thread-safe reference counting.
- Use `RefCell<T>` for interior mutability in single-threaded contexts and `Mutex<T>` or `RwLock<T>` for multi-threaded contexts.

## Patterns to Avoid

- Don't use `unwrap()` or `expect()` unless absolutely necessary—prefer proper error handling.
- Avoid panics in library code—return `Result` instead.
- Don't rely on global mutable state—use dependency injection or thread-safe containers.
- Avoid deeply nested logic—refactor with functions or combinators.
- Don't ignore warnings—treat them as errors during CI.
- Avoid `unsafe` unless required and fully documented.
- Don't overuse `clone()`, use borrowing instead of cloning unless ownership transfer is needed.
- Avoid premature `collect()`, keep iterators lazy until you actually need the collection.
- Avoid unnecessary allocations—prefer borrowing and zero-copy operations.

## Code Style and Formatting

- Follow the Rust Style Guide and use `rustfmt` for automatic formatting.
- Keep lines under 100 characters when possible.
- Place function and struct documentation immediately before the item using `///`.
- Use `cargo clippy` to catch common mistakes and enforce best practices.

## Error Handling

- Use `Result<T, E>` for recoverable errors and `panic!` only for unrecoverable errors.
- Prefer `?` operator over `unwrap()` or `expect()` for error propagation.
- Create custom error types using `thiserror` or implement `std::error::Error`.
- Use `Option<T>` for values that may or may not exist.
- Provide meaningful error messages and context.
- Error types should be meaningful and well-behaved (implement standard traits).
- Validate function arguments and return appropriate errors for invalid input.

## API Design Guidelines

### Common Traits Implementation
Eagerly implement common traits where appropriate:
- `Copy`, `Clone`, `Eq`, `PartialEq`, `Ord`, `PartialOrd`, `Hash`, `Debug`, `Display`, `Default`
- Use standard conversion traits: `From`, `AsRef`, `AsMut`
- Collections should implement `FromIterator` and `Extend`
- Note: `Send` and `Sync` are auto-implemented by the compiler when safe; avoid manual implementation unless using `unsafe` code

### Type Safety and Predictability
- Use newtypes to provide static distinctions
- Arguments should convey meaning through types; prefer specific types over generic `bool` parameters
- Use `Option<T>` appropriately for truly optional values
- Functions with a clear receiver should be methods
- Only smart pointers should implement `Deref` and `DerefMut`

### Future Proofing
- Use sealed traits to protect against downstream implementations
- Structs should have private fields
- Functions should validate their arguments
- All public types must implement `Debug`

## Testing and Documentation

- Write comprehensive unit tests using `#[cfg(test)]` modules and `#[test]` annotations.
- Use test modules alongside the code they test (`mod tests { ... }`).
- Write integration tests in `tests/` directory with descriptive filenames.
- Write clear and concise comments for each function, struct, enum, and complex logic.
- Ensure functions have descriptive names and include comprehensive documentation.
- Document all public APIs with rustdoc (`///` comments) following the [API Guidelines](https://rust-lang.github.io/api-guidelines/).
- Use `#[doc(hidden)]` to hide implementation details from public documentation.
- Document error conditions, panic scenarios, and safety considerations.
- Examples should use `?` operator, not `unwrap()` or deprecated `try!` macro.

## Project Organization

- Use semantic versioning in `Cargo.toml`.
- Include comprehensive metadata: `description`, `license`, `repository`, `keywords`, `categories`.
- Use feature flags for optional functionality.
- Organize code into modules using `mod.rs` or named files.
- Keep `main.rs` or `lib.rs` minimal - move logic to modules.

## Quality Checklist

Before publishing or reviewing Rust code, ensure:

### Core Requirements
- [ ] **Naming**: Follows RFC 430 naming conventions
- [ ] **Traits**: Implements `Debug`, `Clone`, `PartialEq` where appropriate
- [ ] **Error Handling**: Uses `Result<T, E>` and provides meaningful error types
- [ ] **Documentation**: All public items have rustdoc comments with examples
- [ ] **Testing**: Comprehensive test coverage including edge cases

### Safety and Quality
- [ ] **Safety**: No unnecessary `unsafe` code, proper error handling
- [ ] **Performance**: Efficient use of iterators, minimal allocations
- [ ] **API Design**: Functions are predictable, flexible, and type-safe
- [ ] **Future Proofing**: Private fields in structs, sealed traits where appropriate
- [ ] **Tooling**: Code passes `cargo fmt`, `cargo clippy`, and `cargo test`


## Project Overview

Chalkbyte is a REST API built with Rust, Axum, and PostgreSQL that implements a hierarchical role-based access control system for managing schools, administrators, teachers, and students.

## Tech Stack

- **Language**: Rust (2024 edition)
- **Framework**: Axum 0.8
- **Database**: PostgreSQL with SQLx
- **Authentication**: JWT with bcrypt
- **Documentation**: Utoipa (OpenAPI/Swagger)
- **Validation**: validator crate
- **Async Runtime**: Tokio

## Project Structure

Chalkbyte uses a **Cargo workspace** with multiple internal crates for improved compilation speed and separation of concerns.

```
.
├── Cargo.toml           # Workspace root with workspace.dependencies
├── crates/
│   ├── chalkbyte-core/      # Shared utilities (errors, pagination, password)
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── errors.rs    # AppError type
│   │       ├── pagination.rs
│   │       ├── password.rs
│   │       └── serde.rs
│   ├── chalkbyte-config/    # Configuration modules
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── cors.rs
│   │       ├── email.rs
│   │       ├── jwt.rs
│   │       └── rate_limit.rs
│   ├── chalkbyte-db/        # Database connection setup
│   │   └── src/
│   │       └── lib.rs
│   ├── chalkbyte-auth/      # JWT claims and helpers
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── claims.rs
│   │       └── jwt.rs
│   ├── chalkbyte-models/    # Domain models and DTOs
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── auth.rs
│   │       ├── branches.rs
│   │       ├── levels.rs
│   │       ├── mfa.rs
│   │       ├── roles.rs
│   │       ├── students.rs
│   │       └── users.rs
│   └── chalkbyte-cli/       # CLI tools and database seeding
│       └── src/
│           ├── lib.rs
│           └── seeder/
│               ├── mod.rs
│               ├── models.rs
│               ├── schools.rs
│               ├── levels.rs
│               ├── branches.rs
│               └── users.rs
├── src/                     # Main application crate
│   ├── main.rs              # Entry point
│   ├── lib.rs               # Library exports
│   ├── router.rs            # Main router setup
│   ├── state.rs             # AppState definition
│   ├── docs.rs              # OpenAPI documentation configuration
│   ├── logging.rs           # Tracing/logging setup
│   ├── metrics.rs           # Metrics configuration
│   ├── validator.rs         # Validation utilities
│   ├── bin/
│   │   └── cli.rs           # CLI binary (uses chalkbyte-cli crate)
│   ├── config/              # App-level configuration
│   ├── middleware/          # Auth middleware and extractors
│   ├── modules/             # Feature modules (NestJS-style)
│   │   ├── mod.rs
│   │   ├── auth/            # Authentication (login only)
│   │   ├── branches/        # Branch management
│   │   ├── levels/          # Level management
│   │   ├── mfa/             # Multi-factor authentication
│   │   ├── roles/           # Role management
│   │   ├── schools/         # School management
│   │   ├── students/        # Student management
│   │   └── users/           # User management
│   └── utils/               # App-level utilities
├── migrations/              # SQLx database migrations
├── docs/                    # Documentation markdown files
└── tests/                   # Integration tests
```

## Workspace Crates

### `chalkbyte-core`
Shared utilities used across the application:
- `AppError` - Unified error type with Axum `IntoResponse` implementation
- `PaginationParams`, `PaginatedResponse` - Pagination utilities
- `password` - Password hashing with bcrypt

### `chalkbyte-config`
Configuration modules:
- `JwtConfig` - JWT settings from environment
- `CorsConfig` - CORS configuration
- `EmailConfig` - Email/SMTP settings
- `RateLimitConfig` - Rate limiting settings

### `chalkbyte-db`
Database connection:
- `create_pool()` - Creates SQLx PostgreSQL connection pool

### `chalkbyte-auth`
Authentication primitives:
- `Claims` - JWT claims structure
- JWT token creation and validation helpers

### `chalkbyte-models`
Domain models and DTOs for all modules:
- `users` - User, CreateUserDto, UpdateUserDto, UserResponse
- `roles` - Role, Permission, CreateRoleDto
- `auth` - LoginDto, AuthResponse, RefreshToken
- `levels` - Level, CreateLevelDto
- `branches` - Branch, CreateBranchDto
- `students` - Student, CreateStudentDto
- `mfa` - MfaSetup, MfaVerify

### `chalkbyte-cli`
CLI tools and database seeding:
- `create_system_admin()` - Create system administrator accounts
- `seeder` module - Database seeding functionality
  - `SeedConfig`, `UsersPerSchool`, `LevelsPerSchool` - Configuration types
  - `seed_all()` - Full database seeding
  - `seed_schools_only()`, `seed_levels_only()`, etc. - Individual seeding
  - `clear_all()`, `clear_users_only()`, etc. - Data clearing

## Architecture Patterns

### Module Structure (NestJS-style)

Each feature module in `src/modules/` follows this pattern:
```
module_name/
├── mod.rs           # Module exports
├── controller.rs    # HTTP handlers (routes)
├── service.rs       # Business logic
├── model.rs         # Re-exports from chalkbyte-models + any local types
└── router.rs        # Axum router configuration
```

### Importing from Workspace Crates

```rust
// Import shared types from workspace crates
use chalkbyte_core::{AppError, PaginatedResponse, PaginationParams};
use chalkbyte_config::JwtConfig;
use chalkbyte_db::create_pool;
use chalkbyte_auth::Claims;
use chalkbyte_models::users::{User, CreateUserDto, UserResponse};
use chalkbyte_cli::{create_system_admin, seeder};
```

### Code Style Guidelines

1. **Error Handling**: Always use `AppError` from `chalkbyte-core`
2. **Database Queries**: Use SQLx with query macros for type safety
3. **Authentication**: Use `AuthUser` extractor from middleware
4. **Documentation**: Add `#[utoipa::path]` attributes to all API endpoints
5. **Validation**: Use validator derive macros on DTOs
6. **Tracing**: Add `#[instrument]` to service methods
7. **Models**: Domain models live in `chalkbyte-models`, module `model.rs` re-exports them

## Role System

### Hierarchy
```
System Admin (CLI-created, no school)
    ↓ creates
Schools + School Admins (school_id assigned)
    ↓ create
Teachers + Students (school_id from admin)
```

### Role Enum
```rust
pub enum UserRole {
    SystemAdmin,  // Full system access
    Admin,        // School-scoped admin
    Teacher,      // School staff
    Student,      // Default role
}
```

### Authorization Pattern

```rust
// Check role in controller
if auth_user.0.role != "system_admin" {
    return Err(AppError::forbidden("Only system admins allowed".to_string()));
}

// Scope queries by school for school admins
if requester_role == UserRole::Admin {
    let school_id = get_school_id_for_admin(&auth_user)?;
    // Filter by school_id
}
```

## Database Conventions

### Tables
- **users**: id, first_name, last_name, email, password, role, school_id
- **schools**: id, name (UNIQUE), address

### Always Include
- `created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()`
- `updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()`
- Primary keys as UUID with `uuid_generate_v4()`
- Indexes on foreign keys and frequently queried columns

### Query Patterns

```rust
// Use query_as! for type safety with structs
let user = sqlx::query_as!(
    User,
    r#"SELECT id, email, role as "role: _", school_id FROM users WHERE id = $1"#,
    id
)
.fetch_one(db)
.await?;

// Handle unique violations
.await
.map_err(|e| {
    if let sqlx::Error::Database(db_err) = &e {
        if db_err.is_unique_violation() {
            return AppError::bad_request(anyhow::anyhow!("Already exists"));
        }
    }
    AppError::from(e)
})?;
```

## API Endpoint Patterns

### Controller Template

```rust
#[utoipa::path(
    post,
    path = "/api/resource",
    request_body = CreateDto,
    responses(
        (status = 200, description = "Success", body = Resource),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    tag = "Resources",
    security(("bearer_auth" = []))
)]
pub async fn create_resource(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(dto): Json<CreateDto>,
) -> Result<Json<Resource>, AppError> {
    // Authorization check
    // Call service
    // Return result
}
```

### Router Template

```rust
pub fn init_module_router() -> Router<AppState> {
    Router::new()
        .route("/", post(create).get(list))
        .route("/{id}", get(get_one).delete(delete))
}
```

## Security Requirements

### MUST DO
- ✅ Check authorization in controllers before service calls
- ✅ Use AuthUser extractor for protected endpoints
- ✅ Scope school admin queries to their school_id
- ✅ Hash passwords with bcrypt (never store plaintext)
- ✅ Return 403 Forbidden for authorization failures
- ✅ Return 401 Unauthorized for authentication failures

### MUST NOT DO
- ❌ Never expose passwords in API responses
- ❌ Never allow public user registration
- ❌ Never let school admins access other schools
- ❌ Never create system admins via API (CLI only)
- ❌ Never skip role checks for protected operations

## CLI Commands

### Running CLI
```bash
# Run CLI binary
cargo run --bin chalkbyte-cli -- <command>

# Examples
cargo run --bin chalkbyte-cli -- create-sysadmin
cargo run --bin chalkbyte-cli -- seed --schools 5
cargo run --bin chalkbyte-cli -- clear-seed
```

### Adding New Commands

CLI commands are defined in `src/bin/cli.rs` using clap.
The CLI binary uses the `chalkbyte-cli` crate for core functionality:
```rust
#[derive(Parser)]
#[command(name = "chalkbyte-cli")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    CreateSysadmin { ... },
    Seed { ... },
    ClearSeed,
    // Add new commands here
}
```

### Adding CLI Functionality

For reusable CLI logic, add to `crates/chalkbyte-cli/src/`:
```rust
// In chalkbyte-cli crate
pub async fn my_new_function(db: &PgPool, ...) -> Result<(), Box<dyn std::error::Error>> {
    // Implementation
}
```

## Testing Patterns

### Running Tests
```bash
# Run all workspace tests
cargo test --workspace

# Run tests for a specific crate
cargo test -p chalkbyte-core

# Run tests for main application
cargo test -p chalkbyte
```

### Manual Testing
Use curl or the test script:
```bash
# Test with curl
TOKEN=$(curl -s -X POST http://localhost:3000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"test@example.com","password":"pass"}' \
  | jq -r '.access_token')

curl -H "Authorization: Bearer $TOKEN" http://localhost:3000/api/endpoint
```

## Common Tasks

### Adding a New Endpoint

1. Add/update model types in `crates/chalkbyte-models/src/<module>.rs`
2. Re-export in module's `model.rs` if needed
3. Add service method in `service.rs`
4. Add controller handler in `controller.rs` with utoipa docs
5. Add route in `router.rs`
6. Update `docs.rs` to include the new path
7. Test with curl or swagger UI

### Adding a New Crate

1. Create directory: `crates/chalkbyte-<name>/`
2. Add `Cargo.toml` with `package` section
3. Add to workspace members in root `Cargo.toml`
4. Add to `[workspace.dependencies]` if needed by other crates
5. Add as dependency in crates that need it

### Adding a New Role Permission

1. Check in controller: `if auth_user.0.role != "required_role"`
2. Update docs/USER_ROLES.md permission matrix
3. Add test in test_system_admin.sh

### Database Migration

1. `sqlx migrate add description`
2. Write SQL in generated file
3. `sqlx migrate run`
4. Update models in `chalkbyte-models` if schema changed

## Important Notes

- **Workspace Build**: Always run `cargo build` from workspace root
- **No Public Registration**: Users created only by admins
- **School Isolation**: School admins see only their school's data
- **CLI System Admin**: Only way to create system administrators
- **Unique School Names**: Enforced at database level
- **JWT Expiry**: Configured via JWT_ACCESS_TOKEN_EXPIRY env var
- **Models Location**: All domain models/DTOs in `chalkbyte-models` crate

## Documentation

When adding features, update:
- `docs/SYSTEM_ADMIN_IMPLEMENTATION.md` - Technical details
- `docs/USER_ROLES.md` - If adding/changing roles or permissions
- `docs/QUICK_REFERENCE.md` - Add command examples
- OpenAPI docs via `#[utoipa::path]` attributes

## Debugging

- Check server logs for tracing output
- Use `#[instrument]` for function tracing
- Check database with psql or pgAdmin
- Use Swagger UI to test endpoints interactively
- SQL query logging enabled via tracing

## Code Review Checklist

Before suggesting code:
- [ ] Added proper error handling with AppError from `chalkbyte-core`
- [ ] Added authorization checks for protected endpoints
- [ ] Added OpenAPI documentation
- [ ] Followed the module structure pattern
- [ ] Used proper type safety with SQLx
- [ ] Added validation to DTOs where needed
- [ ] Scoped queries by school_id for school admins
- [ ] Updated relevant documentation
- [ ] Models placed in correct crate (`chalkbyte-models` for shared, module for local)

## When in Doubt

- Check existing patterns in `src/modules/schools/` or `src/modules/users/`
- Check workspace crates for shared utilities
- Refer to `docs/SYSTEM_ADMIN_IMPLEMENTATION.md`
- Follow Axum and SQLx best practices
- Maintain consistency with existing code style

## File Upload & File Storage

### Overview

The application supports file uploads via an abstracted file storage interface (`FileStorage` trait) that allows swapping storage backends without changing business logic.

### Current Implementation

**LocalFileStorage**: Stores files on the local filesystem in the `./uploads` directory.

Future implementations can include S3, MinIO, Google Cloud Storage, etc.

### School Logo Upload Endpoints

#### Upload/Replace School Logo
```
POST /api/schools/{school_id}/logo
Content-Type: image/{png,jpeg,webp}
Authorization: Bearer <token>
Body: <binary file data>

Response: 200 OK
{
  "id": "uuid",
  "name": "School Name",
  "logo_path": "schools/abc-123-timestamp.png",
  ...
}
```

#### Delete School Logo
```
DELETE /api/schools/{school_id}/logo
Authorization: Bearer <token>

Response: 204 No Content
```

#### Access School Logo
```
GET /files/schools/abc-123-timestamp.png

Response: 200 OK <binary image data>
```

### File Upload Restrictions

- **Supported MIME Types**: `image/png`, `image/jpeg`, `image/webp`
- **Maximum File Size**: 5 MB (configurable via `LocalFileStorage::with_max_size()`)
- **Public Access**: Files served without authentication via `/files` path
- **Storage Keys**: Timestamp-based keys prevent collisions (format: `schools/abc-123-timestamp.ext`)

### Authorization

- **System Admins**: Can upload logos for any school
- **School Admins**: Can only upload logos for their own school
- **Teachers/Students**: Cannot upload logos (403 Forbidden)

### File Storage Architecture

#### FileStorage Trait

Located in `crates/chalkbyte-core/src/file_storage.rs`:

```rust
pub trait FileStorage: Send + Sync {
    fn save<'a>(
        &'a self,
        key: &'a str,
        content: &'a [u8],
    ) -> Pin<Box<dyn Future<Output = Result<String, StorageError>> + Send + 'a>>;
    
    fn delete<'a>(
        &'a self,
        key: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), StorageError>> + Send + 'a>>;
    
    fn get_url(&self, key: &str) -> Result<String, StorageError>;
}
```

#### LocalFileStorage Implementation

```rust
pub struct LocalFileStorage {
    base_dir: PathBuf,              // Directory to store files
    base_url: String,               // Public URL prefix
    max_file_size: usize,          // Max bytes (default: 5MB)
    allowed_mime_types: Vec<String> // Allowed MIME types
}
```

### Adding Custom Storage Backends

To add a new storage backend (e.g., S3):

1. **Create implementation** of `FileStorage` trait in a new module/crate
2. **Implement methods**: `save()`, `delete()`, `get_url()`
3. **Ensure `Send + Sync`**: All async operations must return `Send` futures
4. **Update AppState initialization** to use the new backend:
   ```rust
   let file_storage: Arc<dyn FileStorage> = Arc::new(S3FileStorage::new(...));
   let state = AppState {
       file_storage,
       ...
   };
   ```

### Database Schema

Schools table includes:
- `logo_path: TEXT UNIQUE` - Storage key for the logo file
- Migration: `20260125150000_add_school_logo_path.sql`

### Important Notes

- **Public Access**: Logo files are publicly accessible (no auth required)
- **Unique Paths**: Use timestamp-based keys to prevent collisions
- **Path Safety**: Keys are validated to prevent directory traversal attacks
- **Async Operations**: All file I/O uses Tokio for non-blocking operations
- **Error Handling**: Use `StorageError` enum for consistent error reporting
- **Cache Invalidation**: School cache is invalidated on logo changes
- **Cleanup**: Old logos are deleted when replaced or when school logo is removed

