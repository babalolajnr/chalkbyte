# MFA (Multi-Factor Authentication) Guide

## Overview

Chalkbyte now supports TOTP-based Multi-Factor Authentication (MFA) using authenticator apps like Google Authenticator, Authy, Microsoft Authenticator, or 1Password.

## Features

- TOTP (Time-based One-Time Password) authentication
- QR code enrollment for easy setup
- 10 single-use recovery codes
- Password-protected MFA disable
- Recovery code regeneration

## API Endpoints

### 1. Check MFA Status

**GET** `/api/mfa/status`

Returns whether MFA is enabled for the authenticated user.

```bash
curl -H "Authorization: Bearer $TOKEN" \
  http://localhost:3000/api/mfa/status
```

Response:
```json
{
  "mfa_enabled": false
}
```

### 2. Enable MFA (Step 1: Generate Secret)

**POST** `/api/mfa/enable`

Generates a TOTP secret and QR code URL. MFA is not yet active.

```bash
curl -X POST \
  -H "Authorization: Bearer $TOKEN" \
  http://localhost:3000/api/mfa/enable
```

Response:
```json
{
  "secret": "JBSWY3DPEHPK3PXP",
  "qr_code_url": "otpauth://totp/Chalkbyte:user@example.com?secret=JBSWY3DPEHPK3PXP&issuer=Chalkbyte",
  "qr_code_base64": "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAASwAAAEs...",
  "manual_entry_key": "JBSWY3DPEHPK3PXP"
}
```

**Setup your authenticator app:**
- Display `qr_code_base64` as an image in your frontend and scan it
- OR open `qr_code_url` in a browser to see the QR code
- OR manually enter the `manual_entry_key`

**Note:** The `qr_code_base64` field contains a base64-encoded PNG image that can be directly embedded in HTML:
```html
<img src="data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAASwAAAEs..." alt="MFA QR Code" />
```

### 3. Verify and Activate MFA (Step 2: Confirm Setup)

**POST** `/api/mfa/verify`

Verify the TOTP code from your authenticator app to activate MFA. Returns recovery codes.

```bash
curl -X POST \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"code": "123456"}' \
  http://localhost:3000/api/mfa/verify
```

Response:
```json
{
  "recovery_codes": [
    "ABCD1234",
    "EFGH5678",
    "IJKL9012",
    "MNOP3456",
    "QRST7890",
    "UVWX1234",
    "YZAB5678",
    "CDEF9012",
    "GHIJ3456",
    "KLMN7890"
  ]
}
```

**IMPORTANT:** Save these recovery codes in a secure location. Each code can only be used once.

### 4. Disable MFA

**POST** `/api/mfa/disable`

Disables MFA and deletes all recovery codes. Requires password confirmation.

```bash
curl -X POST \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"password": "your_password"}' \
  http://localhost:3000/api/mfa/disable
```

Response:
```json
{
  "message": "MFA has been disabled successfully"
}
```

### 5. Regenerate Recovery Codes

**POST** `/api/mfa/recovery-codes/regenerate`

Generate new recovery codes (invalidates all previous codes).

```bash
curl -X POST \
  -H "Authorization: Bearer $TOKEN" \
  http://localhost:3000/api/mfa/recovery-codes/regenerate
```

Response:
```json
{
  "recovery_codes": [
    "NEW1ABCD",
    "NEW2EFGH",
    ...
  ]
}
```

## Login Flow with MFA

### Normal Login (Without MFA)

**POST** `/api/auth/login`

```bash
curl -X POST \
  -H "Content-Type: application/json" \
  -d '{"email": "user@example.com", "password": "password123"}' \
  http://localhost:3000/api/auth/login
```

Response (no MFA):
```json
{
  "access_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "user": {
    "id": "uuid",
    "email": "user@example.com",
    ...
  }
}
```

### Login with MFA Enabled

**Step 1:** Initial login returns temporary token

```bash
curl -X POST \
  -H "Content-Type: application/json" \
  -d '{"email": "user@example.com", "password": "password123"}' \
  http://localhost:3000/api/auth/login
```

Response (MFA required):
```json
{
  "mfa_required": true,
  "temp_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
}
```

**Step 2:** Verify TOTP code

**POST** `/api/auth/mfa/verify`

```bash
curl -X POST \
  -H "Content-Type: application/json" \
  -d '{
    "temp_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
    "code": "123456"
  }' \
  http://localhost:3000/api/auth/mfa/verify
```

Response:
```json
{
  "access_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "user": {
    "id": "uuid",
    "email": "user@example.com",
    ...
  }
}
```

### Login with Recovery Code

If you lost access to your authenticator app, use a recovery code instead.

**POST** `/api/auth/mfa/recovery`

```bash
curl -X POST \
  -H "Content-Type: application/json" \
  -d '{
    "temp_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
    "recovery_code": "ABCD1234"
  }' \
  http://localhost:3000/api/auth/mfa/recovery
```

Response:
```json
{
  "access_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "user": {...}
}
```

**Note:** Recovery codes are single-use and will be marked as used after successful login.

## Complete Enrollment Example

```bash
# 1. Login and get access token
TOKEN=$(curl -s -X POST http://localhost:3000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"user@example.com","password":"password123"}' \
  | jq -r '.access_token')

# 2. Enable MFA and get QR code
curl -X POST -H "Authorization: Bearer $TOKEN" \
  http://localhost:3000/api/mfa/enable \
  | jq

# 3. Scan QR code with authenticator app

# 4. Verify setup with TOTP code from app
curl -X POST \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"code": "123456"}' \
  http://localhost:3000/api/mfa/verify \
  | jq

# 5. Save the recovery codes!
```

## Database Schema

### Users Table (MFA Fields)

```sql
ALTER TABLE users ADD COLUMN mfa_enabled BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE users ADD COLUMN mfa_secret TEXT;
```

### MFA Recovery Codes Table

```sql
CREATE TABLE mfa_recovery_codes (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    code_hash TEXT NOT NULL,
    used BOOLEAN NOT NULL DEFAULT FALSE,
    used_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

## Security Considerations

1. **Recovery Codes**: Stored as bcrypt hashes, not plaintext
2. **Temp Token**: 10-minute expiry for MFA verification
3. **Single-Use Recovery Codes**: Each code marked as used after successful authentication
4. **Password Verification**: Required to disable MFA
5. **TOTP Algorithm**: SHA1, 6 digits, 30-second time step (industry standard)

## Troubleshooting

### "Invalid MFA code" Error

- Ensure device time is synchronized (TOTP is time-based)
- Wait for next code cycle (codes refresh every 30 seconds)
- Check if you're using the correct account in your authenticator app

### Lost Authenticator Device

- Use a recovery code to log in
- After login, regenerate recovery codes or disable and re-enable MFA

### Recovery Codes Not Working

- Each code can only be used once
- Codes are case-sensitive (uppercase letters and numbers)
- Ensure you're using the most recent set if you regenerated them

## Implementation Details

- **Library**: `totp-rs` v5.6 for TOTP generation and verification
- **QR Code**: Base64-encoded PNG image (data URI format) + `otpauth://` URL
- **QR Code Format**: Compatible with all major authenticator apps (Google Authenticator, Authy, Microsoft Authenticator, 1Password, etc.)
- **Recovery Codes**: 10 codes, 8 characters each (alphanumeric)
- **Algorithm**: TOTP with SHA1, 6-digit codes, 30-second window