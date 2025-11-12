# Role-Based Authorization Middleware

Complete guide to implementing role-based access control in Chalkbyte API.

## Overview

The role middleware provides three flexible approaches for implementing role-based authorization in your Axum handlers:

1. **Layer-Based Middleware** - Apply to entire routers or route groups
2. **Extractor-Based Authorization** - Use in handler function parameters
3. **Manual Role Checking** - Runtime checks within handler logic

## Quick Start

```rust
// Import what you need
use crate::middleware::role::{
    require_system_admin,
    RequireAdmin,
    check_any_role,
};

// Apply to router
Router::new()
    .route("/admin", get(handler))
    .layer(middleware::from_fn_with_state(state.clone(), require_system_admin))

// Or use in handler
pub async fn handler(
    _require: RequireAdmin,
    auth_user: AuthUser,
) -> Result<Json<Response>, AppError> {
    // Your logic here
}
```

## Documentation

### 📖 Main Guides

- **[Role Middleware Guide](ROLE_MIDDLEWARE.md)** - Complete documentation with detailed explanations, patterns, and best practices
- **[Quick Reference](ROLE_MIDDLEWARE_QUICK_REFERENCE.md)** - Cheat sheet with code snippets and common patterns
- **[Refactoring Examples](ROLE_MIDDLEWARE_EXAMPLES.md)** - Before/after examples showing how to refactor existing code

### 💻 Code Examples

- **[Example Code](../examples/role_middleware_usage.rs)** - Comprehensive working examples demonstrating all three approaches

### 📚 Related Documentation

- **[User Roles Overview](USER_ROLES.md)** - Understanding the role hierarchy and permission matrix
- **[System Admin Implementation](SYSTEM_ADMIN_IMPLEMENTATION.md)** - Technical details about the system admin role

## Three Approaches at a Glance

### 1. Layer-Based (Router Level)

**Best for:** All routes in a group need the same role requirement

```rust
pub fn init_admin_router() -> Router<AppState> {
    Router::new()
        .route("/users", get(list_users))
        .route("/settings", get(settings))
        .layer(middleware::from_fn_with_state(state.clone(), require_admin))
}
```

**Pros:** Clean, DRY, centralized security  
**Cons:** Less flexible for mixed requirements

### 2. Extractor-Based (Handler Level)

**Best for:** Different handlers need different role requirements

```rust
pub async fn create_user(
    _require: RequireAdmin,  // Validates before handler runs
    auth_user: AuthUser,
    Json(dto): Json<CreateUserDto>,
) -> Result<Json<User>, AppError> {
    // Your logic here
}
```

**Pros:** Self-documenting, type-safe, flexible  
**Cons:** Repeated extractor in each handler

### 3. Manual Checking (In Handler Logic)

**Best for:** Complex runtime conditions and business logic

```rust
pub async fn update_user(
    auth_user: AuthUser,
    Path(user_id): Path<Uuid>,
) -> Result<Json<User>, AppError> {
    let user_role = parse_role_from_string(&auth_user.0.role)?;
    
    match user_role {
        UserRole::Student => {
            // Students can only update themselves
            if auth_user.0.sub != user_id.to_string() {
                return Err(AppError::forbidden("Access denied".to_string()));
            }
        }
        UserRole::Admin => {
            // Check school scoping
            check_same_school(&state.db, &auth_user.0.sub, &user_id).await?;
        }
        UserRole::SystemAdmin => {
            // Can update anyone
        }
        _ => return Err(AppError::forbidden("Insufficient permissions".to_string())),
    }
    
    // Your logic here
}
```

**Pros:** Maximum flexibility, fine-grained control  
**Cons:** More verbose, easier to forget checks

## Role Hierarchy

```
SystemAdmin (level 3)
    ↓ creates
Schools + Admin (level 2)
    ↓ creates
Teacher (level 1) + Student (level 0)
```

## Available Functions

### Middleware Functions

| Function | Allowed Roles |
|----------|---------------|
| `require_system_admin` | SystemAdmin only |
| `require_admin` | SystemAdmin, Admin |
| `require_teacher` | SystemAdmin, Admin, Teacher |
| `require_roles(...)` | Custom role list |

### Extractors

| Extractor | Allowed Roles |
|-----------|---------------|
| `RequireSystemAdmin` | SystemAdmin only |
| `RequireAdmin` | SystemAdmin, Admin |
| `RequireTeacher` | SystemAdmin, Admin, Teacher |

### Helper Functions

| Function | Purpose |
|----------|---------|
| `check_role(&auth_user, role)` | Check for exact role |
| `check_any_role(&auth_user, &[roles])` | Check for any of the roles |
| `parse_role_from_string(&str)` | Convert string to UserRole enum |
| `check_role_hierarchy(&role, &min)` | Check if role >= minimum level |
| `role_hierarchy_level(&role)` | Get numeric level (0-3) |

## Common Patterns

### Pattern: Self-or-Admin

Users can access their own resources, admins can access anyone's:

