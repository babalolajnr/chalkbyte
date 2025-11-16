# Students Management API

## Overview

School admins can now manage students belonging to their schools through a complete CRUD API. All operations are automatically scoped to the admin's school.

## Endpoints

### Create Student
```
POST /api/students
Authorization: Bearer <school_admin_token>
Content-Type: application/json

{
  "first_name": "John",
  "last_name": "Doe",
  "email": "john.doe@example.com",
  "password": "securepass123"
}
```

**Response**: Student object with generated ID

**Authorization**: School admins only. Student automatically assigned to admin's school.

---

### List Students
```
GET /api/students
Authorization: Bearer <school_admin_token>
```

**Response**: Array of students belonging to the admin's school, ordered by last name, first name.

**Authorization**: School admins only. Returns only students from their school.

---

### Get Student by ID
```
GET /api/students/{id}
Authorization: Bearer <school_admin_token>
```

**Response**: Student details if found in admin's school.

**Authorization**: School admins only. Returns 404 if student not in their school.

---

### Update Student
```
PUT /api/students/{id}
Authorization: Bearer <school_admin_token>
Content-Type: application/json

{
  "first_name": "Jane",
  "last_name": "Smith",
  "email": "jane.smith@example.com",
  "password": "newpassword123"  // optional
}
```

**Response**: Updated student object

**Authorization**: School admins only. Can only update students in their school.

**Note**: All fields are optional. Password will be hashed if provided.

---

### Delete Student
```
DELETE /api/students/{id}
Authorization: Bearer <school_admin_token>
```

**Response**: 204 No Content on success

**Authorization**: School admins only. Can only delete students in their school.

---

## Validation Rules

- `first_name`: 1-100 characters, required
- `last_name`: 1-100 characters, required
- `email`: Valid email format, unique across all users, required
- `password`: Minimum 8 characters, required on create, optional on update

## Security Features

1. **Automatic School Scoping**: Students are automatically assigned to the admin's school on creation
2. **School Isolation**: Admins can only see/manage students from their own school
3. **Password Hashing**: All passwords are hashed with bcrypt before storage
4. **Authorization Checks**: All endpoints verify admin role and school ownership
5. **Unique Email**: Email uniqueness enforced at database level

## Database Schema

Students are stored in the `users` table with:
- `role = 'student'`
- `school_id` matching the creating admin's school
- `password` stored as bcrypt hash
- `created_at` and `updated_at` timestamps

## Example Workflow

1. School admin logs in to get JWT token
2. Admin creates students for their school
3. Admin can list all their school's students
4. Admin can update student details (including password reset)
5. Admin can delete students if needed

## Error Responses

- **400 Bad Request**: Validation failed or duplicate email
- **401 Unauthorized**: Missing or invalid token
- **403 Forbidden**: Not a school admin or wrong school
- **404 Not Found**: Student not found in admin's school
- **500 Internal Server Error**: Database or server error