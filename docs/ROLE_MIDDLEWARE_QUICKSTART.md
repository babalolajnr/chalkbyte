# Role Middleware - 5 Minute Quick Start

Get started with role-based authorization in Chalkbyte in 5 minutes.

## Step 1: Import (30 seconds)

Choose your approach and import what you need:

```rust
// For layer-based (router protection)
use axum::middleware;
use crate::middleware::role::require_admin;

// For extractor-based (handler protection)
use crate::middleware::role::RequireAdmin;

// For manual checks
use crate::middleware::role::check_role;
use crate::modules::users::model::UserRole;
```

## Step 2: Choose Your Approach (1 minute)

### Option A: Layer-Based (Recommended for most cases)

**Use when:** All routes in your router need the same role.

```rust
// In your router file (e.g., src/modules/schools/router.rs)
pub fn init_schools_router() -> Router<AppState> {
    Router::new()
        .route("/", post(create_school).get(list_schools))
        .route("/{id}", delete(delete_school))
        // Protect ALL routes above - only system admins can access
        .layer(middleware::from_fn_with_state(
            state.clone(),
            require_system_admin  // or require_admin, require_teacher
        ))
}
```

**Your handlers are now clean:**
```rust
pub async fn create_school(
    State(state): State<AppState>,
    auth_user: AuthUser,  // Already verified by middleware!
    Json(dto): Json<CreateSchoolDto>,
) -> Result<Json<School>, AppError> {
    // No role check needed - middleware handles it!
    let school = SchoolService::create_school(&state.db, dto).await?;
    Ok(Json(school))
}
```

### Option B: Extractor-Based

**Use when:** Different handlers need different roles.

```rust
// Just add the extractor to your handler parameters
pub async fn create_user(
    State(state): State<AppState>,
    _require: RequireAdmin,  // This validates the role!
    auth_user: AuthUser,
    Json(dto): Json<CreateUserDto>,
) -> Result<Json<User>, AppError> {
    // Handler logic - role already checked
    let user = UserService::create_user(&state.db, dto).await?;
    Ok(Json(user))
}
```

### Option C: Manual Checking

**Use when:** You need conditional logic based on role.

```rust
pub async fn update_user(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(user_id): Path<Uuid>,
) -> Result<Json<User>, AppError> {
    let user_role = parse_role_from_string(&auth_user.0.role)?;
    
    // Students can only update themselves
    if user_role == UserRole::Student {
        if auth_user.0.sub != user_id.to_string() {
            return Err(AppError::forbidden("Access denied".to_string()));
        }
    }
    
    // Your logic here
}
```

## Step 3: Available Options (1 minute)

### Pre-built Middleware Functions

| Function | Who Can Access |
|----------|---------------|
| `require_system_admin` | SystemAdmin only |
| `require_admin` | SystemAdmin, Admin |
| `require_teacher` | SystemAdmin, Admin, Teacher |

### Pre-built Extractors

| Extractor | Who Can Access |
|-----------|---------------|
| `RequireSystemAdmin` | SystemAdmin only |
| `RequireAdmin` | SystemAdmin, Admin |
| `RequireTeacher` | SystemAdmin, Admin, Teacher |

### Helper Functions

```rust
// Check exact role
check_role(&auth_user, UserRole::SystemAdmin)?;

// Check any of multiple roles
check_any_role(&auth_user, &[UserRole::SystemAdmin, UserRole::Admin])?;

// Parse role string to enum
let user_role = parse_role_from_string(&auth_user.0.role)?;
```

## Step 4: Add OpenAPI Documentation (1 minute)

Always document the role requirement:

```rust
#[utoipa::path(
    post,
    path = "/api/schools",
    request_body = CreateSchoolDto,
    responses(
        (status = 201, description = "School created", body = School),
        (status = 401, description = "Unauthorized - Missing/invalid token"),
        (status = 403, description = "Forbidden - System admin only")  // ← Document this!
    ),
    tag = "Schools",
    security(("bearer_auth" = []))
)]
pub async fn create_school(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(dto): Json<CreateSchoolDto>,
) -> Result<Json<School>, AppError> {
    // Handler logic
}
```

## Step 5: Test (1.5 minutes)

```bash
# Start server
cargo run

# In another terminal
./test_role_middleware.sh
```

Or test manually:

