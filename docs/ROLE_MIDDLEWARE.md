# Role-Based Authorization Middleware

This document explains how to use the role-based authorization middleware in Chalkbyte.

## Overview

The role middleware provides multiple approaches for implementing role-based access control (RBAC) in your API endpoints:

1. **Layer-based middleware** - Apply to entire routers or route groups
2. **Extractor-based approach** - Use in handler function parameters
3. **Helper functions** - Manual role checking in controller logic

## Table of Contents

- [Quick Start](#quick-start)
- [Approach 1: Layer-Based Middleware](#approach-1-layer-based-middleware)
- [Approach 2: Extractor-Based Authorization](#approach-2-extractor-based-authorization)
- [Approach 3: Manual Role Checking](#approach-3-manual-role-checking)
- [Role Hierarchy](#role-hierarchy)
- [Best Practices](#best-practices)
- [Examples](#examples)

## Quick Start

### Import the middleware

```rust
use crate::middleware::role::{
    require_system_admin,
    require_admin,
    require_teacher,
    RequireSystemAdmin,
    RequireAdmin,
    RequireTeacher,
    check_role,
    check_any_role,
};
```

## Approach 1: Layer-Based Middleware

Apply role checking to entire route groups using Axum's layer system. This is the cleanest approach when all routes in a group require the same role(s).

### Apply to a Router

```rust
use axum::{Router, routing::get, middleware};
use crate::middleware::role::require_system_admin;

pub fn init_admin_router() -> Router<AppState> {
    Router::new()
        .route("/settings", get(system_settings))
        .route("/users", get(list_all_users))
        // All routes above require SystemAdmin role
        .layer(middleware::from_fn_with_state(
            state.clone(),
            require_system_admin
        ))
}
```

### Pre-built Middleware Functions

#### `require_system_admin`
Only allows users with `SystemAdmin` role.

```rust
Router::new()
    .route("/system", get(handler))
    .layer(middleware::from_fn_with_state(state.clone(), require_system_admin))
```

#### `require_admin`
Allows users with `SystemAdmin` or `Admin` roles.

```rust
Router::new()
    .route("/school-admin", get(handler))
    .layer(middleware::from_fn_with_state(state.clone(), require_admin))
```

#### `require_teacher`
Allows users with `SystemAdmin`, `Admin`, or `Teacher` roles.

```rust
Router::new()
    .route("/grades", get(handler))
    .layer(middleware::from_fn_with_state(state.clone(), require_teacher))
```

### Custom Role Combinations

For custom role requirements, use `require_roles`:

```rust
use crate::middleware::role::require_roles;
use crate::modules::users::model::UserRole;

Router::new()
    .route("/custom", get(handler))
    .layer(middleware::from_fn_with_state(
        state.clone(),
        |state, req, next| require_roles(
            state,
            req,
            next,
            vec![UserRole::Admin, UserRole::Teacher]
        )
    ))
```

### Apply to Specific Routes

You can apply middleware to specific routes within a router:

```rust
pub fn init_mixed_router() -> Router<AppState> {
    Router::new()
        // Public route (with auth but no role check)
        .route("/profile", get(get_profile))
        
        // Admin-only routes
        .route("/users", get(list_users).post(create_user))
        .layer(middleware::from_fn_with_state(state.clone(), require_admin))
}
```

## Approach 2: Extractor-Based Authorization

Use extractors in handler parameters for inline role checking. This approach is great for single handlers that need role protection.

### Using Pre-built Extractors

```rust
use crate::middleware::role::RequireSystemAdmin;
use crate::middleware::auth::AuthUser;

#[utoipa::path(
    get,
    path = "/api/system/settings",
    responses(
        (status = 200, description = "Success"),
        (status = 403, description = "Forbidden - System admin only")
    ),
    tag = "System",
    security(("bearer_auth" = []))
)]
pub async fn system_settings(
    State(state): State<AppState>,
    _require_admin: RequireSystemAdmin,  // Validates role before handler runs
    auth_user: AuthUser,
) -> Result<Json<SettingsResponse>, AppError> {
    // This code only runs if user is a SystemAdmin
    let settings = get_system_settings(&state.db).await?;
    Ok(Json(settings))
}
```

### Available Extractors

#### `RequireSystemAdmin`
```rust
pub async fn handler(
    _require: RequireSystemAdmin,
    auth_user: AuthUser,
) -> Result<Json<Response>, AppError> {
    // Only SystemAdmin can access
}
```

#### `RequireAdmin`
```rust
pub async fn handler(
    _require: RequireAdmin,
    auth_user: AuthUser,
) -> Result<Json<Response>, AppError> {
    // SystemAdmin or Admin can access
}
```

#### `RequireTeacher`
```rust
pub async fn handler(
    _require: RequireTeacher,
    auth_user: AuthUser,
) -> Result<Json<Response>, AppError> {
    // SystemAdmin, Admin, or Teacher can access
}
```

### Extractor Benefits

- **Declarative**: Role requirements are visible in function signature
- **Early rejection**: Request is rejected before handler logic runs
- **Type-safe**: Compile-time checking of required extractors
- **Per-handler**: Different handlers in the same router can have different requirements

## Approach 3: Manual Role Checking

Use helper functions for manual role checking within controller logic. This is useful when:
- Role checking depends on runtime conditions
- You need to check permissions at different points in the handler
- You want fine-grained control over error messages

### `check_role` - Single Role

```rust
use crate::middleware::role::check_role;
use crate::modules::users::model::UserRole;

pub async fn delete_user(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(user_id): Path<Uuid>,
) -> Result<Json<Response>, AppError> {
    // Manual role check
    check_role(&auth_user, UserRole::SystemAdmin)?;
    
    // Handler logic
    let deleted = delete_user_by_id(&state.db, user_id).await?;
    Ok(Json(deleted))
}
```

### `check_any_role` - Multiple Allowed Roles

```rust
use crate::middleware::role::check_any_role;
use crate::modules::users::model::UserRole;

pub async fn view_grades(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(student_id): Path<Uuid>,
) -> Result<Json<GradesResponse>, AppError> {
    // Check if user is admin or teacher
    check_any_role(&auth_user, &[
        UserRole::SystemAdmin,
        UserRole::Admin,
        UserRole::Teacher
    ])?;
    
    // Additional logic for school scoping
    let grades = get_student_grades(&state.db, student_id).await?;
    Ok(Json(grades))
}
```

### Conditional Role Checking

```rust
pub async fn get_user(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(user_id): Path<Uuid>,
) -> Result<Json<User>, AppError> {
    // Parse user's role
    let user_role = parse_role_from_string(&auth_user.0.role)?;
    
    // Students can only view their own profile
    if user_role == UserRole::Student {
        if auth_user.0.sub != user_id.to_string() {
            return Err(AppError::forbidden(
                "Students can only view their own profile".to_string()
            ));
        }
    }
    
    // Admins and teachers can view any profile in their school
    // System admins can view any profile
    
    let user = get_user_by_id(&state.db, user_id).await?;
    Ok(Json(user))
}
```

## Role Hierarchy

The system implements a hierarchical role structure:

```
SystemAdmin (level 3)
    ↓
Admin (level 2)
    ↓
Teacher (level 1)
    ↓
Student (level 0)
```

### Using Role Hierarchy

```rust
use crate::middleware::role::{check_role_hierarchy, parse_role_from_string};

pub async fn handler(auth_user: AuthUser) -> Result<Json<Response>, AppError> {
    let user_role = parse_role_from_string(&auth_user.0.role)?;
    
    // Check if user has at least Admin level access
    check_role_hierarchy(&user_role, &UserRole::Admin)?;
    
    // Handler logic
}
```

The `check_role_hierarchy` function checks if the user's role level is **at least** the required level. For example:
- `SystemAdmin` satisfies `Admin` requirement ✓
- `Admin` satisfies `Teacher` requirement ✓
- `Teacher` does NOT satisfy `Admin` requirement ✗

## Best Practices

### 1. Choose the Right Approach

- **Layer-based**: Use when all routes in a group need the same role(s)
- **Extractor-based**: Use for individual handler protection
- **Manual checking**: Use when role checks depend on runtime conditions

### 2. Combine Approaches

```rust
pub fn init_schools_router() -> Router<AppState> {
    Router::new()
        // All routes require at least Admin role
        .route("/", get(list_schools).post(create_school))
        .route("/{id}", get(get_school).delete(delete_school))
        .layer(middleware::from_fn_with_state(state.clone(), require_admin))
}

// Then in controller, add additional checks
pub async fn delete_school(
    State(state): State<AppState>,
    auth_user: AuthUser,  // Already verified as Admin by middleware
    Path(id): Path<Uuid>,
) -> Result<Json<Response>, AppError> {
    // Additional check: only SystemAdmin can delete schools
    check_role(&auth_user, UserRole::SystemAdmin)?;
    
    service::delete_school(&state.db, id).await?;
    Ok(Json(json!({"message": "School deleted"})))
}
```

### 3. Always Include AuthUser

Even when using extractors or middleware, include `AuthUser` in your handler to access user information:

```rust
pub async fn handler(
    _require: RequireAdmin,
    auth_user: AuthUser,  // Still needed to access user data
) -> Result<Json<Response>, AppError> {
    let user_id = auth_user.0.sub;
    let user_email = auth_user.0.email;
    // Use user information in handler
}
```

### 4. School Scoping Pattern

For school-scoped resources, combine role checking with school filtering:

```rust
pub async fn list_teachers(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> Result<Json<Vec<User>>, AppError> {
    let user_role = parse_role_from_string(&auth_user.0.role)?;
    
    let teachers = match user_role {
        UserRole::SystemAdmin => {
            // System admins see all teachers
            get_all_teachers(&state.db).await?
        }
        UserRole::Admin => {
            // School admins see only their school's teachers
            let school_id = get_school_id_for_admin(&state.db, &auth_user.0.sub).await?;
            get_teachers_by_school(&state.db, school_id).await?
        }
        _ => {
            return Err(AppError::forbidden(
                "Only admins can list teachers".to_string()
            ));
        }
    };
    
    Ok(Json(teachers))
}
```

### 5. OpenAPI Documentation

Always document role requirements in your API docs:

```rust
#[utoipa::path(
    post,
    path = "/api/schools",
    request_body = CreateSchoolDto,
    responses(
        (status = 200, description = "School created", body = School),
        (status = 401, description = "Unauthorized - Missing or invalid token"),
        (status = 403, description = "Forbidden - System admin role required")
    ),
    tag = "Schools",
    security(("bearer_auth" = []))
)]
pub async fn create_school(
    State(state): State<AppState>,
    _require: RequireSystemAdmin,
    auth_user: AuthUser,
    Json(dto): Json<CreateSchoolDto>,
) -> Result<Json<School>, AppError> {
    // Handler logic
}
```

## Examples

### Example 1: System Admin Only Router

```rust
use axum::{Router, routing::{get, post, delete}, middleware};
use crate::middleware::role::require_system_admin;

pub fn init_system_router() -> Router<AppState> {
    Router::new()
        .route("/schools", post(create_school).delete(delete_school))
        .route("/admins", post(create_admin))
        .route("/stats", get(system_stats))
        .layer(middleware::from_fn_with_state(state.clone(), require_system_admin))
}
```

### Example 2: Mixed Access Router

```rust
use axum::{Router, routing::{get, post}, middleware};
use crate::middleware::role::{require_admin, require_teacher};

pub fn init_users_router() -> Router<AppState> {
    Router::new()
        // Anyone authenticated can view their profile
        .route("/profile", get(get_profile))
        
        // Nest admin routes
        .nest("/admin", Router::new()
            .route("/users", get(list_users).post(create_user))
            .layer(middleware::from_fn_with_state(state.clone(), require_admin))
        )
        
        // Nest teacher routes
        .nest("/teacher", Router::new()
            .route("/students", get(list_students))
            .layer(middleware::from_fn_with_state(state.clone(), require_teacher))
        )
}
```

### Example 3: Per-Handler Protection

```rust
pub async fn get_user(
    State(state): State<AppState>,
    _require: RequireAdmin,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<User>, AppError> {
    let user = get_user_by_id(&state.db, id).await?;
    Ok(Json(user))
}

pub async fn delete_user(
    State(state): State<AppState>,
    _require: RequireSystemAdmin,  // More restrictive
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Response>, AppError> {
    delete_user_by_id(&state.db, id).await?;
    Ok(Json(json!({"message": "User deleted"})))
}
```

### Example 4: Conditional Logic

```rust
pub async fn update_user(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpdateUserDto>,
) -> Result<Json<User>, AppError> {
    let user_role = parse_role_from_string(&auth_user.0.role)?;
    
    // Users can update their own profile
    if auth_user.0.sub == id.to_string() {
        return update_own_profile(&state.db, id, dto).await;
    }
    
    // Admins can update users in their school
    if user_role == UserRole::Admin {
        let school_id = get_school_id_for_admin(&state.db, &auth_user.0.sub).await?;
        return update_user_in_school(&state.db, id, school_id, dto).await;
    }
    
    // System admins can update anyone
    if user_role == UserRole::SystemAdmin {
        return update_any_user(&state.db, id, dto).await;
    }
    
    Err(AppError::forbidden("Cannot update this user".to_string()))
}
```

## Error Responses

All role checking functions return appropriate HTTP error codes:

- **401 Unauthorized**: Missing or invalid JWT token (from `AuthUser` extractor)
- **403 Forbidden**: Valid token but insufficient role/permissions

Example error response:
```json
{
  "error": "Access denied. Required roles: [SystemAdmin], but user has role: Admin"
}
```

## Testing

When testing protected endpoints:

```bash
# Get token for system admin
ADMIN_TOKEN=$(curl -s -X POST http://localhost:3000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"admin@system.com","password":"admin123"}' \
  | jq -r '.access_token')

# Use token to access protected endpoint
curl -H "Authorization: Bearer $ADMIN_TOKEN" \
  http://localhost:3000/api/system/settings

# Test forbidden access (expect 403)
STUDENT_TOKEN=$(curl -s -X POST http://localhost:3000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"student@school.com","password":"pass"}' \
  | jq -r '.access_token')

curl -H "Authorization: Bearer $STUDENT_TOKEN" \
  http://localhost:3000/api/system/settings
# Should return 403 Forbidden
```

## Summary

The role middleware provides three flexible approaches to authorization:

1. **Layer-based** - Clean, router-level protection
2. **Extractor-based** - Declarative, handler-level protection
3. **Manual** - Fine-grained, conditional protection

Choose the approach that best fits your use case, and combine them when needed for layered security.