# Quick Start: Password Reset

This guide will help you quickly set up and test the password reset functionality.

## Prerequisites

- Docker and Docker Compose installed
- Rust toolchain installed
- PostgreSQL running (via docker-compose)
- A user account in the system

## Setup (5 minutes)

### 1. Update Environment Variables

Add these to your `.env` file:

```env
# Email Configuration (for Mailpit - local testing)
SMTP_HOST=localhost
SMTP_PORT=1025
SMTP_USERNAME=
SMTP_PASSWORD=
FROM_EMAIL=noreply@chalkbyte.com
FROM_NAME=Chalkbyte
FRONTEND_URL=http://localhost:3000
```

### 2. Start Mailpit

```bash
docker compose up mailpit -d
```

Verify it's running:
- Web UI: http://localhost:8025
- SMTP: localhost:1025

### 3. Run Migration

```bash
sqlx migrate run
```

### 4. Start Your Application

```bash
cargo run
```

## Quick Test (2 minutes)

### Option A: Using the Test Script

```bash
./scripts/test_password_reset.sh user@example.com
```

The script will guide you through the entire flow.

### Option B: Manual Testing

#### Step 1: Request Password Reset

```bash
curl -X POST http://localhost:3000/api/auth/forgot-password \
  -H "Content-Type: application/json" \
  -d '{"email":"user@example.com"}'
```

**Response:**
```json
{
  "message": "If an account exists with that email, a password reset link has been sent."
}
```

#### Step 2: Get Token from Email

1. Open http://localhost:8025 in your browser
2. Click on the latest email
3. Copy the token from the reset link URL
   - Format: `http://localhost:3000/reset-password?token=XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX`
   - Token is the UUID after `token=`

#### Step 3: Reset Password

```bash
curl -X POST http://localhost:3000/api/auth/reset-password \
  -H "Content-Type: application/json" \
  -d '{
    "token":"YOUR-TOKEN-HERE",
    "new_password":"newSecurePassword123"
  }'
```

**Response:**
```json
{
  "message": "Password has been reset successfully. You can now log in with your new password."
}
```

#### Step 4: Verify Login

```bash
curl -X POST http://localhost:3000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email":"user@example.com",
    "password":"newSecurePassword123"
  }'
```

**Success Response:**
```json
{
  "access_token": "eyJ...",
  "user": {
    "id": "...",
    "email": "user@example.com",
    ...
  }
}
```

## Using Swagger UI

1. Open http://localhost:3000/swagger-ui/
2. Navigate to **Authentication** section
3. Test endpoints:
   - `POST /api/auth/forgot-password`
   - `POST /api/auth/reset-password`

## Common Issues

### Email Not Received

**Check:**
- Mailpit is running: `docker compose ps mailpit`
- SMTP configuration in `.env` is correct
- User exists in database

**Solution:**
```bash
# Restart Mailpit
docker compose restart mailpit

# Check logs
docker compose logs mailpit
```

### Token Expired

Tokens expire after 1 hour. Request a new password reset.

### Token Already Used

Each token can only be used once. Request a new password reset.

### Invalid Email Format

Ensure email passes validation:
- Must be valid email format
- Example: `user@example.com`

## Production Setup

For production, replace Mailpit with a real SMTP service:

### Gmail Example

```env
SMTP_HOST=smtp.gmail.com
SMTP_PORT=587
SMTP_USERNAME=your-email@gmail.com
SMTP_PASSWORD=your-app-specific-password
FROM_EMAIL=noreply@yourdomain.com
FROM_NAME=Your Company
FRONTEND_URL=https://yourdomain.com
```

**Note:** For Gmail, create an [App Password](https://support.google.com/accounts/answer/185833).

### SendGrid Example

```env
SMTP_HOST=smtp.sendgrid.net
SMTP_PORT=587
SMTP_USERNAME=apikey
SMTP_PASSWORD=your-sendgrid-api-key
FROM_EMAIL=noreply@yourdomain.com
FROM_NAME=Your Company
FRONTEND_URL=https://yourdomain.com
```

## Email Templates

The feature includes two professional email templates:

1. **Password Reset Request**
   - Responsive HTML design
   - Clear reset button
   - 1-hour expiry notice
   - Security information

2. **Reset Confirmation**
   - Success notification
   - Security warning
   - Support contact info

Preview templates at http://localhost:8025 after testing.

## Security Notes

- ✅ Tokens expire after 1 hour
- ✅ One-time use only
- ✅ Email enumeration protection
- ✅ Secure UUID tokens
- ✅ Minimum 8-character passwords
- ✅ Confirmation emails sent
- ✅ HTTPS recommended for production

## Database Schema

```sql
-- password_reset_tokens table
id          | UUID (PK)
user_id     | UUID (FK -> users.id)
token       | TEXT (UNIQUE)
expires_at  | TIMESTAMPTZ
used        | BOOLEAN
created_at  | TIMESTAMPTZ
```

## API Reference

### Forgot Password
- **Endpoint:** `POST /api/auth/forgot-password`
- **Auth:** None
- **Rate Limit:** Consider adding in production
- **Response:** Always 200 (security)

### Reset Password
- **Endpoint:** `POST /api/auth/reset-password`
- **Auth:** None (token-based)
- **Validation:** 8+ character password
- **Response:** 200 on success, 400 on error

## Next Steps

- [ ] Set up production SMTP service
- [ ] Configure frontend reset password page
- [ ] Add rate limiting for forgot password
- [ ] Monitor email delivery in production
- [ ] Set up email analytics (optional)

## Support

- Documentation: `docs/EMAIL_CONFIGURATION.md`
- Swagger API: http://localhost:3000/swagger-ui/
- Test Script: `scripts/test_password_reset.sh`
- Mailpit UI: http://localhost:8025

## Troubleshooting

### Check Application Logs

```bash
# If running with cargo
cargo run

# Check for email-related errors
grep -i "email\|smtp" logs/app.log
```

### Verify Database

```bash
# Connect to PostgreSQL
psql -U chalkbyte -d chalkbyte_db

# Check tokens
SELECT * FROM password_reset_tokens ORDER BY created_at DESC LIMIT 5;

# Check users
SELECT id, email FROM users WHERE email = 'test@example.com';
```

### Test SMTP Connection

```bash
# Test Mailpit is accepting connections
telnet localhost 1025
# Type: QUIT and press enter to exit
```

## Complete Flow Diagram

```
User Requests Reset
       ↓
POST /api/auth/forgot-password
       ↓
System validates email
       ↓
Generate UUID token
       ↓
Save to database (1hr expiry)
       ↓
Send email via SMTP
       ↓
User receives email
       ↓
User clicks reset link
       ↓
Frontend extracts token
       ↓
POST /api/auth/reset-password
       ↓
Validate token (exists, not used, not expired)
       ↓
Update password in database
       ↓
Mark token as used
       ↓
Send confirmation email
       ↓
User logs in with new password
```

---

**Ready to test?** Run: `./scripts/test_password_reset.sh`
