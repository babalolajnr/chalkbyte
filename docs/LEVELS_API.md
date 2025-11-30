# Levels API Documentation

## Overview

The Levels API allows school administrators to create and manage grade levels/classes for their schools. Levels help organize students into groups such as "Grade 1", "Grade 2", "Form A", etc.

## Authorization

All endpoints require:
- Valid JWT Bearer token
- User role: `admin` (School Admin)
- Admin must be associated with a school

## Endpoints

### 1. Create Level

Create a new level for the admin's school.

**Endpoint:** `POST /api/levels`

**Request Body:**
```json
{
  "name": "Grade 5",
  "description": "Fifth grade students"
}
```

**Response:** `201 Created`
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "name": "Grade 5",
  "description": "Fifth grade students",
  "school_id": "123e4567-e89b-12d3-a456-426614174000",
  "created_at": "2024-11-30T14:30:00Z",
  "updated_at": "2024-11-30T14:30:00Z"
}
```

**Constraints:**
- Level name must be unique within the school
- Name length: 1-100 characters

---

### 2. List Levels

Get paginated list of levels with student counts.

**Endpoint:** `GET /api/levels`

**Query Parameters:**
- `name` (optional): Filter by level name (partial match, case-insensitive)
- `limit` (optional): Number of items per page (default: 10, max: 100)
- `offset` (optional): Number of items to skip (default: 0)

**Example:** `GET /api/levels?name=grade&limit=20&offset=0`

**Response:** `200 OK`
```json
{
  "data": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "name": "Grade 5",
      "description": "Fifth grade students",
      "school_id": "123e4567-e89b-12d3-a456-426614174000",
      "student_count": 25,
      "created_at": "2024-11-30T14:30:00Z",
      "updated_at": "2024-11-30T14:30:00Z"
    }
  ],
  "meta": {
    "total": 1,
    "limit": 20,
    "offset": 0,
    "page": null,
    "has_more": false
  }
}
```

---

### 3. Get Level Details

Get details of a specific level with student count.

**Endpoint:** `GET /api/levels/{id}`

**Response:** `200 OK`
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "name": "Grade 5",
  "description": "Fifth grade students",
  "school_id": "123e4567-e89b-12d3-a456-426614174000",
  "student_count": 25,
  "created_at": "2024-11-30T14:30:00Z",
  "updated_at": "2024-11-30T14:30:00Z"
}
```

---

### 4. Update Level

Update level name or description.

**Endpoint:** `PUT /api/levels/{id}`

**Request Body:**
```json
{
  "name": "Grade 5A",
  "description": "Fifth grade, section A"
}
```

