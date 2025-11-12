# Role Middleware - Refactoring Examples

This document shows practical before/after examples of refactoring existing controllers to use the role middleware.

## Table of Contents

- [Example 1: Layer-Based Refactoring](#example-1-layer-based-refactoring)
- [Example 2: Extractor-Based Refactoring](#example-2-extractor-based-refactoring)
- [Example 3: Mixed Approach](#example-3-mixed-approach)
- [Example 4: Complex Authorization Logic](#example-4-complex-authorization-logic)

---

## Example 1: Layer-Based Refactoring

### Before: Manual Role Checks in Each Handler

```rust
// src/modules/schools/controller.rs
use axum::{extract::State, Json, extract::Path};
use uuid::Uuid;
use crate::db::AppState;
use crate::middleware::auth::AuthUser;
use crate::modules::users::model::{CreateSchoolDto, School};
use crate::utils::errors::AppError;
use super::service::SchoolService;

pub async fn create_school(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(dto): Json<CreateSchoolDto>,
) -> Result<Json<School>, AppError> {
    // Manual role check - repeated in every handler
    if auth_user.0.role != "system_admin" {
        return Err(AppError::forbidden(
            "Only system admins can create schools".to_string()
        ));
    }

    let school = SchoolService::create_school(&state.db, dto).await?;
    Ok(Json(school))
}

pub async fn get_all_schools(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> Result<Json<Vec<School>>, AppError> {
    // Manual role check - duplicated code
    if auth_user.0.role != "system_admin" {
        return Err(AppError::forbidden(
            "Only system admins can view all schools".to_string()
        ));
    }

    let schools = SchoolService::get_all_schools(&state.db).await?;
    Ok(Json(schools))
}

pub async fn delete_school(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<(), AppError> {
    // Manual role check - same code repeated again
    if auth_user.0.role != "system_admin" {
        return Err(AppError::forbidden(
            "Only system admins can delete schools".to_string()
        ));
    }

    SchoolService::delete_school(&state.db, id).await?;
    Ok(())
}
```

```rust
// src/modules/schools/router.rs
use axum::{routing::{get, post}, Router};
use crate::db::AppState;
use super::controller::{create_school, delete_school, get_all_schools};

pub fn init_schools_router() -> Router<AppState> {
    Router::new()
        .route("/", post(create_school).get(get_all_schools))
        .route("/{id}", delete(delete_school))
}
```

### After: Using Layer-Based Middleware

```rust
// src/modules/schools/controller.rs
use axum::{extract::State, Json, extract::Path};
use uuid::Uuid;
use crate::db::AppState;
use crate::middleware::auth::AuthUser;
use crate::modules::users::model::{CreateSchoolDto, School};
use crate::utils::errors::AppError;
use super::service::SchoolService;

#[utoipa::path(
    post,
    path = "/api/schools",
    request_body = CreateSchoolDto,
    responses(
        (status = 201, description = "School created successfully", body = School),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - System admin only")
    ),
    tag = "Schools",
    security(("bearer_auth" = []))
)]
pub async fn create_school(
    State(state): State<AppState>,
    auth_user: AuthUser,  // Role check done by middleware
    Json(dto): Json<CreateSchoolDto>,
) -> Result<Json<School>, AppError> {
    // No manual role check needed - middleware handles it!
    let school = SchoolService::create_school(&state.db, dto).await?;
    Ok(Json(school))
}

#[utoipa::path(
    get,
    path = "/api/schools",
    responses(
        (status = 200, description = "List of all schools", body = Vec<School>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - System admin only")
    ),
    tag = "Schools",
    security(("bearer_auth" = []))
)]
pub async fn get_all_schools(
    State(state): State<AppState>,
    auth_user: AuthUser,  // Role check done by middleware
) -> Result<Json<Vec<School>>, AppError> {
    // Clean handler - focused on business logic only
    let schools = SchoolService::get_all_schools(&state.db).await?;
    Ok(Json(schools))
}

#[utoipa::path(
    delete,
    path = "/api/schools/{id}",
    params(
        ("id" = Uuid, Path, description = "School ID")
    ),
    responses(
        (status = 204, description = "School deleted successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - System admin only"),
        (status = 404, description = "School not found")
    ),
    tag = "Schools",
    security(("bearer_auth" = []))
)]
pub async fn delete_school(
    State(state): State<AppState>,
    auth_user: AuthUser,  // Role check done by middleware
    Path(id): Path<Uuid>,
) -> Result<(), AppError> {
    // Clean and simple - no authorization code needed
    SchoolService::delete_school(&state.db, id).await?;
    Ok(())
}
```

```rust
// src/modules/schools/router.rs
use axum::{routing::{get, post, delete}, Router, middleware};
use crate::db::AppState;
use crate::middleware::role::require_system_admin;
use super::controller::{create_school, delete_school, get_all_schools};

pub fn init_schools_router() -> Router<AppState> {
    Router::new()
        .route("/", post(create_school).get(get_all_schools))
        .route("/{id}", delete(delete_school))
        // Apply system admin requirement to ALL routes above
        .layer(middleware::from_fn_with_state(
            state.clone(),
            require_system_admin
        ))
}
```

### Benefits
- ✅ **DRY (Don't Repeat Yourself)**: Authorization logic defined once
- ✅ **Cleaner handlers**: Controllers focus on business logic
- ✅ **Centralized security**: Easy to audit and modify
- ✅ **Less error-prone**: Can't forget to add role checks

---

## Example 2: Extractor-Based Refactoring

### Before: Manual String Comparison

```rust
// src/modules/users/controller.rs
pub async fn create_user(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(dto): Json<CreateUserDto>,
) -> Result<Json<User>, AppError> {
    // String comparison - error-prone
    if auth_user.0.role != "system_admin" && auth_user.0.role != "admin" {
        return Err(AppError::forbidden(
            "Only admins can create users".to_string()
        ));
    }

    let user = UserService::create_user(&state.db, dto).await?;
    Ok(Json(user))
}

pub async fn delete_user(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<(), AppError> {
    // Different role requirement - must remember exact string
    if auth_user.0.role != "system_admin" {
        return Err(AppError::forbidden(
            "Only system admins can delete users".to_string()
        ));
    }

    UserService::delete_user(&state.db, id).await?;
    Ok(())
}
```

### After: Using Extractor-Based Approach

```rust
// src/modules/users/controller.rs
use crate::middleware::role::{RequireAdmin, RequireSystemAdmin};

#[utoipa::path(
    post,
    path = "/api/users",
    request_body = CreateUserDto,
    responses(
        (status = 201, description = "User created", body = User),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - Admin role required")
    ),
    tag = "Users",
    security(("bearer_auth" = []))
)]
pub async fn create_user(
    State(state): State<AppState>,
    _require: RequireAdmin,  // Type-safe role requirement
    auth_user: AuthUser,
    Json(dto): Json<CreateUserDto>,
) -> Result<Json<User>, AppError> {
    // Handler is cleaner and role requirement is visible in signature
    let user = UserService::create_user(&state.db, dto).await?;
    Ok(Json(user))
}

#[utoipa::path(
    delete,
    path = "/api/users/{id}",
    params(
        ("id" = Uuid, Path, description = "User ID")
    ),
    responses(
        (status = 204, description = "User deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - System admin only")
    ),
    tag = "Users",
    security(("bearer_auth" = []))
)]
pub async fn delete_user(
    State(state): State<AppState>,
    _require: RequireSystemAdmin,  // More restrictive requirement
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<(), AppError> {
    // Clear and type-safe
    UserService::delete_user(&state.db, id).await?;
    Ok(())
}
```

### Benefits
- ✅ **Type-safe**: Use `UserRole` enum instead of strings
- ✅ **Self-documenting**: Function signature shows requirements
- ✅ **Flexible**: Different handlers can have different requirements
- ✅ **Compile-time checks**: Rust ensures correct usage

---

## Example 3: Mixed Approach

When you have routes with different access levels, combine approaches:

```rust
// src/modules/grades/router.rs
use axum::{routing::{get, post, put, delete}, Router, middleware};
use crate::db::AppState;
use crate::middleware::role::{require_teacher, require_admin};
use super::controller::{
    get_grades,
    create_grade,
    update_grade,
    delete_grade,
    get_grade_report,
    export_all_grades,
};

pub fn init_grades_router() -> Router<AppState> {
    Router::new()
        // Public routes (authenticated users only)
        .route("/student/:student_id", get(get_grades))
        
        // Teacher routes - can create and update
        .nest("/teacher", Router::new()
            .route("/", post(create_grade))
            .route("/:id", put(update_grade))
            .layer(middleware::from_fn_with_state(
                state.clone(),
                require_teacher
            ))
        )
        
        // Admin-only routes - sensitive operations
        .nest("/admin", Router::new()
            .route("/:id", delete(delete_grade))
            .route("/report", get(get_grade_report))
            .route("/export", get(export_all_grades))
            .layer(middleware::from_fn_with_state(
                state.clone(),
                require_admin
            ))
        )
}
```

```rust
// src/modules/grades/controller.rs
use crate::middleware::role::{check_any_role, parse_role_from_string};
use crate::modules::users::model::UserRole;

pub async fn get_grades(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(student_id): Path<Uuid>,
) -> Result<Json<Vec<Grade>>, AppError> {
    let user_role = parse_role_from_string(&auth_user.0.role)?;
    
    // Students can only view their own grades
    if user_role == UserRole::Student {
        if auth_user.0.sub != student_id.to_string() {
            return Err(AppError::forbidden(
                "Students can only view their own grades".to_string()
            ));
        }
    }
    
    // Teachers and admins can view any student's grades (school scoped)
    let grades = GradeService::get_student_grades(
        &state.db,
        student_id,
        &auth_user
    ).await?;
    
    Ok(Json(grades))
}

// Teacher middleware ensures only teachers+ can access
pub async fn create_grade(
    State(state): State<AppState>,
    auth_user: AuthUser,  // Already verified as Teacher+ by middleware
    Json(dto): Json<CreateGradeDto>,
) -> Result<Json<Grade>, AppError> {
    let grade = GradeService::create_grade(&state.db, dto, &auth_user).await?;
    Ok(Json(grade))
}

// Admin middleware ensures only admins can access
pub async fn delete_grade(
    State(state): State<AppState>,
    auth_user: AuthUser,  // Already verified as Admin+ by middleware
    Path(id): Path<Uuid>,
) -> Result<(), AppError> {
    GradeService::delete_grade(&state.db, id).await?;
    Ok(())
}
```

---

## Example 4: Complex Authorization Logic

For complex scenarios with runtime conditions:

### Before: Messy Authorization Logic

```rust
pub async fn update_user(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(user_id): Path<Uuid>,
    Json(dto): Json<UpdateUserDto>,
) -> Result<Json<User>, AppError> {
    // Complex, hard-to-read authorization logic
    let requester_role = &auth_user.0.role;
    let requester_id = &auth_user.0.sub;
    
    if requester_role == "student" && requester_id != &user_id.to_string() {
        return Err(AppError::forbidden("Cannot update other users".to_string()));
    }
    
    if requester_role == "teacher" && requester_id != &user_id.to_string() {
        return Err(AppError::forbidden("Teachers can only update themselves".to_string()));
    }
    
    if requester_role == "admin" {
        // Need to check school_id matches
        let target_user = get_user(&state.db, user_id).await?;
        let requester_school = get_user_school(&state.db, requester_id).await?;
        if target_user.school_id != requester_school.id {
            return Err(AppError::forbidden("Cannot update users from other schools".to_string()));
        }
    }
    
    // Finally update the user
    let updated = UserService::update_user(&state.db, user_id, dto).await?;
    Ok(Json(updated))
}
```

### After: Using Helper Functions

```rust
use crate::middleware::role::{parse_role_from_string, check_role_hierarchy};
use crate::modules::users::model::UserRole;

pub async fn update_user(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(user_id): Path<Uuid>,
    Json(dto): Json<UpdateUserDto>,
) -> Result<Json<User>, AppError> {
    let user_role = parse_role_from_string(&auth_user.0.role)?;
    let requester_id = &auth_user.0.sub;
    
    // Clear permission checks with helper functions
    match user_role {
        UserRole::Student | UserRole::Teacher => {
            // Can only update self
            if requester_id != &user_id.to_string() {
                return Err(AppError::forbidden(
                    "You can only update your own profile".to_string()
                ));
            }
        }
        UserRole::Admin => {
            // Check school scoping
            check_same_school(&state.db, requester_id, &user_id).await?;
        }
        UserRole::SystemAdmin => {
            // Can update anyone - no additional checks
        }
    }
    
    let updated = UserService::update_user(&state.db, user_id, dto).await?;
    Ok(Json(updated))
}

// Helper function - reusable across controllers
async fn check_same_school(
    db: &PgPool,
    requester_id: &str,
    target_user_id: &Uuid,
) -> Result<(), AppError> {
    let requester = get_user_by_id(db, &Uuid::parse_str(requester_id)?).await?;
    let target = get_user_by_id(db, target_user_id).await?;
    
    match (requester.school_id, target.school_id) {
        (Some(req_school), Some(target_school)) if req_school == target_school => Ok(()),
        _ => Err(AppError::forbidden(
            "Cannot modify users from other schools".to_string()
        )),
    }
}
```

### Benefits of Refactored Version
- ✅ **Readable**: Clear match statement shows permission levels
- ✅ **Reusable**: Helper functions can be used in other controllers
- ✅ **Type-safe**: Using `UserRole` enum instead of strings
- ✅ **Maintainable**: Easy to modify rules for specific roles

---

## Quick Migration Guide

### Step 1: Identify Authorization Patterns

Look for these patterns in your code:
```rust
// Pattern 1: Direct string comparison
if auth_user.0.role != "system_admin" { ... }

// Pattern 2: Multiple role checks
if auth_user.0.role != "admin" && auth_user.0.role != "system_admin" { ... }

// Pattern 3: Complex nested conditions
if role == "admin" {
    if check_school() { ... }
}
```

### Step 2: Choose the Right Approach

- **Same role for entire router?** → Use layer-based middleware
- **Different roles per handler?** → Use extractor-based approach
- **Complex runtime logic?** → Use helper functions

### Step 3: Refactor

```rust
// Add imports
use crate::middleware::role::{
    require_system_admin,
    RequireSystemAdmin,
    check_role,
};

// Apply to router (layer-based)
Router::new()
    .route("/", get(handler))
    .layer(middleware::from_fn_with_state(state.clone(), require_system_admin))

// OR use in handler (extractor-based)
pub async fn handler(
    _require: RequireSystemAdmin,
    auth_user: AuthUser,
) -> Result<Json<Response>, AppError> {
    // ...
}

// OR use helper (manual)
pub async fn handler(auth_user: AuthUser) -> Result<Json<Response>, AppError> {
    check_role(&auth_user, UserRole::SystemAdmin)?;
    // ...
}
```

### Step 4: Test

```bash
# Test with different role tokens
./test_system_admin.sh
./test_school_admin.sh
./test_teacher.sh
./test_student.sh
```

---

## Common Patterns

### Pattern 1: Self-or-Admin

Users can access their own resources, admins can access anyone's:

```rust
pub async fn get_profile(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(user_id): Path<Uuid>,
) -> Result<Json<User>, AppError> {
    let user_role = parse_role_from_string(&auth_user.0.role)?;
    let is_self = auth_user.0.sub == user_id.to_string();
    let is_admin = matches!(user_role, UserRole::SystemAdmin | UserRole::Admin);
    
    if !is_self && !is_admin {
        return Err(AppError::forbidden(
            "Can only view your own profile".to_string()
        ));
    }
    
    let user = UserService::get_user(&state.db, user_id).await?;
    Ok(Json(user))
}
```

### Pattern 2: School-Scoped Lists

Admins see their school, system admins see everything:

```rust
pub async fn list_teachers(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> Result<Json<Vec<User>>, AppError> {
    let user_role = parse_role_from_string(&auth_user.0.role)?;
    
    let teachers = match user_role {
        UserRole::SystemAdmin => {
            UserService::get_all_teachers(&state.db).await?
        }
        UserRole::Admin => {
            let school_id = get_admin_school_id(&state.db, &auth_user.0.sub).await?;
            UserService::get_teachers_by_school(&state.db, school_id).await?
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

### Pattern 3: Progressive Restrictions

Different fields available based on role:

```rust
pub async fn get_user_details(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(user_id): Path<Uuid>,
) -> Result<Json<UserDetails>, AppError> {
    let user_role = parse_role_from_string(&auth_user.0.role)?;
    let user = UserService::get_user(&state.db, user_id).await?;
    
    let details = match user_role {
        UserRole::SystemAdmin => {
            // Full details including sensitive info
            UserDetails::full(user)
        }
        UserRole::Admin => {
            // School-related details
            check_same_school(&state.db, &auth_user.0.sub, &user_id).await?;
            UserDetails::admin_view(user)
        }
        UserRole::Teacher => {
            // Academic details only
            UserDetails::academic_view(user)
        }
        UserRole::Student => {
            // Public profile only
            UserDetails::public_view(user)
        }
    };
    
    Ok(Json(details))
}
```

---

## Summary

The role middleware provides flexible, type-safe authorization:

1. **Layer-based**: Best for uniform access requirements
2. **Extractor-based**: Best for per-handler requirements
3. **Helper functions**: Best for complex runtime logic

Choose the approach that makes your code most readable and maintainable!