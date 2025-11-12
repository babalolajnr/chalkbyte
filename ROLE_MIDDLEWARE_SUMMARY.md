# Role Middleware Implementation Summary

## What Was Implemented

A comprehensive, production-ready role-based access control (RBAC) middleware system for the Chalkbyte API that provides three flexible approaches to authorization.

## Implementation Location

**Main Implementation:**
- `src/middleware/role.rs` - Complete role middleware with all functions and extractors

**Documentation:**
- `docs/ROLE_MIDDLEWARE.md` - Complete guide with detailed explanations
- `docs/ROLE_MIDDLEWARE_QUICK_REFERENCE.md` - Quick reference cheat sheet
- `docs/ROLE_MIDDLEWARE_EXAMPLES.md` - Before/after refactoring examples
- `docs/README_ROLE_MIDDLEWARE.md` - Documentation index and overview

**Examples & Testing:**
- `examples/role_middleware_usage.rs` - Comprehensive code examples
- `test_role_middleware.sh` - Automated testing script

## Three Approaches Implemented

### 1. Layer-Based Middleware
Apply to entire routers or route groups for uniform protection.

**Functions:**
- `require_system_admin` - SystemAdmin only
- `require_admin` - SystemAdmin or Admin
- `require_teacher` - SystemAdmin, Admin, or Teacher
- `require_roles(...)` - Custom role combinations

**Usage:**
```rust
Router::new()
    .route("/admin", get(handler))
    .layer(middleware::from_fn_with_state(state.clone(), require_admin))
```

### 2. Extractor-Based Authorization
Use in handler parameters for per-handler protection.

**Extractors:**
- `RequireSystemAdmin` - SystemAdmin only
- `RequireAdmin` - SystemAdmin or Admin
- `RequireTeacher` - SystemAdmin, Admin, or Teacher

**Usage:**
```rust
pub async fn handler(
    _require: RequireAdmin,
    auth_user: AuthUser,
) -> Result<Json<Response>, AppError> {
    // Handler logic
}
```

### 3. Manual Role Checking
Helper functions for runtime conditional logic.

**Functions:**
- `check_role(&auth_user, role)` - Check exact role
- `check_any_role(&auth_user, &[roles])` - Check multiple roles
- `parse_role_from_string(&str)` - Convert string to UserRole
- `check_role_hierarchy(&role, &min)` - Check role level
- `role_hierarchy_level(&role)` - Get numeric level (0-3)

**Usage:**
```rust
pub async fn handler(auth_user: AuthUser) -> Result<...> {
    check_role(&auth_user, UserRole::SystemAdmin)?;
    // Handler logic
}
```

## Key Features

### ✅ Type Safety
- Uses `UserRole` enum instead of error-prone string comparisons
- Compile-time checking with Rust's type system
- No magic strings for role names

### ✅ Role Hierarchy
```
SystemAdmin (level 3) - Full system access
    ↓
Admin (level 2) - School-scoped administration
    ↓
Teacher (level 1) - Teaching staff
    ↓
Student (level 0) - Default user role
```

### ✅ Flexible Authorization
- Choose the approach that fits your use case
- Combine multiple approaches in the same application
- Easy to refactor from one approach to another

### ✅ Proper Error Handling
- **401 Unauthorized**: Missing or invalid JWT token
- **403 Forbidden**: Valid token but insufficient permissions
- Clear error messages for debugging

### ✅ Well Documented
- Comprehensive documentation with examples
- Quick reference for common patterns
- Real-world refactoring examples
- Testing script included

## Integration with Existing System

The middleware integrates seamlessly with existing Chalkbyte components:

- **Works with `AuthUser` extractor**: Already validates JWT tokens
- **Uses `UserRole` enum**: From `modules/users/model.rs`
- **Returns `AppError`**: Consistent with existing error handling
- **Follows Axum patterns**: Standard middleware implementation
- **OpenAPI compatible**: Works with existing utoipa documentation

## Common Patterns Supported

### Self-or-Admin Pattern
Users access their own resources, admins access anyone's:
```rust
let is_self = auth_user.0.sub == user_id.to_string();
let is_admin = matches!(user_role, UserRole::SystemAdmin | UserRole::Admin);
if !is_self && !is_admin {
    return Err(AppError::forbidden(...));
}
```

### School-Scoped Data Pattern
System admins see all, school admins see their school only:
```rust
match user_role {
    UserRole::SystemAdmin => get_all(&db).await?,
    UserRole::Admin => {
        let school_id = get_admin_school(&db, &auth_user.0.sub).await?;
        get_by_school(&db, school_id).await?
    }
    _ => return Err(AppError::forbidden(...)),
}
```

### Nested Router Pattern
Different access levels for different route groups:
```rust
Router::new()
    .nest("/admin", admin_routes().layer(require_admin))
    .nest("/system", system_routes().layer(require_system_admin))
```

## How to Use

### Quick Start

1. **Import what you need:**
```rust
use crate::middleware::role::{require_admin, RequireAdmin, check_role};
```

