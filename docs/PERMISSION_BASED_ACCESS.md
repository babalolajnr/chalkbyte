# Permission-Based Access Control System

This document describes the refactored permission-based access control (PBAC) system in Chalkbyte API.

## Overview

The access control system has been refactored to embed user roles and permissions directly in JWT tokens. This provides:

1. **Faster authorization checks** - No database queries needed for most permission checks
2. **Granular permissions** - Fine-grained control over what users can do
3. **Type-safe extractors** - Compile-time verified permission requirements
4. **Backward compatibility** - Database-backed checks still available when fresh data is needed

## How It Works

### JWT Token Structure

When a user logs in, their JWT access token now includes:

```json
{
  "sub": "user-uuid",
  "email": "user@example.com",
  "school_id": "school-uuid-or-null",
  "role_ids": ["role-uuid-1", "role-uuid-2"],
  "permissions": ["users:read", "users:create", "levels:read"],
  "exp": 1234567890,
  "iat": 1234567800
}
```

### Permission Names

Permissions follow the format `resource:action`:

| Resource | Actions |
|----------|---------|
| users | create, read, update, delete |
| schools | create, read, update, delete |
| students | create, read, update, delete |
| levels | create, read, update, delete, assign_students |
| branches | create, read, update, delete, assign_students |
| roles | create, read, update, delete, assign |
| reports | view, export |
| settings | read, update |

## Usage in Controllers

### Using Permission Extractors (Recommended)

The simplest way to require a permission is to use the pre-defined extractors:

```rust
use crate::middleware::auth::{RequireLevelsCreate, RequireLevelsRead};

// Requires "levels:create" permission
pub async fn create_level(
    State(state): State<AppState>,
    RequireLevelsCreate(auth_user): RequireLevelsCreate,
    Json(dto): Json<CreateLevelDto>,
) -> Result<(StatusCode, Json<Level>), AppError> {
    // auth_user contains the AuthUser with all claims
    let school_id = get_admin_school_id(&state.db, &auth_user).await?;
    // ...
}

// Requires "levels:read" permission
pub async fn get_levels(
    State(state): State<AppState>,
    RequireLevelsRead(auth_user): RequireLevelsRead,
    Query(filters): Query<LevelFilterParams>,
) -> Result<Json<PaginatedLevelsResponse>, AppError> {
    // ...
}
```

### Available Permission Extractors

```rust
// Users
RequireUsersCreate, RequireUsersRead, RequireUsersUpdate, RequireUsersDelete

// Schools
RequireSchoolsCreate, RequireSchoolsRead, RequireSchoolsUpdate, RequireSchoolsDelete

// Students
RequireStudentsCreate, RequireStudentsRead, RequireStudentsUpdate, RequireStudentsDelete

// Levels
RequireLevelsCreate, RequireLevelsRead, RequireLevelsUpdate, RequireLevelsDelete
RequireLevelsAssignStudents

// Branches
RequireBranchesCreate, RequireBranchesRead, RequireBranchesUpdate, RequireBranchesDelete
RequireBranchesAssignStudents

// Roles
RequireRolesCreate, RequireRolesRead, RequireRolesUpdate, RequireRolesDelete, RequireRolesAssign

// Reports
RequireReportsView, RequireReportsExport

// Settings
RequireSettingsRead, RequireSettingsUpdate
```

### Creating Custom Permission Extractors

Use the `require_permission!` macro to create custom extractors:

```rust
use crate::require_permission;

// Creates a new extractor called RequireCustomPermission
require_permission!(RequireCustomPermission, "custom:permission");

// Use it in your handler
pub async fn custom_handler(
    State(state): State<AppState>,
    RequireCustomPermission(auth_user): RequireCustomPermission,
) -> Result<Json<Response>, AppError> {
    // ...
}
```

### Manual Permission Checks

For complex authorization logic, use the `AuthUser` methods directly:

```rust
use crate::middleware::auth::AuthUser;

pub async fn complex_handler(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> Result<Json<Response>, AppError> {
    // Check single permission
    if !auth_user.has_permission("users:create") {
        return Err(AppError::forbidden("Missing permission: users:create".to_string()));
    }

    // Check any of multiple permissions
    if !auth_user.has_any_permission(&["admin:full", "users:delete"]) {
        return Err(AppError::forbidden("Insufficient permissions".to_string()));
    }

    // Check all permissions are present
    if !auth_user.has_all_permissions(&["users:read", "users:update"]) {
        return Err(AppError::forbidden("Missing required permissions".to_string()));
    }

    // Check role
    if auth_user.has_role(&system_roles::SYSTEM_ADMIN) {
        // System admin specific logic
    }

    // Get user info from JWT
    let user_id = auth_user.user_id()?;
    let school_id = auth_user.school_id(); // Option<Uuid>
    let email = auth_user.email();

    // ...
}
```