```rust
let is_self = auth_user.0.sub == user_id.to_string();
let is_admin = matches!(user_role, UserRole::SystemAdmin | UserRole::Admin);

if !is_self && !is_admin {
    return Err(AppError::forbidden("Access denied".to_string()));
}
```

### Pattern: School-Scoped Data

System admins see everything, school admins see their school only:

```rust
let data = match user_role {
    UserRole::SystemAdmin => get_all(&db).await?,
    UserRole::Admin => {
        let school_id = get_admin_school(&db, &auth_user.0.sub).await?;
        get_by_school(&db, school_id).await?
    }
    _ => return Err(AppError::forbidden("Admin required".to_string())),
};
```

### Pattern: Nested Routers

Different access levels for different route groups:

```rust
Router::new()
    .route("/profile", get(get_profile))  // Any authenticated user
    .nest("/admin", admin_routes().layer(require_admin))
    .nest("/system", system_routes().layer(require_system_admin))
```

## Best Practices

✅ **DO:**
- Use layer-based for uniform role requirements
- Use extractors for per-handler requirements
- Use manual checks for complex business logic
- Combine approaches when needed
- Always document role requirements in OpenAPI docs
- Scope school admins to their school_id

❌ **DON'T:**
- Use string comparisons for roles (`role == "admin"`)
- Skip authorization checks on sensitive operations
- Allow school admins to access other schools' data
- Hardcode role checks - use the provided functions
- Forget to include `AuthUser` to access user data

## Error Responses

| HTTP Code | Meaning |
|-----------|---------|
| 401 Unauthorized | Missing or invalid JWT token |
| 403 Forbidden | Valid token but insufficient role/permissions |

## Testing

```bash
# Get token for system admin
ADMIN_TOKEN=$(curl -s -X POST http://localhost:3000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"admin@system.com","password":"pass"}' \
  | jq -r '.access_token')

# Test protected endpoint
curl -H "Authorization: Bearer $ADMIN_TOKEN" \
  http://localhost:3000/api/system/settings

# Test forbidden access (should return 403)
curl -H "Authorization: Bearer $STUDENT_TOKEN" \
  http://localhost:3000/api/system/settings
```

## Migration Guide

### From Manual String Checks

**Before:**
```rust
if auth_user.0.role != "system_admin" {
    return Err(AppError::forbidden("Access denied".to_string()));
}
```

**After (Layer-based):**
```rust
// In router
.layer(middleware::from_fn_with_state(state.clone(), require_system_admin))

// In handler - no check needed!
pub async fn handler(auth_user: AuthUser) -> Result<...> {
    // Business logic only
}
```

**After (Extractor):**
```rust
pub async fn handler(
    _require: RequireSystemAdmin,
    auth_user: AuthUser,
) -> Result<...> {
    // Business logic only
}
```

## Decision Tree

```
Need role checking?
    │
    ├─ Same role for all routes?
    │   └─ Use: Layer-based middleware (cleanest)
    │
    ├─ Different roles per handler?
    │   └─ Use: Extractor-based (most declarative)
    │
    └─ Complex runtime conditions?
        └─ Use: Manual checking (most flexible)
```

## Examples Index

1. **System Admin Only Routes** → See [ROLE_MIDDLEWARE_EXAMPLES.md - Example 1](ROLE_MIDDLEWARE_EXAMPLES.md#example-1-layer-based-refactoring)
2. **Mixed Role Requirements** → See [ROLE_MIDDLEWARE_EXAMPLES.md - Example 3](ROLE_MIDDLEWARE_EXAMPLES.md#example-3-mixed-approach)
3. **Complex Authorization** → See [ROLE_MIDDLEWARE_EXAMPLES.md - Example 4](ROLE_MIDDLEWARE_EXAMPLES.md#example-4-complex-authorization-logic)
4. **School-Scoped Data** → See [ROLE_MIDDLEWARE.md - Best Practices](ROLE_MIDDLEWARE.md#best-practices)

## Implementation Details

The middleware is implemented in:
- **Source:** `src/middleware/role.rs`
- **Tests:** Inline unit tests in the module
- **Integration:** Import via `crate::middleware::role`

## Support

For questions or issues:
1. Check the [Full Documentation](ROLE_MIDDLEWARE.md)
2. Review the [Examples](../examples/role_middleware_usage.rs)
3. See existing implementations in `src/modules/schools/` and `src/modules/users/`

## Contributing

When adding new role requirements:
1. Use the existing middleware functions
2. Document in OpenAPI with proper response codes
3. Update permission matrix in [USER_ROLES.md](USER_ROLES.md)
4. Add tests for authorization failures

---

**Quick Links:**
- [Complete Guide →](ROLE_MIDDLEWARE.md)
- [Quick Reference →](ROLE_MIDDLEWARE_QUICK_REFERENCE.md)
- [Examples →](ROLE_MIDDLEWARE_EXAMPLES.md)
- [Code Examples →](../examples/role_middleware_usage.rs)