**Response:** `200 OK`
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "name": "Grade 5A",
  "description": "Fifth grade, section A",
  "school_id": "123e4567-e89b-12d3-a456-426614174000",
  "created_at": "2024-11-30T14:30:00Z",
  "updated_at": "2024-11-30T15:45:00Z"
}
```

**Notes:**
- Both fields are optional
- Only provided fields will be updated

---

### 5. Delete Level

Delete a level. Students in the level will have their `level_id` set to NULL.

**Endpoint:** `DELETE /api/levels/{id}`

**Response:** `204 No Content`

---

### 6. Assign Students to Level

Bulk assign multiple students to a level.

**Endpoint:** `POST /api/levels/{id}/students`

**Request Body:**
```json
{
  "student_ids": [
    "a1b2c3d4-e5f6-4a5b-8c7d-9e0f1a2b3c4d",
    "b2c3d4e5-f6a7-5b6c-9d8e-0f1a2b3c4d5e"
  ]
}
```

**Response:** `200 OK`
```json
{
  "assigned_count": 2,
  "failed_ids": []
}
```

**Notes:**
- Only students from the same school can be assigned
- Students must have role `student`
- Failed IDs are students that don't exist or don't belong to the school
- Successful assignments are still processed even if some fail

---

### 7. Get Students in Level

List all students assigned to a specific level.

**Endpoint:** `GET /api/levels/{id}/students`

**Response:** `200 OK`
```json
[
  {
    "id": "a1b2c3d4-e5f6-4a5b-8c7d-9e0f1a2b3c4d",
    "first_name": "John",
    "last_name": "Doe",
    "email": "john.doe@example.com",
    "role": "student",
    "school_id": "123e4567-e89b-12d3-a456-426614174000"
  }
]
```

**Notes:**
- Students are sorted by last_name, then first_name

---

### 8. Move Student to Different Level

Move a single student to a different level or remove from all levels.

**Endpoint:** `PATCH /api/levels/students/{student_id}/move`

**Request Body (Move to level):**
```json
{
  "level_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

**Request Body (Remove from all levels):**
```json
{
  "level_id": null
}
```

**Response:** `204 No Content`

**Notes:**
- Student must belong to the admin's school
- Target level must exist and belong to the admin's school

---

### 9. Remove Student from Level

Remove a student from their current level.

**Endpoint:** `DELETE /api/levels/students/{student_id}`

**Response:** `204 No Content`

**Notes:**
- Equivalent to moving student with `level_id: null`
- Student's `level_id` field is set to NULL

---

## Error Responses

### 400 Bad Request
```json
{
  "message": "A level with this name already exists in this school"
}
```

### 401 Unauthorized
```json
{
  "message": "Invalid user ID"
}
```

### 403 Forbidden
```json
{
  "message": "Only school admins can create levels"
}
```

### 404 Not Found
```json
{
  "message": "Level not found"
}
```

---

## Usage Examples

### Creating and Managing Levels

```bash
# 1. Login as school admin
TOKEN=$(curl -X POST http://localhost:3000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"admin@school.com","password":"password123"}' \
  | jq -r '.access_token')

# 2. Create levels
curl -X POST http://localhost:3000/api/levels \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name":"Grade 1","description":"First grade students"}'

curl -X POST http://localhost:3000/api/levels \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name":"Grade 2","description":"Second grade students"}'

# 3. List all levels
curl -X GET http://localhost:3000/api/levels \
  -H "Authorization: Bearer $TOKEN"

# 4. Get specific level
LEVEL_ID="550e8400-e29b-41d4-a716-446655440000"
curl -X GET http://localhost:3000/api/levels/$LEVEL_ID \
  -H "Authorization: Bearer $TOKEN"
```

### Managing Students in Levels

```bash
# 1. Assign multiple students to a level
curl -X POST http://localhost:3000/api/levels/$LEVEL_ID/students \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "student_ids": [
      "a1b2c3d4-e5f6-4a5b-8c7d-9e0f1a2b3c4d",
      "b2c3d4e5-f6a7-5b6c-9d8e-0f1a2b3c4d5e"
    ]
  }'

# 2. Get all students in the level
curl -X GET http://localhost:3000/api/levels/$LEVEL_ID/students \
  -H "Authorization: Bearer $TOKEN"

# 3. Move a student to different level
STUDENT_ID="a1b2c3d4-e5f6-4a5b-8c7d-9e0f1a2b3c4d"
NEW_LEVEL_ID="660e8400-e29b-41d4-a716-446655440001"
curl -X PATCH http://localhost:3000/api/levels/students/$STUDENT_ID/move \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d "{\"level_id\":\"$NEW_LEVEL_ID\"}"

# 4. Remove student from level
curl -X DELETE http://localhost:3000/api/levels/students/$STUDENT_ID \
  -H "Authorization: Bearer $TOKEN"
```

---

## Business Rules

1. **School Isolation**: Admins can only manage levels for their own school
2. **Unique Names**: Level names must be unique within each school
3. **Cascading Deletes**: Deleting a level sets students' `level_id` to NULL (soft dependency)
4. **Student Assignment**: Only students with role `student` can be assigned to levels
5. **School Matching**: Students can only be assigned to levels in their school
6. **Bulk Operations**: Failed assignments in bulk operations don't prevent successful ones

---

## Database Schema

```sql
CREATE TABLE levels (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(100) NOT NULL,
    description TEXT,
    school_id UUID NOT NULL REFERENCES schools(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT unique_level_name_per_school UNIQUE (name, school_id)
);

ALTER TABLE users ADD COLUMN level_id UUID REFERENCES levels(id) ON DELETE SET NULL;
```

---

## Common Use Cases

### Organizing Students by Grade
```bash
# Create grade levels
for grade in {1..6}; do
  curl -X POST http://localhost:3000/api/levels \
    -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d "{\"name\":\"Grade $grade\",\"description\":\"Grade $grade students\"}"
done
```

### Promoting Students to Next Level
```bash
# Get all students in Grade 5
GRADE_5_STUDENTS=$(curl -s -X GET http://localhost:3000/api/levels/$GRADE_5_ID/students \
  -H "Authorization: Bearer $TOKEN" | jq -r '.[].id')

# Move them to Grade 6
for student_id in $GRADE_5_STUDENTS; do
  curl -X PATCH http://localhost:3000/api/levels/students/$student_id/move \
    -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d "{\"level_id\":\"$GRADE_6_ID\"}"
done
```

### Finding Unassigned Students
```bash
# Get all students without a level
curl -X GET "http://localhost:3000/api/students?level_id=null" \
  -H "Authorization: Bearer $TOKEN"
```
