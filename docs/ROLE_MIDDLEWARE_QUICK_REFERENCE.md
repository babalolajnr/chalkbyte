# Role Middleware Quick Reference

Quick cheat sheet for using role-based authorization in Chalkbyte.

## Import Statements

```rust
// Layer-based middleware
use axum::middleware;
use crate::middleware::role::{
    require_system_admin,
    require_admin,
    require_teacher,
    require_roles,
};

// Extractor-based
use crate::middleware::role::{
    RequireSystemAdmin,
    RequireAdmin,
    RequireTeacher,
};

// Manual checking
use crate::middleware::role::{
    check_role,
    check_any_role,
    parse_role_from_string,
    check_role_hierarchy,
};

use crate::modules::users::model::UserRole;
use crate::middleware::auth::AuthUser;
```

## Three Approaches

### 1. Layer-Based (Router Level)

```rust
// Apply to entire router
pub fn init_router() -> Router<AppState> {
    Router::new()
        .route("/", get(handler))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            require_system_admin  // or require_admin, require_teacher
        ))
}

// Handler is clean
pub async fn handler(
    State(state): State<AppState>,
    auth_user: AuthUser,  // Already verified by middleware
) -> Result<Json<Response>, AppError> {
    // Business logic only
}
```

### 2. Extractor-Based (Handler Level)

```rust
// Handler with role requirement
pub async fn handler(
    State(state): State<AppState>,
    _require: RequireSystemAdmin,  // Validates before handler runs
    auth_user: AuthUser,
) -> Result<Json<Response>, AppError> {
    // Business logic only
}
```

### 3. Manual (In Handler Logic)

```rust
// Manual role check
pub async fn handler(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> Result<Json<Response>, AppError> {
    check_role(&auth_user, UserRole::SystemAdmin)?;
    // Or: check_any_role(&auth_user, &[UserRole::SystemAdmin, UserRole::Admin])?;
    
    // Business logic
}
```

## Pre-built Middleware Functions

| Function | Allowed Roles |
|----------|---------------|
| `require_system_admin` | SystemAdmin |
| `require_admin` | SystemAdmin, Admin |
| `require_teacher` | SystemAdmin, Admin, Teacher |

## Pre-built Extractors

| Extractor | Allowed Roles |
|-----------|---------------|
| `RequireSystemAdmin` | SystemAdmin |
| `RequireAdmin` | SystemAdmin, Admin |
| `RequireTeacher` | SystemAdmin, Admin, Teacher |

## Helper Functions

```rust
// Single role check
check_role(&auth_user, UserRole::SystemAdmin)?;

// Multiple roles
check_any_role(&auth_user, &[UserRole::SystemAdmin, UserRole::Admin])?;

// Parse role from JWT string
let user_role = parse_role_from_string(&auth_user.0.role)?;

// Hierarchy check (at least the required level)
check_role_hierarchy(&user_role, &UserRole::Admin)?;
```

## Role Hierarchy

```
SystemAdmin (level 3) - Full system access
    ↓
Admin (level 2) - School-scoped admin
    ↓
Teacher (level 1) - Teaching staff
    ↓
Student (level 0) - Default role
```

## Custom Role Combinations