```bash
# Login as system admin
TOKEN=$(curl -s -X POST http://localhost:3000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"sysadmin@test.com","password":"pass"}' \
  | jq -r '.access_token')

# Test protected endpoint
curl -H "Authorization: Bearer $TOKEN" \
  http://localhost:3000/api/schools

# Test forbidden (should return 403)
curl -H "Authorization: Bearer $STUDENT_TOKEN" \
  http://localhost:3000/api/schools
```

## Common Patterns

### Pattern 1: Nested Routes with Different Roles

```rust
Router::new()
    .route("/profile", get(get_profile))  // Any authenticated user
    
    .nest("/admin", Router::new()
        .route("/users", get(list_users))
        .layer(middleware::from_fn_with_state(state.clone(), require_admin))
    )
    
    .nest("/system", Router::new()
        .route("/settings", get(settings))
        .layer(middleware::from_fn_with_state(state.clone(), require_system_admin))
    )
```

### Pattern 2: Self-or-Admin Access

```rust
let user_role = parse_role_from_string(&auth_user.0.role)?;
let is_self = auth_user.0.sub == user_id.to_string();
let is_admin = matches!(user_role, UserRole::SystemAdmin | UserRole::Admin);

if !is_self && !is_admin {
    return Err(AppError::forbidden("Access denied".to_string()));
}
```

### Pattern 3: School-Scoped Data

```rust
let user_role = parse_role_from_string(&auth_user.0.role)?;

let data = match user_role {
    UserRole::SystemAdmin => get_all(&state.db).await?,
    UserRole::Admin => {
        let school_id = get_admin_school(&state.db, &auth_user.0.sub).await?;
        get_by_school(&state.db, school_id).await?
    }
    _ => return Err(AppError::forbidden("Admin required".to_string())),
};
```

## Error Codes

| Code | Meaning |
|------|---------|
| 401 | Missing or invalid JWT token |
| 403 | Valid token but insufficient role/permissions |

## Quick Decision Guide

```
Need role checking?
    │
    ├─ All routes need same role?
    │   └─ Use: Layer-based (cleanest)
    │
    ├─ Different handlers need different roles?
    │   └─ Use: Extractor-based (declarative)
    │
    └─ Complex conditional logic?
        └─ Use: Manual checks (most flexible)
```

## Real World Example

**Before (manual checks everywhere):**
```rust
pub async fn create_school(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(dto): Json<CreateSchoolDto>,
) -> Result<Json<School>, AppError> {
    if auth_user.0.role != "system_admin" {
        return Err(AppError::forbidden("Access denied".to_string()));
    }
    let school = SchoolService::create_school(&state.db, dto).await?;
    Ok(Json(school))
}
```

**After (clean with middleware):**
```rust
// In router
Router::new()
    .route("/", post(create_school))
    .layer(middleware::from_fn_with_state(state.clone(), require_system_admin))

// Handler is now clean!
pub async fn create_school(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(dto): Json<CreateSchoolDto>,
) -> Result<Json<School>, AppError> {
    let school = SchoolService::create_school(&state.db, dto).await?;
    Ok(Json(school))
}
```

## Next Steps

- **Read full docs:** [ROLE_MIDDLEWARE.md](ROLE_MIDDLEWARE.md)
- **See examples:** [ROLE_MIDDLEWARE_EXAMPLES.md](ROLE_MIDDLEWARE_EXAMPLES.md)
- **Quick reference:** [ROLE_MIDDLEWARE_QUICK_REFERENCE.md](ROLE_MIDDLEWARE_QUICK_REFERENCE.md)
- **Code examples:** [../examples/role_middleware_usage.rs](../examples/role_middleware_usage.rs)

## Troubleshooting

**Getting 401 Unauthorized?**
- Check that you're sending the Authorization header: `Authorization: Bearer <token>`
- Verify token is valid (try logging in again)

**Getting 403 Forbidden?**
- Check the user's role in the database
- Verify you're using the right middleware/extractor
- Check the role hierarchy (Student < Teacher < Admin < SystemAdmin)

**Compilation errors?**
- Make sure you imported from `crate::middleware::role`
- Check you have `auth_user: AuthUser` parameter
- Verify `UserRole` is imported from `crate::modules::users::model`

## Summary

✅ **Import** what you need  
✅ **Choose** layer, extractor, or manual approach  
✅ **Apply** to routes or handlers  
✅ **Document** in OpenAPI  
✅ **Test** with the script

You're done! 🎉

The middleware handles all authorization checks, returning 401 for missing tokens and 403 for insufficient permissions.