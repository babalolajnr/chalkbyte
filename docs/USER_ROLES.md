# User Roles

This document describes the user role system implemented in the application.

## Available Roles

The system supports four user roles with hierarchical permissions:

1. **System Admin** - Super administrator with complete system access
2. **Admin** - School administrator with school-scoped permissions
3. **Teacher** - Teaching staff with elevated permissions
4. **Student** - Standard user with basic access (default role)

## Role Hierarchy & Permissions

### System Admin
- Can create and manage schools
- Can create admins, teachers, and students for any school
- Has access to all system resources
- Not tied to any specific school

### Admin (School Admin)
- Tied to a specific school via `school_id`
- Can create teachers, students, and other admins for their school only
- Can only view and manage users within their school
- Cannot create system admins
- Cannot create users for other schools

### Teacher
- Tied to a specific school via `school_id`
- Has elevated permissions within their school
- Cannot create or manage users

### Student
- Tied to a specific school via `school_id`
- Standard user with basic access

## Implementation Details

### Database Schema

- A PostgreSQL enum type `user_role` is created with values: `system_admin`, `admin`, `teacher`, `student`
- The `users` table has a `role` column of type `user_role` with a default value of `student`
- The `users` table has a `school_id` column referencing the `schools` table
- A `schools` table stores school information
- Indexes are created on the `role` and `school_id` columns for optimized queries

### API Usage

#### School Creation (System Admin Only)

System admins can create schools:

```json
POST /api/schools
{
  "name": "Springfield High School",
  "address": "123 Main St, Springfield"
}
```

#### User Creation

When creating a user via `/api/users`, you can specify role and school:

```json
POST /api/users
{
  "first_name": "John",
  "last_name": "Doe",
  "email": "john@example.com",
  "role": "teacher",
  "school_id": "uuid-of-school"
}
```

**Authorization rules:**
- Only `system_admin` and `admin` roles can create users
- School admins can only create users for their own school
- School admins cannot create `system_admin` users
- System admins can create users for any school

#### Creating the First System Admin

System admins must be created via CLI command:

```bash
cargo run -- create-sysadmin Super Admin admin@system.com securepassword123
```

This ensures only authorized personnel can create system administrators.

#### JWT Token

User roles are included in JWT tokens as part of the claims. The `role` field in the JWT payload will contain one of: `system_admin`, `admin`, `teacher`, or `student`.

#### User Response

All user objects returned by the API will include the `role` and `school_id` fields:

```json
{
  "id": "uuid",
  "first_name": "John",
  "last_name": "Doe",
  "email": "john@example.com",
  "role": "teacher",
  "school_id": "uuid-of-school"
}
```

## Migrations

The role system was added via two migrations:

1. `20251111195802_add_user_roles.sql`:
   - Created the `user_role` enum type with `admin`, `teacher`, `student`
   - Added the `role` column to the `users` table
   - All existing users are assigned the default role of `student`

2. `20251112081331_add_system_admin_and_schools.sql`:
   - Added `system_admin` to the `user_role` enum
   - Created the `schools` table
   - Added `school_id` column to `users` table
   - Created indexes for optimized queries

## API Endpoints

### Schools (System Admin Only)
- `POST /api/schools` - Create a new school
- `GET /api/schools` - List all schools
- `GET /api/schools/{id}` - Get school by ID
- `DELETE /api/schools/{id}` - Delete a school

### Users
- `POST /api/users` - Create user (admin/system_admin only)
- `GET /api/users` - List users (scoped by role)
- `GET /api/users/profile` - Get current user profile

### Authentication
- `POST /api/auth/login` - Login (no public registration)

### CLI Commands
- `cargo run -- create-sysadmin <first_name> <last_name> <email> <password>` - Create system admin
