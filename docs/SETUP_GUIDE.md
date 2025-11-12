# Complete Setup Guide

## Overview

This guide walks you through setting up the system from scratch, including creating the first system admin, schools, and users.

## Prerequisites

- PostgreSQL database running
- `.env` file configured with `DATABASE_URL`
- Rust and Cargo installed

## Step 1: Run Database Migrations

```bash
cargo sqlx migrate run
```

This will create all necessary tables including:
- `users` table with roles and school associations
- `schools` table with unique name constraint
- Indexes for performance

## Step 2: Create the First System Admin

Use the CLI command to create a system administrator:

```bash
cargo run -- create-sysadmin FirstName LastName email@domain.com SecurePassword123
```

Example:
```bash
cargo run -- create-sysadmin Super Admin admin@system.com MySecurePass123
```

Output:
```
✅ System admin created successfully!
   Email: admin@system.com
   Name: Super Admin
```

**Important**: Only system admins can be created via CLI. This is the only way to create the first admin.

## Step 3: Start the Server

```bash
cargo run
```

The server will start on `http://localhost:3000`

## Step 4: Login as System Admin

```bash
curl -X POST http://localhost:3000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "admin@system.com",
    "password": "MySecurePass123"
  }'
```

Save the `access_token` from the response. You'll use it for subsequent requests.

## Step 5: Create Schools

System admins can create schools:

```bash
curl -X POST http://localhost:3000/api/schools \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -d '{
    "name": "Washington High School",
    "address": "123 Main St, Washington DC"
  }'
```

**Note**: School names must be unique. Attempting to create a duplicate will fail.

Save the school `id` from the response.

## Step 6: Create School Admins

System admins create school administrators and assign them to schools:

```bash
curl -X POST http://localhost:3000/api/users \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_SYSTEM_ADMIN_TOKEN" \
  -d '{
    "first_name": "Principal",
    "last_name": "Smith",
    "email": "principal@washington.edu",
    "role": "admin",
    "school_id": "SCHOOL_ID_FROM_STEP_5"
  }'
```

**Important**: This creates the user without a password. You need to:
1. Manually set a password in the database, OR
2. Implement a password invitation/reset flow

### Setting Password Manually (Temporary Solution)

```sql
-- Hash a password first (use bcrypt)
UPDATE users 
SET password = '$2b$12$HashedPasswordHere' 
WHERE email = 'principal@washington.edu';
```

Or use this helper query with a plaintext password (for development only):
```bash
# Create a hashed password using a helper script
cargo run -- hash-password "TemporaryPassword123"
# Then update the database
```

## Step 7: School Admin Creates Teachers and Students

The school admin logs in and creates users for their school:

### Login as School Admin
```bash
curl -X POST http://localhost:3000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "principal@washington.edu",
    "password": "their-password"
  }'
```

### Create a Teacher
```bash
curl -X POST http://localhost:3000/api/users \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer SCHOOL_ADMIN_TOKEN" \
  -d '{
    "first_name": "Jane",
    "last_name": "Teacher",
    "email": "jteacher@washington.edu",
    "role": "teacher"
  }'
```

**Note**: The school admin doesn't need to specify `school_id` - it's automatically set to their school.

### Create a Student
```bash
curl -X POST http://localhost:3000/api/users \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer SCHOOL_ADMIN_TOKEN" \
  -d '{
    "first_name": "John",
    "last_name": "Student",
    "email": "jstudent@washington.edu",
    "role": "student"
  }'
```

## Verification

### List All Schools (System Admin)
```bash
curl -X GET http://localhost:3000/api/schools \
  -H "Authorization: Bearer SYSTEM_ADMIN_TOKEN"
```

### List Users (Scoped by Role)
```bash
curl -X GET http://localhost:3000/api/users \
  -H "Authorization: Bearer TOKEN"
```

- System admins see all users across all schools
- School admins see only users from their school

### Get School Details
```bash
curl -X GET http://localhost:3000/api/schools/SCHOOL_ID \
  -H "Authorization: Bearer TOKEN"
```

## Role Permissions Matrix

| Action | System Admin | School Admin | Teacher | Student |
|--------|-------------|--------------|---------|---------|
| Create system admin | CLI only | ❌ | ❌ | ❌ |
| Create schools | ✅ | ❌ | ❌ | ❌ |
| Delete schools | ✅ | ❌ | ❌ | ❌ |
| View all schools | ✅ | ❌ | ❌ | ❌ |
| Create users (any school) | ✅ | ❌ | ❌ | ❌ |
| Create users (own school) | ✅ | ✅ | ❌ | ❌ |
| View all users | ✅ | ❌ | ❌ | ❌ |
| View school users | ✅ | ✅ | ❌ | ❌ |

## Security Notes

1. **No Public Registration**: Users can only be created by admins
2. **Unique School Names**: Prevents duplicate schools
3. **School Isolation**: School admins are restricted to their school
4. **CLI-Only System Admin Creation**: Prevents unauthorized admin creation
5. **JWT Authentication**: All API endpoints require valid tokens

## Troubleshooting

### Cannot Create User - "Forbidden"
- Verify you're using the correct role (admin or system_admin)
- Check that the token is valid and not expired

### Cannot Create School Admin
- Ensure you're logged in as system admin
- Verify the school_id exists
- Check that the email is unique

### School Admin Cannot Create Users
- Verify the school admin has a `school_id` assigned
- Check that they're authenticated with the correct token

### Duplicate School Name Error
- School names must be unique
- Choose a different name or update the existing school

## Next Steps

After setup, consider implementing:

1. **Password Invitation Flow**: Send emails to new admins/teachers/students with password setup links
2. **User Management UI**: Build an admin dashboard
3. **Bulk User Import**: CSV import for students and teachers
4. **Audit Logging**: Track admin actions
5. **Password Reset**: Allow users to reset their passwords

## API Documentation

Interactive API documentation is available at:
- Swagger UI: http://localhost:3000/swagger-ui
- Scalar UI: http://localhost:3000/scalar

## Database Inspection

To view data directly:

```sql
-- View all schools
SELECT id, name, address FROM schools;

-- View all users with their roles and schools
SELECT 
  id, 
  first_name, 
  last_name, 
  email, 
  role, 
  school_id 
FROM users 
ORDER BY role, school_id;

-- View users by school
SELECT 
  u.first_name, 
  u.last_name, 
  u.email, 
  u.role,
  s.name as school_name
FROM users u
LEFT JOIN schools s ON u.school_id = s.id
WHERE u.school_id = 'YOUR_SCHOOL_ID';
```

## Support

For issues or questions:
1. Check the API documentation
2. Review error messages in server logs
3. Verify database state with SQL queries
4. Consult the implementation documents