### JWT-Based Role Checks (Fast, No DB)

```rust
use crate::middleware::role::{
    is_system_admin_jwt,
    is_admin_jwt,
    is_teacher_or_above_jwt,
    check_user_has_permission_jwt,
    check_user_has_any_role_jwt,
};

// Check roles from JWT (no database query)
if is_system_admin_jwt(&auth_user) {
    // System admin logic
}

if is_admin_jwt(&auth_user) {
    // Admin (system or school) logic
}

if is_teacher_or_above_jwt(&auth_user) {
    // Teacher, admin, or system admin
}

// Check permission from JWT
if check_user_has_permission_jwt(&auth_user, "users:create") {
    // Has permission
}
```

### Database-Backed Checks (Fresh Data)

When you need to verify against the latest database state (e.g., after role changes):

```rust
use crate::middleware::role::{
    is_system_admin,
    is_admin,
    check_user_has_permission,
    check_user_has_any_role,
};

// These perform database queries
let is_admin = is_admin(&state.db, user_id).await?;
let has_perm = check_user_has_permission(&state.db, user_id, "users:create").await?;
```

## School Scoping

### Getting School ID

For operations that require school scoping:

```rust
use crate::middleware::role::get_admin_school_id;

pub async fn school_scoped_operation(
    State(state): State<AppState>,
    RequireLevelsCreate(auth_user): RequireLevelsCreate,
) -> Result<Json<Response>, AppError> {
    // Gets school_id from JWT claims, or errors for system admins
    let school_id = get_admin_school_id(&state.db, &auth_user).await?;
    
    // For system admins who need to specify a school:
    // They should pass school_id in the request body/query
}
```

### System Admin Operations

System admins don't have a `school_id` in their JWT. For school-scoped operations:

```rust
use crate::utils::auth_helpers::get_school_id_with_override;

pub async fn admin_operation(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Query(params): Query<OperationParams>,
) -> Result<Json<Response>, AppError> {
    // System admins can specify school_id in params
    // School admins use their own school_id
    let school_id = get_school_id_with_override(
        &state.db,
        &auth_user,
        params.school_id,
    ).await?;
    
    // ...
}
```

## Migration Guide

### Before (Role-Based)

```rust
use crate::middleware::role::{get_user_id_from_auth, is_admin};

async fn require_admin_access(db: &PgPool, auth_user: &AuthUser) -> Result<Uuid, AppError> {
    let user_id = get_user_id_from_auth(auth_user)?;
    if !is_admin(db, user_id).await? {
        return Err(AppError::forbidden("Only school admins...".to_string()));
    }
    get_admin_school_id(db, auth_user).await
}

pub async fn create_level(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(dto): Json<CreateLevelDto>,
) -> Result<(StatusCode, Json<Level>), AppError> {
    let school_id = require_admin_access(&state.db, &auth_user).await?;
    // ...
}
```

### After (Permission-Based)

```rust
use crate::middleware::auth::RequireLevelsCreate;
use crate::middleware::role::get_admin_school_id;

pub async fn create_level(
    State(state): State<AppState>,
    RequireLevelsCreate(auth_user): RequireLevelsCreate,
    Json(dto): Json<CreateLevelDto>,
) -> Result<(StatusCode, Json<Level>), AppError> {
    let school_id = get_admin_school_id(&state.db, &auth_user).await?;
    // ...
}
```

## Best Practices

1. **Prefer Permission Extractors**: Use `RequireXxxYyy` extractors for automatic permission checking
2. **Use JWT Checks First**: For performance, check JWT-embedded data before hitting the database
3. **Database Checks for Sensitive Operations**: Use DB checks when you need guaranteed fresh data
4. **Be Specific**: Use granular permissions (`levels:create`) instead of broad role checks
5. **Document Permissions**: Update OpenAPI docs to indicate required permissions

## Token Refresh

When a user's roles or permissions change:

1. Changes take effect on next login or token refresh
2. Current tokens remain valid until expiration
3. For immediate effect, revoke all user's refresh tokens

```rust
// Force user to re-authenticate
AuthService::revoke_all_refresh_tokens(&db, user_id).await?;
```

## Error Responses

Permission failures return:

```json
{
  "status": 403,
  "error": "Access denied. Missing required permission: levels:create"
}
```

Authentication failures return:

```json
{
  "status": 401,
  "error": "Invalid or expired token"
}
```

## See Also

- [ROLES_PERMISSIONS.md](../ROLES_PERMISSIONS.md) - Permission definitions and role management
- [docs/USER_ROLES.md](USER_ROLES.md) - User role hierarchy
- [docs/SYSTEM_ADMIN_IMPLEMENTATION.md](SYSTEM_ADMIN_IMPLEMENTATION.md) - System admin details