# Roles & Permissions API Documentation

Frontend integration guide for the Chalkbyte Roles & Permissions system.

## Base URL

```
/api
```

## Authentication

All endpoints require a valid JWT token in the Authorization header:

```
Authorization: Bearer <access_token>
```

---

## Data Models

### Permission

```typescript
interface Permission {
  id: string;           // UUID
  name: string;
  description: string | null;
  category: string;
  created_at: string;   // ISO 8601 datetime
  updated_at: string;
}
```

### CustomRole (with permissions)

```typescript
interface CustomRoleWithPermissions {
  id: string;           // UUID
  name: string;
  description: string | null;
  school_id: string | null;  // null = system role
  is_system_role: boolean;
  created_at: string;
  updated_at: string;
  permissions: Permission[];
}
```

### Pagination

```typescript
interface PaginationMeta {
  total: number;
  limit: number;
  offset?: number;
  page?: number;
  has_more: boolean;
}

// Query parameters (all optional)
interface PaginationParams {
  limit?: number;   // 1-100, default: 10
  offset?: number;  // default: 0
  page?: number;    // alternative to offset
}
```

---

## Login Response

The login endpoint (`POST /api/auth/login`) now includes the user's custom roles and permissions in the response:

```typescript
interface LoginResponse {
  access_token: string;
  refresh_token: string;
  user: User;
  roles: CustomRoleWithPermissions[];
  permissions: Permission[];  // Deduplicated list from all roles
}
```

**Example Response:**

```json
{
  "access_token": "eyJ...",
  "refresh_token": "eyJ...",
  "user": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "first_name": "John",
    "last_name": "Doe",
    "email": "john@school.com",
    "role": "admin",
    "school_id": "550e8400-e29b-41d4-a716-446655440001"
  },
  "roles": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440002",
      "name": "Grade Coordinator",
      "description": "Manages grade-level activities",
      "school_id": "550e8400-e29b-41d4-a716-446655440001",
      "is_system_role": false,
      "created_at": "2024-01-15T10:30:00Z",
      "updated_at": "2024-01-15T10:30:00Z",
      "permissions": [
        {
          "id": "550e8400-e29b-41d4-a716-446655440003",
          "name": "grades.view",
          "description": "View student grades",
          "category": "grades",
          "created_at": "2024-01-01T00:00:00Z",
          "updated_at": "2024-01-01T00:00:00Z"
        }
      ]
    }
  ],
  "permissions": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440003",
      "name": "grades.view",
      "description": "View student grades",
      "category": "grades",
      "created_at": "2024-01-01T00:00:00Z",
      "updated_at": "2024-01-01T00:00:00Z"
    }
  ]
}
```

This applies to all login methods:
- `POST /api/auth/login` - Standard login
- `POST /api/auth/mfa/verify` - MFA verification
- `POST /api/auth/mfa/recovery` - MFA recovery code login
- `POST /api/auth/refresh` - Token refresh

---

## Endpoints

### Permissions

#### List Permissions

```
GET /api/roles/permissions
```

**Query Parameters:**

| Parameter | Type   | Description              |
|-----------|--------|--------------------------|
| category  | string | Filter by category       |
| limit     | number | Items per page (1-100)   |
| offset    | number | Offset for pagination    |
| page      | number | Page number              |

**Response:** `200 OK`

```typescript
{
  data: Permission[];
  meta: PaginationMeta;
}
```

**Example:**

```javascript
const response = await fetch('/api/roles/permissions?category=users&limit=20', {
  headers: { 'Authorization': `Bearer ${token}` }
});
const { data, meta } = await response.json();
```

---

#### Get Permission by ID

```
GET /api/roles/permissions/{id}
```

**Path Parameters:**

| Parameter | Type | Description    |
|-----------|------|----------------|
| id        | UUID | Permission ID  |

**Response:** `200 OK`

```typescript
Permission
```

**Error Responses:**
- `404 Not Found` - Permission not found

---

### Roles

#### Create Role

```
POST /api/roles
```

**Request Body:**

```typescript
{
  name: string;                    // 1-100 characters, required
  description?: string;            // max 500 characters
  school_id?: string;              // UUID, omit for system role
  permission_ids?: string[];       // UUIDs of permissions to assign
}
```