2. **Choose your approach:**
   - Layer-based for uniform requirements
   - Extractor-based for per-handler requirements
   - Manual for complex logic

3. **Apply to your routes/handlers:**
```rust
// Layer approach
Router::new()
    .route("/admin", get(handler))
    .layer(middleware::from_fn_with_state(state.clone(), require_admin))

// Extractor approach
pub async fn handler(_require: RequireAdmin, auth_user: AuthUser) { ... }

// Manual approach
pub async fn handler(auth_user: AuthUser) {
    check_role(&auth_user, UserRole::Admin)?;
    ...
}
```

### Refactoring Existing Code

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

// Handler - no check needed!
pub async fn handler(auth_user: AuthUser) -> Result<...> { ... }
```

## Testing

Run the comprehensive test suite:
```bash
# Start the server
cargo run

# In another terminal, run tests
./test_role_middleware.sh
```

The test script validates:
- ✅ All three authorization approaches
- ✅ Role hierarchy (SystemAdmin > Admin > Teacher > Student)
- ✅ Unauthorized access (401)
- ✅ Forbidden access (403)
- ✅ Invalid token handling
- ✅ School-scoped data access

## Documentation Index

| Document | Purpose |
|----------|---------|
| [ROLE_MIDDLEWARE.md](docs/ROLE_MIDDLEWARE.md) | Complete guide with examples and best practices |
| [ROLE_MIDDLEWARE_QUICK_REFERENCE.md](docs/ROLE_MIDDLEWARE_QUICK_REFERENCE.md) | Cheat sheet with code snippets |
| [ROLE_MIDDLEWARE_EXAMPLES.md](docs/ROLE_MIDDLEWARE_EXAMPLES.md) | Before/after refactoring examples |
| [README_ROLE_MIDDLEWARE.md](docs/README_ROLE_MIDDLEWARE.md) | Documentation overview |
| [role_middleware_usage.rs](examples/role_middleware_usage.rs) | Comprehensive code examples |

## Best Practices

✅ **DO:**
- Use layer-based for uniform role requirements across routes
- Use extractors for declarative per-handler requirements
- Use manual checks for complex business logic
- Combine approaches when needed
- Document role requirements in OpenAPI docs
- Scope school admins to their school_id

❌ **DON'T:**
- Use string comparisons for roles (`role == "admin"`)
- Skip authorization checks on sensitive operations
- Allow school admins to access other schools' data
- Hardcode role checks - use the provided functions
- Forget to include `AuthUser` to access user information

## Benefits

1. **Security:** Centralized, consistent authorization logic
2. **Maintainability:** Easy to audit and modify permissions
3. **Flexibility:** Three approaches for different use cases
4. **Type Safety:** Compile-time checks prevent errors
5. **DRY:** No repeated authorization code
6. **Clarity:** Self-documenting code with clear requirements
7. **Testability:** Easy to test authorization logic

## Next Steps

To start using the middleware in your modules:

1. **Review the documentation:**
   - Start with `docs/README_ROLE_MIDDLEWARE.md`
   - Read `docs/ROLE_MIDDLEWARE_QUICK_REFERENCE.md` for quick reference
   - Study `docs/ROLE_MIDDLEWARE_EXAMPLES.md` for refactoring patterns

2. **Examine the examples:**
   - Check `examples/role_middleware_usage.rs` for comprehensive examples
   - Look at existing implementations in `src/modules/schools/controller.rs`

3. **Refactor your code:**
   - Replace manual string checks with middleware/extractors
   - Add proper OpenAPI documentation
   - Test with `test_role_middleware.sh`

4. **Update documentation:**
   - Document role requirements in `docs/USER_ROLES.md`
   - Add examples to your module's documentation

## Implementation Notes

- **No breaking changes:** Middleware works alongside existing `AuthUser` extractor
- **Backward compatible:** Can be adopted incrementally
- **Performance:** Minimal overhead - single role check per request
- **Testing:** Unit tests included in `src/middleware/role.rs`
- **Compilation:** Successfully compiles with `cargo check --lib`

## Reference Links

Based on Axum middleware documentation:
https://github.com/tokio-rs/axum/blob/main/axum/src/docs/middleware.md

Follows Axum best practices for:
- Middleware implementation
- Extractor patterns
- Error handling
- Type safety

## Support

For help:
1. Check the documentation in `docs/`
2. Review examples in `examples/role_middleware_usage.rs`
3. Study existing implementations in `src/modules/`
4. Run `./test_role_middleware.sh` to see it in action

## Summary

This implementation provides a complete, production-ready role-based authorization system with:
- ✅ Three flexible approaches (layer, extractor, manual)
- ✅ Type-safe role checking
- ✅ Comprehensive documentation
- ✅ Working examples
- ✅ Automated testing
- ✅ Best practices and patterns
- ✅ Easy integration with existing code

The middleware is ready to use and can be adopted incrementally across the codebase.