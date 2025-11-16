# Email Configuration

This document describes the email configuration for password reset functionality in Chalkbyte.

## Environment Variables

Add the following environment variables to your `.env` file:

```env
# Email Configuration
SMTP_HOST=localhost
SMTP_PORT=1025
SMTP_USERNAME=
SMTP_PASSWORD=
FROM_EMAIL=noreply@chalkbyte.com
FROM_NAME=Chalkbyte
FRONTEND_URL=http://localhost:3000
```

## Development Setup with Mailpit

Mailpit is included in the `docker-compose.yml` for local email testing.

### Starting Mailpit

```bash
docker compose up mailpit -d
```

### Accessing Mailpit

- **Web UI**: http://localhost:8025
- **SMTP Server**: localhost:1025

The Web UI allows you to view all emails sent by the application during development.

## Password Reset Flow

### 1. Request Password Reset

**Endpoint**: `POST /api/auth/forgot-password`

**Request Body**:
```json
{
  "email": "user@example.com"
}
```

**Response**: Always returns 200 to prevent email enumeration
```json
{
  "message": "If an account exists with that email, a password reset link has been sent."
}
```

### 2. User Receives Email

The user receives an email containing:
- A reset link with token: `{FRONTEND_URL}/reset-password?token={TOKEN}`
- Token expiry information (1 hour)
- Security notice

### 3. Reset Password

**Endpoint**: `POST /api/auth/reset-password`

**Request Body**:
```json
{
  "token": "uuid-token-from-email",
  "new_password": "newSecurePassword123"
}
```

**Response**:
```json
{
  "message": "Password has been reset successfully. You can now log in with your new password."
}
```

### 4. Confirmation Email

After successful password reset, the user receives a confirmation email.

## Email Templates

Two email templates are included:

### 1. Password Reset Request
- Professional HTML layout with responsive design
- Clear call-to-action button
- Plain text fallback for email clients without HTML support
- Security notice about ignoring email if not requested

### 2. Password Reset Confirmation
- Confirmation of successful password reset
- Security warning to contact support if change was not made
- Professional styling consistent with brand

## Database Schema

Password reset tokens are stored in the `password_reset_tokens` table:

```sql
CREATE TABLE password_reset_tokens (
    id UUID PRIMARY KEY,
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    token TEXT UNIQUE NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    used BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ DEFAULT NOW()
);
```

### Token Management

- **Expiry**: Tokens expire after 1 hour
- **One-time Use**: Tokens are marked as used after successful reset
- **Auto-cleanup**: Old unused tokens for a user are deleted when requesting a new reset
- **Security**: Tokens are UUIDs for cryptographic randomness

## Testing

### Manual Testing with curl

```bash
# 1. Request password reset
curl -X POST http://localhost:3000/api/auth/forgot-password \
  -H "Content-Type: application/json" \
  -d '{"email":"test@example.com"}'

# 2. Check Mailpit UI at http://localhost:8025 for the email
# Copy the token from the email

# 3. Reset password
curl -X POST http://localhost:3000/api/auth/reset-password \
  -H "Content-Type: application/json" \
  -d '{"token":"YOUR_TOKEN_HERE","new_password":"newPassword123"}'

# 4. Login with new password
curl -X POST http://localhost:3000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"test@example.com","password":"newPassword123"}'
```

## Production Configuration

### Using Gmail SMTP

```env
SMTP_HOST=smtp.gmail.com
SMTP_PORT=587
SMTP_USERNAME=your-email@gmail.com
SMTP_PASSWORD=your-app-password
FROM_EMAIL=noreply@yourdomain.com
FROM_NAME=Your App Name
FRONTEND_URL=https://yourdomain.com
```

### Using SendGrid

```env
SMTP_HOST=smtp.sendgrid.net
SMTP_PORT=587
SMTP_USERNAME=apikey
SMTP_PASSWORD=your-sendgrid-api-key
FROM_EMAIL=noreply@yourdomain.com
FROM_NAME=Your App Name
FRONTEND_URL=https://yourdomain.com
```

### Using AWS SES

```env
SMTP_HOST=email-smtp.us-east-1.amazonaws.com
SMTP_PORT=587
SMTP_USERNAME=your-ses-smtp-username
SMTP_PASSWORD=your-ses-smtp-password
FROM_EMAIL=noreply@yourdomain.com
FROM_NAME=Your App Name
FRONTEND_URL=https://yourdomain.com
```

## Security Considerations

1. **Email Enumeration Prevention**: Always return the same response regardless of whether the email exists
2. **Token Expiry**: Tokens expire after 1 hour
3. **One-time Use**: Tokens cannot be reused
4. **Secure Storage**: Tokens are stored as plain UUIDs (not passwords)
5. **Password Requirements**: Minimum 8 characters enforced via validation
6. **Confirmation Emails**: Users are notified of successful password changes
7. **HTTPS**: Always use HTTPS in production for the `FRONTEND_URL`

## Error Handling

- Invalid/expired tokens return 400 Bad Request
- Used tokens return 400 Bad Request with specific message
- Email sending failures are logged and return 500 Internal Server Error
- Invalid email format returns 400 Bad Request (validation error)

## API Documentation

The password reset endpoints are documented in the Swagger/OpenAPI documentation:
- Swagger UI: http://localhost:3000/swagger-ui/
- Scalar UI: http://localhost:3000/scalar