**Authorization Rules:**
- System admins can create system roles (omit `school_id`) or school-scoped roles
- School admins can only create roles for their own school

**Response:** `201 Created`

```typescript
CustomRoleWithPermissions
```

**Error Responses:**
- `400 Bad Request` - Validation error or duplicate name
- `403 Forbidden` - Insufficient permissions

**Example:**

```javascript
const response = await fetch('/api/roles', {
  method: 'POST',
  headers: {
    'Authorization': `Bearer ${token}`,
    'Content-Type': 'application/json'
  },
  body: JSON.stringify({
    name: 'Grade Coordinator',
    description: 'Manages grade-level activities',
    school_id: 'abc123-...',
    permission_ids: ['perm-id-1', 'perm-id-2']
  })
});
```

---

#### List Roles

```
GET /api/roles
```

**Query Parameters:**

| Parameter      | Type    | Description                    |
|----------------|---------|--------------------------------|
| school_id      | UUID    | Filter by school               |
| is_system_role | boolean | Filter system roles only       |
| name           | string  | Search by name                 |
| limit          | number  | Items per page                 |
| offset/page    | number  | Pagination                     |

**Authorization Rules:**
- System admins see all roles
- School admins see their school's roles + system roles

**Response:** `200 OK`

```typescript
{
  data: CustomRoleWithPermissions[];
  meta: PaginationMeta;
}
```

---

#### Get Role by ID

```
GET /api/roles/{id}
```

**Path Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| id        | UUID | Role ID     |

**Response:** `200 OK`

```typescript
CustomRoleWithPermissions
```

**Error Responses:**
- `403 Forbidden` - Role belongs to another school
- `404 Not Found` - Role not found

---

#### Update Role

```
PUT /api/roles/{id}
```

**Path Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| id        | UUID | Role ID     |

**Request Body:**

```typescript
{
  name?: string;         // 1-100 characters
  description?: string;  // max 500 characters
}
```

**Response:** `200 OK`

```typescript
CustomRoleWithPermissions
```

**Error Responses:**
- `400 Bad Request` - Validation error
- `403 Forbidden` - Cannot modify role from another school
- `404 Not Found` - Role not found

---

#### Delete Role

```
DELETE /api/roles/{id}
```

**Path Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| id        | UUID | Role ID     |

**Response:** `200 OK` (empty body)

**Error Responses:**
- `403 Forbidden` - Cannot delete role from another school
- `404 Not Found` - Role not found

---

### Role Permission Management

#### Assign Permissions to Role

```
POST /api/roles/{id}/permissions
```

**Path Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| id        | UUID | Role ID     |

**Request Body:**

```typescript
{
  permission_ids: string[];  // Array of permission UUIDs
}
```

**Response:** `200 OK`

```typescript
CustomRoleWithPermissions  // Updated role with all permissions
```

**Example:**

```javascript
await fetch(`/api/roles/${roleId}/permissions`, {
  method: 'POST',
  headers: {
    'Authorization': `Bearer ${token}`,
    'Content-Type': 'application/json'
  },
  body: JSON.stringify({
    permission_ids: ['perm-1', 'perm-2', 'perm-3']
  })
});
```

---

#### Remove Permission from Role

```
DELETE /api/roles/{role_id}/permissions/{permission_id}
```

**Path Parameters:**

| Parameter     | Type | Description   |
|---------------|------|---------------|
| role_id       | UUID | Role ID       |
| permission_id | UUID | Permission ID |

**Response:** `200 OK`

```typescript
CustomRoleWithPermissions  // Updated role
```

---

### User Role Assignment

#### Get User's Roles

```
GET /api/users/{user_id}/roles
```

**Path Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| user_id   | UUID | User ID     |

**Response:** `200 OK`

```typescript
CustomRoleWithPermissions[]
```

---

#### Assign Role to User

```
POST /api/users/{user_id}/roles
```

**Path Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| user_id   | UUID | User ID     |

**Request Body:**

```typescript
{
  role_id: string;  // UUID of role to assign
}
```

**Response:** `200 OK`

```typescript
{
  message: string;
  user_id: string;
  role_id: string;
}
```

**Error Responses:**
- `400 Bad Request` - Role already assigned to user
- `403 Forbidden` - Cannot assign role to user in another school
- `404 Not Found` - User or role not found

---

#### Remove Role from User

