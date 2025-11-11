# Authentication Implementation

This project implements JWT-based authentication for the Chalkbyte API.

## Features

- User registration with password hashing (bcrypt)
- User login with JWT token generation
- Protected routes using authentication middleware
- Token-based authorization

## API Endpoints

### Public Endpoints

#### Register User
```bash
POST /api/auth/register
Content-Type: application/json

{
  "first_name": "John",
  "last_name": "Doe",
  "email": "john@example.com",
  "password": "password123"
}
```

**Response (201 Created):**
```json
{
  "id": "227bcad5-0690-4ece-9c00-3d78e302616a",
  "first_name": "John",
  "last_name": "Doe",
  "email": "john@example.com"
}
```

#### Login User
```bash
POST /api/auth/login
Content-Type: application/json

{
  "email": "john@example.com",
  "password": "password123"
}
```

**Response (200 OK):**
```json
{
  "access_token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...",
  "user": {
    "id": "227bcad5-0690-4ece-9c00-3d78e302616a",
    "first_name": "John",
    "last_name": "Doe",
    "email": "john@example.com"
  }
}
```

### Protected Endpoints

These endpoints require a valid JWT token in the `Authorization` header.

#### Get User Profile
```bash
GET /api/users/profile
Authorization: Bearer <access_token>
```

**Response (200 OK):**
```json
{
  "user_id": "227bcad5-0690-4ece-9c00-3d78e302616a",
  "email": "john@example.com"
}
```

#### Get All Users
```bash
GET /api/users
Authorization: Bearer <access_token>
```

**Response (200 OK):**
```json
[
  {
    "id": "227bcad5-0690-4ece-9c00-3d78e302616a",
    "first_name": "John",
    "last_name": "Doe",
    "email": "john@example.com"
  }
]
```

## Configuration

Add these environment variables to your `.env` file:

```env
# JWT Configuration
JWT_SECRET=your-secret-key-change-in-production
JWT_ACCESS_EXPIRY=3600        # Token expiry in seconds (1 hour)
JWT_REFRESH_EXPIRY=604800     # Refresh token expiry (7 days - not yet implemented)
```

## Implementation Details

### Password Hashing
- Uses `bcrypt` with default cost factor (12)
- Passwords are hashed before storing in database
- Plain text passwords are never stored

### JWT Tokens
- Generated using `jsonwebtoken` crate
- Contains user ID, email, issued at (iat), and expiry (exp)
- Tokens expire after configured duration (default: 1 hour)

### Authentication Middleware
- Implemented as an Axum extractor (`AuthUser`)
- Extracts and validates JWT from `Authorization: Bearer <token>` header
- Returns `401 Unauthorized` for missing or invalid tokens
- Automatically injects user claims into protected route handlers

## Usage Example

### Protecting a Route

Add `AuthUser` parameter to any route handler to protect it:

```rust
use crate::middleware::auth::AuthUser;

#[instrument]
pub async fn protected_route(
    auth_user: AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    // Access user information from auth_user.0
    let user_id = &auth_user.0.sub;
    let email = &auth_user.0.email;
    
    // Your logic here
    Ok(Json(json!({
        "user_id": user_id,
        "email": email
    })))
}
```

### Testing with cURL

1. Register a new user:
```bash
curl -X POST http://localhost:3000/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "first_name": "John",
    "last_name": "Doe",
    "email": "john@example.com",
    "password": "password123"
  }'
```

2. Login and save the token:
```bash
TOKEN=$(curl -X POST http://localhost:3000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "john@example.com",
    "password": "password123"
  }' | jq -r '.access_token')
```

3. Access protected routes:
```bash
curl -X GET http://localhost:3000/api/users/profile \
  -H "Authorization: Bearer $TOKEN"
```

## Error Responses

### Missing Authorization Header
```json
{
  "error": "Missing authorization header"
}
```

### Invalid Token Format
```json
{
  "error": "Invalid authorization header format"
}
```

### Invalid or Expired Token
```json
{
  "error": "Invalid or expired token"
}
```

### Invalid Credentials
```json
{
  "error": "Invalid email or password"
}
```

### Email Already Exists
```json
{
  "error": "Email already exists"
}
```

## Security Considerations

1. **Always use HTTPS in production** to prevent token interception
2. **Change JWT_SECRET** to a strong, random value in production
3. **Token expiry**: Tokens expire after 1 hour by default
4. **Password requirements**: Minimum 8 characters (enforced by validation)
5. **Email validation**: Validates email format using `validator` crate

## Future Enhancements

- [ ] Refresh token implementation
- [ ] Token blacklist for logout
- [ ] Password reset functionality
- [ ] Two-factor authentication (2FA)
- [ ] Email verification
- [ ] Rate limiting for login attempts
- [ ] OAuth integration (Google, GitHub, etc.)
