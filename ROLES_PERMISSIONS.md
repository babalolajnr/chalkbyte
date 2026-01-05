# Roles and Permissions System

This document describes the custom roles and permissions system in Chalkbyte API.

## Overview

The roles and permissions system allows:
- **System Admins**: Create system-wide roles for backoffice platform management
- **School Admins**: Create custom roles scoped to their school for managing school users

### JWT-Embedded Permissions

As of the latest update, user roles and permissions are **embedded directly in JWT tokens** during login. This provides:
- **Fast authorization** - No database queries for most permission checks
- **Granular control** - Fine-grained permissions like `users:create`, `levels:read`
- **Type-safe extractors** - Compile-time verified permission requirements in Rust

See [docs/PERMISSION_BASED_ACCESS.md](docs/PERMISSION_BASED_ACCESS.md) for implementation details.

## Database Schema

### Tables

#### `permissions`
Stores available permission types.

| Column | Type | Description |
|--------|------|-------------|
| id | UUID | Primary key |
| name | VARCHAR(100) | Unique permission name (e.g., "users:create") |
| description | TEXT | Human-readable description |
| category | VARCHAR(50) | Permission category (e.g., "users", "schools") |
| created_at | TIMESTAMPTZ | Creation timestamp |
| updated_at | TIMESTAMPTZ | Last update timestamp |

#### `roles`
Stores roles (system-wide or school-scoped).

| Column | Type | Description |
|--------|------|-------------|
| id | UUID | Primary key |
| name | VARCHAR(100) | Role name |
| description | TEXT | Role description |
| school_id | UUID | School ID (NULL for system roles) |
| is_system_role | BOOLEAN | True for system-wide roles |
| created_at | TIMESTAMPTZ | Creation timestamp |
| updated_at | TIMESTAMPTZ | Last update timestamp |

#### `role_permissions`
Junction table linking roles to permissions.

| Column | Type | Description |
|--------|------|-------------|
| id | UUID | Primary key |
| role_id | UUID | Foreign key to roles |
| permission_id | UUID | Foreign key to permissions |
| created_at | TIMESTAMPTZ | Creation timestamp |

#### `user_roles`
Assigns users to roles.

| Column | Type | Description |
|--------|------|-------------|
| id | UUID | Primary key |
| user_id | UUID | Foreign key to users |
| role_id | UUID | Foreign key to roles |
| assigned_at | TIMESTAMPTZ | Assignment timestamp |
| assigned_by | UUID | User who made the assignment |

## Available Permissions

### Users
- `users:create` - Create new users
- `users:read` - View user information
- `users:update` - Update user information
- `users:delete` - Delete users

### Schools
- `schools:create` - Create new schools
- `schools:read` - View school information
- `schools:update` - Update school information
- `schools:delete` - Delete schools

### Students
- `students:create` - Create new students
- `students:read` - View student information
- `students:update` - Update student information
- `students:delete` - Delete students

### Levels
- `levels:create` - Create new levels
- `levels:read` - View level information
- `levels:update` - Update level information
- `levels:delete` - Delete levels
- `levels:assign_students` - Assign students to levels

### Branches
- `branches:create` - Create new branches
- `branches:read` - View branch information
- `branches:update` - Update branch information
- `branches:delete` - Delete branches
- `branches:assign_students` - Assign students to branches

### Roles
- `roles:create` - Create custom roles
- `roles:read` - View roles
- `roles:update` - Update roles
- `roles:delete` - Delete roles
- `roles:assign` - Assign roles to users

### Reports
- `reports:view` - View reports and analytics
- `reports:export` - Export reports

### Settings
- `settings:read` - View settings
- `settings:update` - Update settings

## API Endpoints

### Permissions

#### List All Permissions
```
GET /api/roles/permissions
```

Query Parameters:
- `category` - Filter by permission category
- `limit` - Items per page (default: 50)
- `page` - Page number

#### Get Permission by ID
```
GET /api/roles/permissions/{id}
```

### Custom Roles

#### Create Role
```
POST /api/roles
```

Request Body:
```json
{
  "name": "School Manager",
  "description": "Can manage school settings and users",
  "school_id": "uuid-of-school",  // null for system roles (system admin only)
  "permission_ids": ["uuid-1", "uuid-2"]
}
```

#### List Roles
```
GET /api/roles
```

