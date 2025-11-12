# System Admin Quick Start Guide

## Overview

This guide will help you quickly get started with the system admin features.

## Step 1: Start the Server

```bash
cargo run
```

Server will be available at `http://localhost:3000`

## Step 2: Create a System Admin

Create a system admin using the CLI command:

```bash
cargo run -- create-sysadmin System Admin sysadmin@example.com your-secure-password
```

This will create a system admin account directly in the database.

## Step 3: Login

```bash
curl -X POST http://localhost:3000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "sysadmin@example.com",
    "password": "your-secure-password"
  }'
```

Save the `access_token` from the response.

## Step 4: Create a School

```bash
curl -X POST http://localhost:3000/api/schools \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -d '{
    "name": "My High School",
    "address": "123 School Street"
  }'
```

Save the school `id` from the response.

## Step 5: Create a School Admin

```bash
curl -X POST http://localhost:3000/api/users \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -d '{
    "first_name": "School",
    "last_name": "Admin",
    "email": "admin@school.edu",
    "role": "admin",
    "school_id": "SCHOOL_ID_FROM_STEP_4"
  }'
```

**Note**: This endpoint creates a user without a password. You should implement a password invitation/reset flow, or use a direct database update to set the password temporarily.

## Step 6: Create Teachers and Students

As a school admin (after logging in):

```bash
# Create a teacher
curl -X POST http://localhost:3000/api/users \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer SCHOOL_ADMIN_TOKEN" \
  -d '{
    "first_name": "Jane",
    "last_name": "Teacher",
    "email": "teacher@school.edu",
    "role": "teacher",
    "school_id": "YOUR_SCHOOL_ID"
  }'

# Create a student
curl -X POST http://localhost:3000/api/users \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer SCHOOL_ADMIN_TOKEN" \
  -d '{
    "first_name": "John",
    "last_name": "Student",
    "email": "student@school.edu",
    "role": "student",
    "school_id": "YOUR_SCHOOL_ID"
  }'
```

## Common Operations

### List All Schools (System Admin)

```bash
curl -X GET http://localhost:3000/api/schools \
  -H "Authorization: Bearer YOUR_SYSTEM_ADMIN_TOKEN"
```

### List Users (Scoped by Role)

```bash
curl -X GET http://localhost:3000/api/users \
  -H "Authorization: Bearer YOUR_TOKEN"
```

- System admins see all users
- School admins see only their school's users

### Get a Specific School

```bash
curl -X GET http://localhost:3000/api/schools/SCHOOL_ID \
  -H "Authorization: Bearer YOUR_TOKEN"
```

### Delete a School (System Admin Only)

```bash
curl -X DELETE http://localhost:3000/api/schools/SCHOOL_ID \
  -H "Authorization: Bearer YOUR_SYSTEM_ADMIN_TOKEN"
```

## Role Permissions Summary

| Action | System Admin | School Admin | Teacher | Student |
|--------|-------------|--------------|---------|---------|
| Create Schools | ✅ | ❌ | ❌ | ❌ |
| View All Schools | ✅ | ❌ | ❌ | ❌ |
| Delete Schools | ✅ | ❌ | ❌ | ❌ |
| Create Users (Any School) | ✅ | ❌ | ❌ | ❌ |
| Create Users (Own School) | ✅ | ✅ | ❌ | ❌ |
| View All Users | ✅ | ❌ | ❌ | ❌ |
| View School Users | ✅ | ✅ | ❌ | ❌ |
| Create System Admins | ✅ | ❌ | ❌ | ❌ |

## API Documentation

Interactive API documentation is available at:

- **Swagger UI**: http://localhost:3000/swagger-ui
- **Scalar UI**: http://localhost:3000/scalar

## Testing

Run the provided test script:

```bash
./test_system_admin.sh
```

This will test all major functionality including authorization checks.

## Troubleshooting

### "Forbidden" Error

- Check that you're using the correct role's token
- Verify the user has the necessary permissions
- For school admins, ensure they are assigned to a school

### "School not found" Error

- Verify the school ID is correct
- Ensure the school exists in the database

### "Admin must be assigned to a school" Error

- School admins need a `school_id` to create users
- Update the admin's `school_id` in the database

## Database Check

To verify the data in your database:

```sql
-- View all schools
SELECT * FROM schools;

-- View all users with their schools
SELECT id, first_name, last_name, email, role, school_id FROM users;

-- View users by school
SELECT u.* FROM users u WHERE u.school_id = 'SCHOOL_ID';
```

## Next Steps

1. Implement password reset/invitation flow for admins created via API
2. Add bulk user import functionality
3. Implement audit logging for admin actions
4. Add school-specific settings and configuration
5. Create admin dashboard with analytics
