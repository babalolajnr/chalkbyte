# System Admin Implementation

## Overview

This document describes the implementation of a hierarchical role system with System Admin capabilities, school management, and role-based access control.

## Role Hierarchy

The system now supports four roles with hierarchical permissions:

### 1. System Admin
- **Highest privilege level**
- Can create and manage schools
- Can create users (admins, teachers, students) for any school
- Can view all users across all schools
- Not tied to any specific school (`school_id` is NULL)

### 2. Admin (School Admin)
- **School-scoped administrator**
- Tied to a specific school via `school_id`
- Can create teachers, students, and other admins **only** for their school
- Can only view and manage users within their school
- **Cannot** create system admins
- **Cannot** create users for other schools

### 3. Teacher
- Tied to a specific school via `school_id`
- Elevated permissions within their school
- Cannot create or manage users

### 4. Student
- Default role
- Tied to a specific school via `school_id`
- Standard user with basic access

## Database Schema Changes

### Migration: `20251112081331_add_system_admin_and_schools.sql`

1. **Extended `user_role` enum**:
   - Added `system_admin` value

2. **Created `schools` table**:
   ```sql
   CREATE TABLE schools (
       id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
       name VARCHAR NOT NULL,
       address TEXT,
       created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
       updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
   );
   ```

3. **Updated `users` table**:
   - Added `school_id UUID` column with foreign key reference to `schools(id)`
   - Added index on `school_id` for performance

### Migration: `20251112083030_add_unique_school_name.sql`

1. **Added unique constraint**:
   - School names must be unique across the system
   - Prevents duplicate school creation

## API Endpoints

### School Management (System Admin Only)

#### Create School
```http
POST /api/schools
Authorization: Bearer <system_admin_token>
Content-Type: application/json

{
  "name": "Springfield High School",
  "address": "123 Main St, Springfield"
}
```

#### List All Schools
```http
GET /api/schools
Authorization: Bearer <system_admin_token>
```

#### Get School by ID
```http
GET /api/schools/{id}
Authorization: Bearer <token>
```

#### Delete School
```http
DELETE /api/schools/{id}
Authorization: Bearer <system_admin_token>
```

### User Management

#### Create User (Admin/System Admin Only)
```http
POST /api/users
Authorization: Bearer <admin_token>
Content-Type: application/json

{
  "first_name": "John",
  "last_name": "Doe",
  "email": "john@example.com",
  "role": "teacher",
  "school_id": "uuid-of-school"
}
```

**Authorization Rules**:
- System admins can create users for any school
- School admins can only create users for their own school
- School admins cannot create system admins
- The `school_id` is automatically set to the admin's school for school admins

#### List Users
```http
GET /api/users
Authorization: Bearer <admin_token>
```

**Scope**:
- System admins see all users across all schools
- School admins only see users from their school

### Authentication

#### Creating System Admin (CLI Only)

System admins can only be created via the CLI command:

```bash
cargo run -- create-sysadmin FirstName LastName email@example.com password123
```

This ensures controlled access to system administrator creation.

#### Login
```http
POST /api/auth/login
Content-Type: application/json

{
  "email": "jane@example.com",
  "password": "password123"
}
```

Returns JWT token with role embedded in claims.

## Implementation Details

### Authorization Enforcement

Authorization is enforced in the controller layer:

1. **School Creation**: Only `system_admin` role allowed
2. **User Creation**: 
   - Only `system_admin` and `admin` roles allowed
   - School admins' `school_id` is automatically enforced
   - Validation ensures school exists before assignment
3. **User Listing**:
   - System admins get all users
   - School admins get filtered by their `school_id`

### JWT Token Claims

JWT tokens include the user's role:
```json
{
  "sub": "user-uuid",
  "email": "user@example.com",
  "role": "system_admin",
  "exp": 1234567890,
  "iat": 1234567890
}
```

Role values: `system_admin`, `admin`, `teacher`, `student`

### User Response Format

All user objects now include `school_id`:
```json
{
  "id": "uuid",
  "first_name": "John",
  "last_name": "Doe",
  "email": "john@example.com",
  "role": "teacher",
  "school_id": "school-uuid-or-null"
}
```

## Testing

A test script is provided: `test_system_admin.sh`

Run the script to test the complete workflow:
```bash
./test_system_admin.sh
```

The script tests:
1. System admin registration and login
2. School creation by system admin
3. School admin creation by system admin
4. Authorization checks (school admin cannot create schools)
5. Student registration
6. Authorization checks (students cannot create users)

## Security Features

1. **Role-based access control**: Enforced at the API level
2. **School isolation**: School admins can only access their school's data
3. **Hierarchical permissions**: Clear separation of privileges
4. **JWT authentication**: Secure token-based authentication
5. **Foreign key constraints**: Data integrity at the database level

## API Documentation

Interactive API documentation is available at:
- Swagger UI: http://localhost:3000/swagger-ui
- Scalar UI: http://localhost:3000/scalar

All new endpoints are documented with request/response schemas and authorization requirements.

## Future Enhancements

Potential improvements:
- Bulk user import for schools
- School admin dashboard with analytics
- Permission system with fine-grained access control
- Audit logging for admin actions
- Multi-tenancy support with school isolation
- Role assignment workflow with approval