```
DELETE /api/users/{user_id}/roles/{role_id}
```

**Path Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| user_id   | UUID | User ID     |
| role_id   | UUID | Role ID     |

**Response:** `200 OK` (empty body)

---

#### Get User's Effective Permissions

```
GET /api/users/{user_id}/permissions
```

Returns all unique permissions from all roles assigned to the user.

**Path Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| user_id   | UUID | User ID     |

**Authorization Rules:**
- Users can view their own permissions
- Admins can view permissions of users in their school
- System admins can view any user's permissions

**Response:** `200 OK`

```typescript
Permission[]  // Deduplicated list
```

---

## Frontend Integration Examples

### React Hook Example

```typescript
// useRoles.ts
import { useState, useEffect } from 'react';

interface UseRolesOptions {
  schoolId?: string;
  isSystemRole?: boolean;
}

export function useRoles(options: UseRolesOptions = {}) {
  const [roles, setRoles] = useState<CustomRoleWithPermissions[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);

  useEffect(() => {
    const params = new URLSearchParams();
    if (options.schoolId) params.set('school_id', options.schoolId);
    if (options.isSystemRole !== undefined) {
      params.set('is_system_role', String(options.isSystemRole));
    }

    fetch(`/api/roles?${params}`, {
      headers: { 'Authorization': `Bearer ${getToken()}` }
    })
      .then(res => res.json())
      .then(data => {
        setRoles(data.data);
        setLoading(false);
      })
      .catch(err => {
        setError(err);
        setLoading(false);
      });
  }, [options.schoolId, options.isSystemRole]);

  return { roles, loading, error };
}
```

### Permission Check Utility

```typescript
// permissions.ts
export function hasPermission(
  userPermissions: Permission[],
  requiredPermission: string
): boolean {
  return userPermissions.some(p => p.name === requiredPermission);
}

export function hasAnyPermission(
  userPermissions: Permission[],
  required: string[]
): boolean {
  return required.some(r => hasPermission(userPermissions, r));
}

export function hasAllPermissions(
  userPermissions: Permission[],
  required: string[]
): boolean {
  return required.every(r => hasPermission(userPermissions, r));
}

// Usage in component
const { permissions } = useUserPermissions(userId);

if (hasPermission(permissions, 'users.create')) {
  // Show create user button
}
```

### Using Login Response for Permissions

```typescript
// authStore.ts (e.g., Zustand, Redux, or Context)
interface AuthState {
  user: User | null;
  customRoles: CustomRoleWithPermissions[];
  permissions: Permission[];
  setAuth: (response: LoginResponse) => void;
}

// On login success
const login = async (email: string, password: string) => {
  const response = await fetch('/api/auth/login', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ email, password })
  });
  
  const data: LoginResponse = await response.json();
  
  // Store tokens
  localStorage.setItem('access_token', data.access_token);
  localStorage.setItem('refresh_token', data.refresh_token);
  
  // Store user, roles, and permissions in state
  authStore.setAuth(data);
};

// Permission check using stored permissions
const canCreateUser = hasPermission(authStore.permissions, 'users.create');
```

### Role Assignment Component

```typescript
// AssignRoleModal.tsx
async function assignRole(userId: string, roleId: string) {
  const response = await fetch(`/api/users/${userId}/roles`, {
    method: 'POST',
    headers: {
      'Authorization': `Bearer ${token}`,
      'Content-Type': 'application/json'
    },
    body: JSON.stringify({ role_id: roleId })
  });

  if (!response.ok) {
    const error = await response.json();
    throw new Error(error.message);
  }

  return response.json();
}
```

---

## Error Response Format

All error responses follow this structure:

```typescript
{
  error: string;
  message: string;
}
```

Common HTTP status codes:
- `400` - Bad Request (validation errors)
- `401` - Unauthorized (missing/invalid token)
- `403` - Forbidden (insufficient permissions)
- `404` - Not Found
- `500` - Internal Server Error

---

## Permission Categories

Permissions are organized by category. Common categories include:

| Category | Description                    |
|----------|--------------------------------|
| users    | User management permissions    |
| schools  | School management permissions  |
| roles    | Role management permissions    |
| reports  | Reporting permissions          |

Use the `category` filter on `GET /api/roles/permissions` to fetch permissions by category for organized UI display.