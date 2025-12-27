# Levels API Documentation

Base URL: `/api/levels`

**Authorization**: All endpoints require `Bearer` token authentication. Only users with `admin` role (school admins) can access these endpoints.

---

## Endpoints

### 1. Create Level

**POST** `/api/levels`

Creates a new level within the admin's school.

**Request Body:**
```json
{
  "name": "string (required, 1-100 chars)",
  "description": "string (optional)"
}
```

**Response:** `201 Created`
```json
{
  "id": "uuid",
  "name": "string",
  "description": "string | null",
  "school_id": "uuid",
  "created_at": "ISO 8601 datetime",
  "updated_at": "ISO 8601 datetime"
}
```

**Errors:** `400`, `401`, `403`

---

### 2. List Levels

**GET** `/api/levels`

Returns paginated list of levels for the admin's school with student counts.

**Query Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| `name` | string | Filter by level name (partial match) |
| `page` | integer | Page number (default: 1) |
| `per_page` | integer | Items per page |

**Response:** `200 OK`
```json
{
  "data": [
    {
      "id": "uuid",
      "name": "string",
      "description": "string | null",
      "school_id": "uuid",
      "student_count": 0,
      "created_at": "ISO 8601 datetime",
      "updated_at": "ISO 8601 datetime"
    }
  ],
  "meta": {
    "current_page": 1,
    "per_page": 10,
    "total_items": 100,
    "total_pages": 10
  }
}
```

**Errors:** `401`, `403`

---

### 3. Get Level by ID

**GET** `/api/levels/{id}`

**Path Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| `id` | uuid | Level ID |

**Response:** `200 OK`
```json
{
  "id": "uuid",
  "name": "string",
  "description": "string | null",
  "school_id": "uuid",
  "student_count": 0,
  "created_at": "ISO 8601 datetime",
  "updated_at": "ISO 8601 datetime"
}
```

**Errors:** `401`, `403`, `404`

---

### 4. Update Level

**PUT** `/api/levels/{id}`

**Path Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| `id` | uuid | Level ID |

**Request Body:**
```json
{
  "name": "string (optional, 1-100 chars)",
  "description": "string (optional)"
}
```

**Response:** `200 OK`
```json
{
  "id": "uuid",
  "name": "string",
  "description": "string | null",
  "school_id": "uuid",
  "created_at": "ISO 8601 datetime",
  "updated_at": "ISO 8601 datetime"
}
```

**Errors:** `400`, `401`, `403`, `404`

---

### 5. Delete Level

**DELETE** `/api/levels/{id}`

**Path Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| `id` | uuid | Level ID |

**Response:** `204 No Content`

**Errors:** `401`, `403`, `404`

---

### 6. Assign Students to Level

**POST** `/api/levels/{id}/students`

Bulk assign students to a level.

**Path Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| `id` | uuid | Level ID |

**Request Body:**
```json
{
  "student_ids": ["uuid", "uuid"]
}
```
*Note: `student_ids` must contain at least 1 ID.*

**Response:** `200 OK`
```json
{
  "assigned_count": 5,
  "failed_ids": ["uuid"]
}
```

**Errors:** `400`, `401`, `403`, `404`

---

### 7. Get Students in Level

**GET** `/api/levels/{id}/students`

**Path Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| `id` | uuid | Level ID |

**Response:** `200 OK`
```json
[
  {
    "id": "uuid",
    "first_name": "string",
    "last_name": "string",
    "email": "string",
    "role": "student",
    "school_id": "uuid",
    "created_at": "ISO 8601 datetime",
    "updated_at": "ISO 8601 datetime"
  }
]
```

**Errors:** `401`, `403`, `404`

---

### 8. Move Student to Level

**PATCH** `/api/levels/students/{student_id}/move`

Move a student to a different level or remove from current level.

**Path Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| `student_id` | uuid | Student ID |

**Request Body:**
```json
{
  "level_id": "uuid | null"
}
```
*Note: Set `level_id` to `null` to remove student from their current level.*

**Response:** `204 No Content`

**Errors:** `400`, `401`, `403`, `404`

---

### 9. Remove Student from Level

**DELETE** `/api/levels/students/{student_id}`

Removes a student from their assigned level.

**Path Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| `student_id` | uuid | Student ID |

**Response:** `204 No Content`

**Errors:** `401`, `403`, `404`

---

## Error Responses

| Status | Description |
|--------|-------------|
| `400` | Bad Request - Invalid input or validation failed |
| `401` | Unauthorized - Missing or invalid token |
| `403` | Forbidden - User lacks permission (not an admin) |
| `404` | Not Found - Resource doesn't exist or belongs to another school |

**Error Response Format:**
```json
{
  "error": "Error message description"
}
```

---

## Example Usage

### Create a Level
```bash
curl -X POST http://localhost:3000/api/levels \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{"name": "Grade 10", "description": "Tenth grade students"}'
```

### List Levels with Filter
```bash
curl "http://localhost:3000/api/levels?name=Grade&page=1&per_page=10" \
  -H "Authorization: Bearer <token>"
```

### Assign Students to Level
```bash
curl -X POST http://localhost:3000/api/levels/{level_id}/students \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{"student_ids": ["uuid1", "uuid2", "uuid3"]}'
```

### Move Student to Another Level
```bash
curl -X PATCH http://localhost:3000/api/levels/students/{student_id}/move \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{"level_id": "new-level-uuid"}'
```
