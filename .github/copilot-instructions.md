# GitHub Copilot Instructions for Chalkbyte

## Project Overview

Chalkbyte is a REST API built with Rust, Axum, and PostgreSQL that implements a hierarchical role-based access control system for managing schools, administrators, teachers, and students.

## Tech Stack

- **Language**: Rust (2021 edition)
- **Framework**: Axum 0.8
- **Database**: PostgreSQL with SQLx
- **Authentication**: JWT with bcrypt
- **Documentation**: Utoipa (OpenAPI/Swagger)
- **Validation**: validator crate
- **Async Runtime**: Tokio

## Project Structure

```
src/
├── cli/              # CLI commands (e.g., create-sysadmin)
├── config/           # Configuration modules (JWT, database)
├── middleware/       # Auth middleware and extractors
├── modules/          # Feature modules (NestJS-style architecture)
│   ├── auth/        # Authentication (login only, no registration)
│   ├── schools/     # School management
│   └── users/       # User management
├── utils/           # Utilities (errors, JWT, password hashing)
├── db.rs            # Database connection setup
├── docs.rs          # OpenAPI documentation configuration
├── main.rs          # Entry point with CLI argument handling
├── router.rs        # Main router setup
└── validator.rs     # Validation utilities

migrations/          # SQLx database migrations
docs/                # Documentation markdown files
```

## Architecture Patterns

### Module Structure (NestJS-style)

Each feature module follows this pattern:
```
module_name/
├── mod.rs           # Module exports
├── controller.rs    # HTTP handlers (routes)
├── service.rs       # Business logic
├── model.rs         # Data models, DTOs, database structs
└── router.rs        # Axum router configuration
```

### Code Style Guidelines

1. **Error Handling**: Always use `AppError` from `utils/errors.rs`
2. **Database Queries**: Use SQLx with query macros for type safety
3. **Authentication**: Use `AuthUser` extractor from middleware
4. **Documentation**: Add `#[utoipa::path]` attributes to all API endpoints
5. **Validation**: Use validator derive macros on DTOs
6. **Tracing**: Add `#[instrument]` to service methods

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

### Adding New Commands

```rust
// In main.rs
if args.len() > 1 && args[1] == "command-name" {
    handle_command(args).await;
    return;
}

// Create handler function
async fn handle_command(args: Vec<String>) {
    // Parse args
    // Connect to database
    // Execute operation
    // Print result
}
```

## Testing Patterns

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

1. Create/update model in `model.rs` with DTOs
2. Add service method in `service.rs`
3. Add controller handler in `controller.rs` with utoipa docs
4. Add route in `router.rs`
5. Update `docs.rs` to include the new path
6. Test with curl or swagger UI

### Adding a New Role Permission

1. Check in controller: `if auth_user.0.role != "required_role"`
2. Update docs/USER_ROLES.md permission matrix
3. Add test in test_system_admin.sh

### Database Migration

1. `sqlx migrate add description`
2. Write SQL in generated file
3. `sqlx migrate run`
4. Update models if schema changed

## Important Notes

- **No Public Registration**: Users created only by admins
- **School Isolation**: School admins see only their school's data
- **CLI System Admin**: Only way to create system administrators
- **Unique School Names**: Enforced at database level
- **JWT Expiry**: Configured via JWT_ACCESS_TOKEN_EXPIRY env var

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
- [ ] Added proper error handling with AppError
- [ ] Added authorization checks for protected endpoints
- [ ] Added OpenAPI documentation
- [ ] Followed the module structure pattern
- [ ] Used proper type safety with SQLx
- [ ] Added validation to DTOs where needed
- [ ] Scoped queries by school_id for school admins
- [ ] Updated relevant documentation

## When in Doubt

- Check existing patterns in `modules/schools/` or `modules/users/`
- Refer to `docs/SYSTEM_ADMIN_IMPLEMENTATION.md`
- Follow Axum and SQLx best practices
- Maintain consistency with existing code style