Query Parameters:
- `school_id` - Filter by school ID
- `is_system_role` - Filter system roles only (true/false)
- `name` - Search by name
- `limit` - Items per page
- `page` - Page number

#### Get Role by ID
```
GET /api/roles/{id}
```

#### Update Role
```
PUT /api/roles/{id}
```

Request Body:
```json
{
  "name": "Updated Role Name",
  "description": "Updated description"
}
```

#### Delete Role
```
DELETE /api/roles/{id}
```

### Role Permissions

#### Assign Permissions to Role
```
POST /api/roles/{id}/permissions
```

Request Body:
```json
{
  "permission_ids": ["uuid-1", "uuid-2", "uuid-3"]
}
```

#### Remove Permission from Role
```
DELETE /api/roles/{role_id}/permissions/{permission_id}
```

### User Role Assignments

#### Assign Role to User
```
POST /api/users/{user_id}/roles
```

Request Body:
```json
{
  "role_id": "uuid-of-role"
}
```

#### Remove Role from User
```
DELETE /api/users/{user_id}/roles/{role_id}
```

#### Get User's Roles
```
GET /api/users/{user_id}/roles
```

#### Get User's Permissions
```
GET /api/users/{user_id}/permissions
```

## Authorization Rules

### System Admin
- Can create system-wide roles (`is_system_role = true`, `school_id = NULL`)
- Can create school-scoped roles for any school
- Can assign system roles to system-level users (users without school_id)
- Can assign school roles to users in that school
- Can view and manage all roles

### School Admin
- Can only create roles for their own school
- Can only assign roles to users in their school
- Cannot create or manage system roles
- Can only view roles belonging to their school

## Usage Examples

### Creating a School Admin Role

```bash
# First, get available permissions
curl -X GET "http://localhost:3000/api/roles/permissions" \
  -H "Authorization: Bearer $TOKEN"

# Create a role with specific permissions
curl -X POST "http://localhost:3000/api/roles" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Teacher Lead",
    "description": "Lead teacher with additional permissions",
    "school_id": "school-uuid",
    "permission_ids": [
      "students-read-uuid",
      "students-update-uuid",
      "levels-read-uuid"
    ]
  }'
```

### Assigning a Role to a User

```bash
curl -X POST "http://localhost:3000/api/users/{user_id}/roles" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "role_id": "role-uuid"
  }'
```

### Checking User Permissions

```bash
curl -X GET "http://localhost:3000/api/users/{user_id}/permissions" \
  -H "Authorization: Bearer $TOKEN"
```

## Permission Checking in Code

### Using Permission Extractors (Recommended)

Use type-safe permission extractors in controllers:

```rust
use crate::middleware::auth::{RequireLevelsCreate, RequireLevelsRead};

// Requires "levels:create" permission - checked from JWT (no DB query)
pub async fn create_level(
    State(state): State<AppState>,
    RequireLevelsCreate(auth_user): RequireLevelsCreate,
    Json(dto): Json<CreateLevelDto>,
) -> Result<(StatusCode, Json<Level>), AppError> {
    let school_id = get_admin_school_id(&state.db, &auth_user).await?;
    // ...
}
```

### JWT-Based Checks (Fast, No DB)

```rust
use crate::middleware::auth::AuthUser;

// Check permission from JWT claims
if auth_user.has_permission("users:create") {
    // Has permission
}

// Check any of multiple permissions
if auth_user.has_any_permission(&["admin:full", "users:delete"]) {
    // Has at least one
}

// Check role from JWT
if auth_user.has_role(&system_roles::SYSTEM_ADMIN) {
    // Is system admin
}
```

### Database-Backed Checks (Fresh Data)

When you need to verify against the latest database state:

```rust
// Check if user has a specific permission (DB query)
let has_permission = service::user_has_permission(&db, user_id, "users:create").await?;

// Get all permissions for a user (DB query)
let permissions = service::get_user_permissions(&db, user_id).await?;
```

## Notes

- A user can have multiple custom roles
- Permissions are cumulative across all assigned roles
- Deleting a role automatically removes all user assignments (CASCADE)
- Role names must be unique within their scope (per school or system-wide)
- System roles can only be assigned to users without a school_id
- School roles can only be assigned to users belonging to that school
- **JWT tokens include role_ids and permissions** - changes take effect on next login/token refresh
- For immediate permission changes, revoke user's refresh tokens to force re-authentication