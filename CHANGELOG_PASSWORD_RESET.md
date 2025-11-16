# Password Reset Feature Changelog

## Overview
Added complete forgot/reset password functionality with email integration and testing support.

## Changes Made

### 1. Dependencies Added
- **lettre v0.11**: Email sending library with SMTP support
  - Added to `Cargo.toml` with features: `tokio1-native-tls`, `builder`, `smtp-transport`

### 2. Docker Compose Updates
- **Mailpit Service**: Added email testing service
  - SMTP port: 1025
  - Web UI port: 8025
  - Configuration: Accepts any authentication for development

### 3. Database Migration
- **File**: `migrations/20251116142011_add_password_reset_tokens.sql`
- **Table**: `password_reset_tokens`
  - `id`: UUID primary key
  - `user_id`: Foreign key to users table
  - `token`: Unique reset token (UUID)
  - `expires_at`: Token expiration timestamp
  - `used`: Boolean flag for one-time use
  - `created_at`: Timestamp
- **Indexes**: Added on user_id, token, and expires_at for performance

### 4. Configuration Module
- **File**: `src/config/email.rs`
- Environment variables:
  - `SMTP_HOST`: Default localhost
  - `SMTP_PORT`: Default 1025
  - `SMTP_USERNAME`: Optional
  - `SMTP_PASSWORD`: Optional
  - `FROM_EMAIL`: Default noreply@chalkbyte.com
  - `FROM_NAME`: Default Chalkbyte
  - `FRONTEND_URL`: Default http://localhost:3000

### 5. Email Service
- **File**: `src/utils/email.rs`
- **EmailService struct**: Handles all email operations
- **Methods**:
  - `send_password_reset_email()`: Sends reset link email
  - `send_password_reset_confirmation()`: Sends confirmation email
- **Templates**: Professional HTML emails with plain text fallback
  - Password reset request template (styled with Chalkbyte branding)
  - Password reset confirmation template (success notification)

### 6. Auth Module Updates

#### Models (`src/modules/auth/model.rs`)
- `ForgotPasswordRequest`: DTO for requesting password reset
- `ResetPasswordRequest`: DTO for resetting password with token
- `MessageResponse`: Generic success message response

#### Service (`src/modules/auth/service.rs`)
- `forgot_password()`: 
  - Validates user exists
  - Generates UUID token
  - Stores token with 1-hour expiry
  - Sends reset email
  - Returns success regardless of email existence (security)
  
- `reset_password()`:
  - Validates token exists and not expired
  - Checks token hasn't been used
  - Updates user password
  - Marks token as used
  - Sends confirmation email

#### Controller (`src/modules/auth/controller.rs`)
- `forgot_password`: POST endpoint handler
- `reset_password`: POST endpoint handler
- Both include OpenAPI documentation

#### Router (`src/modules/auth/router.rs`)
- Route: `/auth/forgot-password` (POST)
- Route: `/auth/reset-password` (POST)

### 7. AppState Updates
- **File**: `src/db.rs`
- Added `email_config: EmailConfig` field
- Initialized in `init_app_state()`

### 8. OpenAPI Documentation
- **File**: `src/docs.rs`
- Added forgot/reset password endpoints to API docs
- Added new DTOs to schemas
- Available in Swagger UI and Scalar UI

### 9. Documentation
- **File**: `docs/EMAIL_CONFIGURATION.md`
  - Complete setup guide
  - Environment variable documentation
  - API endpoint examples
  - Testing instructions
  - Production SMTP configurations (Gmail, SendGrid, AWS SES)
  - Security considerations

### 10. Test Script
- **File**: `scripts/test_password_reset.sh`
- Interactive test script for full password reset flow
- Guides user through:
  1. Requesting password reset
  2. Checking Mailpit for email
  3. Extracting token
  4. Resetting password
  5. Testing login with new password

## Security Features

1. **Email Enumeration Prevention**: Same response regardless of email existence
2. **Token Expiry**: 1-hour expiration on reset tokens
3. **One-time Use**: Tokens marked as used after successful reset
4. **Old Token Cleanup**: Previous unused tokens deleted when requesting new reset
5. **Cryptographic Tokens**: UUIDs for unpredictable token generation
6. **Password Validation**: Minimum 8 characters enforced
7. **Confirmation Emails**: Users notified of successful password changes

## API Endpoints

### POST /api/auth/forgot-password
Request password reset email

**Request**:
```json
{
  "email": "user@example.com"
}
```

**Response**: 200 OK
```json
{
  "message": "If an account exists with that email, a password reset link has been sent."
}
```

### POST /api/auth/reset-password
Reset password using token

**Request**:
```json
{
  "token": "uuid-token",
  "new_password": "newPassword123"
}
```

**Response**: 200 OK
```json
{
  "message": "Password has been reset successfully. You can now log in with your new password."
}
```

## Testing

### Development Setup
1. Start Mailpit: `docker compose up mailpit -d`
2. Start application with email config in `.env`
3. Run test script: `./scripts/test_password_reset.sh user@example.com`
4. View emails at http://localhost:8025

### Manual Testing
Use curl commands or Swagger UI at http://localhost:3000/swagger-ui/

## Email Templates

Both templates feature:
- Responsive HTML design
- Chalkbyte branding colors
- Clear call-to-action buttons
- Plain text fallback
- Security notices
- Professional styling

## Future Enhancements

Potential improvements:
- Rate limiting on password reset requests
- Email verification on registration
- Two-factor authentication
- Password reset history/audit log
- Custom email templates via database
- Email template variables/personalization
- Background job queue for email sending

## Migration Path

To apply this feature to existing installations:
1. Run `cargo build` to download new dependencies
2. Run `sqlx migrate run` to create password_reset_tokens table
3. Add email configuration to `.env`
4. Start Mailpit or configure production SMTP
5. Restart application

## Rollback

To rollback if needed:
1. Revert code changes
2. Run: `sqlx migrate revert` to drop password_reset_tokens table
3. Remove email configuration from `.env`
4. Remove Mailpit from docker-compose.yml

## Notes

- Email sending is async and non-blocking
- SMTP connections use connection pooling via lettre
- Failed email sends return 500 error (logged for debugging)
- Mailpit stores emails in memory (lost on restart)
- Production should use dedicated SMTP service
- Consider adding Redis for token storage in high-traffic scenarios