```rust
// Custom middleware with specific roles
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

## Common Patterns

### Pattern 1: Nested Routers with Different Roles

```rust
pub fn init_router() -> Router<AppState> {
    Router::new()
        // Public authenticated routes
        .route("/profile", get(get_profile))
        
        // Admin routes
        .nest("/admin", Router::new()
            .route("/users", get(list_users))
            .layer(middleware::from_fn_with_state(state.clone(), require_admin))
        )
        
        // System admin routes
        .nest("/system", Router::new()
            .route("/settings", get(settings))
            .layer(middleware::from_fn_with_state(state.clone(), require_system_admin))
        )
}
```

### Pattern 2: Self-or-Admin Access

```rust
pub async fn get_user(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(user_id): Path<Uuid>,
) -> Result<Json<User>, AppError> {
    let user_role = parse_role_from_string(&auth_user.0.role)?;
    let is_self = auth_user.0.sub == user_id.to_string();
    let is_admin = matches!(user_role, UserRole::SystemAdmin | UserRole::Admin);
    
    if !is_self && !is_admin {
        return Err(AppError::forbidden("Access denied".to_string()));
    }
    
    // Business logic
}
```

### Pattern 3: School-Scoped Data

```rust
pub async fn list_teachers(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> Result<Json<Vec<User>>, AppError> {
    let user_role = parse_role_from_string(&auth_user.0.role)?;
    
    let teachers = match user_role {
        UserRole::SystemAdmin => {
            get_all_teachers(&state.db).await?
        }
        UserRole::Admin => {
            let school_id = get_admin_school(&state.db, &auth_user.0.sub).await?;
            get_teachers_by_school(&state.db, school_id).await?
        }
        _ => {
            return Err(AppError::forbidden("Admin required".to_string()));
        }
    };
    
    Ok(Json(teachers))
}
```

## OpenAPI Documentation

Always document role requirements:

```rust
#[utoipa::path(
    post,
    path = "/api/resources",
    request_body = CreateDto,
    responses(
        (status = 201, description = "Created", body = Resource),
        (status = 401, description = "Unauthorized - Missing/invalid token"),
        (status = 403, description = "Forbidden - Admin role required")
    ),
    tag = "Resources",
    security(("bearer_auth" = []))
)]
pub async fn create_resource(
    State(state): State<AppState>,
    _require: RequireAdmin,
    auth_user: AuthUser,
    Json(dto): Json<CreateDto>,
) -> Result<Json<Resource>, AppError> {
    // Handler logic
}
```

## Error Codes

| Code | Meaning |
|------|---------|
| 401 | Missing or invalid JWT token |
| 403 | Valid token but insufficient role/permissions |

## Testing

```bash
# Get token
TOKEN=$(curl -s -X POST http://localhost:3000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"user@example.com","password":"pass"}' \
  | jq -r '.access_token')

# Use token
curl -H "Authorization: Bearer $TOKEN" \
  http://localhost:3000/api/protected-endpoint

# Test forbidden (expect 403)
curl -H "Authorization: Bearer $STUDENT_TOKEN" \
  http://localhost:3000/api/admin-only-endpoint
```

## Decision Tree

```
Need role checking?
    │
    ├─ All routes same role?
    │   └─ Use: Layer-based middleware
    │
    ├─ Different roles per handler?
    │   └─ Use: Extractor-based
    │
    └─ Complex runtime conditions?
        └─ Use: Manual checking
```

## Best Practices

✅ **DO:**
- Use layer-based for uniform requirements
- Use extractors for per-handler requirements
- Combine approaches when needed
- Document role requirements in OpenAPI
- Include `AuthUser` to access user data

❌ **DON'T:**
- Use string comparisons (`auth_user.0.role == "admin"`)
- Skip role checks on sensitive operations
- Forget to scope school admins to their school
- Expose passwords or sensitive data in responses

## Complete Example

```rust
// Router setup
pub fn init_users_router() -> Router<AppState> {
    Router::new()
        // Anyone authenticated
        .route("/profile", get(get_profile))
        
        // Admin and above
        .route("/", get(list_users).post(create_user))
        .layer(middleware::from_fn_with_state(state.clone(), require_admin))
}

// Handler with additional check
pub async fn create_user(
    State(state): State<AppState>,
    auth_user: AuthUser,  // Verified as Admin+ by middleware
    Json(dto): Json<CreateUserDto>,
) -> Result<Json<User>, AppError> {
    // Additional business logic check
    let user_role = parse_role_from_string(&auth_user.0.role)?;
    
    if user_role == UserRole::Admin {
        // School admins create users in their school
        let school_id = get_admin_school(&state.db, &auth_user.0.sub).await?;
        dto.school_id = Some(school_id);
    }
    
    let user = UserService::create_user(&state.db, dto).await?;
    Ok(Json(user))
}
```

## See Also

- [Full Documentation](ROLE_MIDDLEWARE.md)
- [Refactoring Examples](ROLE_MIDDLEWARE_EXAMPLES.md)
- [User Roles Overview](USER_ROLES.md)
- [Example Code](../examples/role_middleware_usage.rs)