# User Roles

This document describes the user role system implemented in the application.

## Available Roles

The system supports three user roles:

1. **Admin** - Administrator with full system access
2. **Teacher** - Teaching staff with elevated permissions
3. **Student** - Standard user with basic access (default role)

## Implementation Details

### Database Schema

- A PostgreSQL enum type `user_role` is created with values: `admin`, `teacher`, `student`
- The `users` table has a `role` column of type `user_role` with a default value of `student`
- An index is created on the `role` column for optimized queries

### API Usage

#### Registration

When registering a new user, you can optionally specify a role:

```json
{
  "first_name": "John",
  "last_name": "Doe",
  "email": "john@example.com",
  "password": "password123",
  "role": "teacher"
}
```

If no role is specified, the user will be assigned the default role of `student`.

#### JWT Token

User roles are included in JWT tokens as part of the claims. The `role` field in the JWT payload will contain one of: `admin`, `teacher`, or `student`.

#### User Response

All user objects returned by the API will include the `role` field:

```json
{
  "id": "uuid",
  "first_name": "John",
  "last_name": "Doe",
  "email": "john@example.com",
  "role": "teacher"
}
```

## Migration

The role system was added via migration `20251111195802_add_user_roles.sql`:
- Created the `user_role` enum type
- Added the `role` column to the `users` table
- All existing users are assigned the default role of `student`
- Created an index on the role column

## Future Enhancements

Consider implementing:
- Role-based authorization middleware
- Permission system tied to roles
- Role hierarchy and inheritance
- Dynamic role assignment endpoints (admin-only)
