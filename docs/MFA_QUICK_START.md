# MFA Quick Start Guide

## TL;DR

Add two-factor authentication to your Chalkbyte account in 3 steps:

1. **Enable MFA** → Get QR code
2. **Scan QR code** → With authenticator app
3. **Verify code** → Save recovery codes

---

## Step-by-Step Setup

### 1. Login and Get Token

```bash
TOKEN=$(curl -s -X POST http://localhost:3000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"user@example.com","password":"password123"}' \
  | jq -r '.access_token')
```

### 2. Enable MFA

```bash
curl -X POST -H "Authorization: Bearer $TOKEN" \
  http://localhost:3000/api/mfa/enable | jq
```

**Response:**
```json
{
  "secret": "WI2YCNMT44OIQEUF2TV4UOEU2C7ARUWA",
  "qr_code_url": "otpauth://totp/Chalkbyte:user@example.com?secret=...",
  "qr_code_base64": "data:image/png;base64,iVBORw0KGgo...",
  "manual_entry_key": "WI2YCNMT44OIQEUF2TV4UOEU2C7ARUWA"
}
```

### 3. Scan QR Code

**Option A: Display in Frontend**
```html
<img src="data:image/png;base64,iVBORw0KGgo..." alt="MFA QR Code" />
```

**Option B: Save and Open**
```bash
# Extract base64 image data (remove data URI prefix)
echo "iVBORw0KGgo..." | base64 -d > qr.png
open qr.png  # macOS
xdg-open qr.png  # Linux
```

**Scan with any authenticator app:**
- Google Authenticator
- Microsoft Authenticator
- Authy
- 1Password
- Bitwarden

### 4. Verify and Activate

```bash
# Enter the 6-digit code from your app
curl -X POST \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"code": "123456"}' \
  http://localhost:3000/api/mfa/verify | jq
```

**Response:**
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

**⚠️ SAVE THESE RECOVERY CODES!** Each can be used once if you lose your device.

---

## Login with MFA Enabled

### Step 1: Initial Login

```bash
curl -X POST \
  -H "Content-Type: application/json" \
  -d '{"email":"user@example.com","password":"password123"}' \
  http://localhost:3000/api/auth/login | jq
```

**Response:**
```json
{
  "mfa_required": true,
  "temp_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
}
```

### Step 2: Verify TOTP Code

```bash
TEMP_TOKEN="eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."

curl -X POST \
  -H "Content-Type: application/json" \
  -d "{\"temp_token\":\"$TEMP_TOKEN\",\"code\":\"123456\"}" \
  http://localhost:3000/api/auth/mfa/verify | jq
```

**Response:**
```json
{
  "access_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "user": { ... }
}
```

### Use Recovery Code (Alternative)

If you lost your authenticator:

```bash
curl -X POST \
  -H "Content-Type: application/json" \
  -d "{\"temp_token\":\"$TEMP_TOKEN\",\"recovery_code\":\"ABCD1234\"}" \
  http://localhost:3000/api/auth/mfa/recovery | jq
```

---

## Management Commands

### Check Status

```bash
curl -H "Authorization: Bearer $TOKEN" \
  http://localhost:3000/api/mfa/status | jq
```

### Regenerate Recovery Codes

```bash
curl -X POST -H "Authorization: Bearer $TOKEN" \
  http://localhost:3000/api/mfa/recovery-codes/regenerate | jq
```

### Disable MFA

```bash
curl -X POST \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"password":"password123"}' \
  http://localhost:3000/api/mfa/disable | jq
```

---

## Frontend Integration

See `docs/mfa_frontend_example.html` for a complete working example with:
- ✅ Beautiful UI
- ✅ QR code display
- ✅ Step-by-step wizard
- ✅ Recovery codes download
- ✅ Error handling

Open in browser:
```bash
open docs/mfa_frontend_example.html
```

---

## Common Issues

### "Invalid MFA code"
- ✓ Check device time is synced
- ✓ Wait for next code (30-second refresh)
- ✓ Verify correct account in app

### QR Code Won't Scan
- ✓ Use `manual_entry_key` instead
- ✓ Increase brightness
- ✓ Try different authenticator app

### Lost Authenticator
- ✓ Use a recovery code to login
- ✓ Then regenerate codes or disable/re-enable MFA

---

## Security Best Practices

1. **Save recovery codes offline** (password manager, printed paper)
2. **Don't share QR code or secret** with anyone
3. **Use different authenticator app** than your email provider
4. **Keep backup device** with MFA configured
5. **Regenerate codes** if you suspect compromise

---

## API Reference

| Endpoint | Method | Auth | Description |
|----------|--------|------|-------------|
| `/api/mfa/status` | GET | Bearer | Check if MFA is enabled |
| `/api/mfa/enable` | POST | Bearer | Generate TOTP secret & QR |
| `/api/mfa/verify` | POST | Bearer | Activate MFA with code |
| `/api/mfa/disable` | POST | Bearer | Disable MFA (requires password) |
| `/api/mfa/recovery-codes/regenerate` | POST | Bearer | Get new recovery codes |
| `/api/auth/mfa/verify` | POST | None | Complete login with TOTP |
| `/api/auth/mfa/recovery` | POST | None | Complete login with recovery code |

---

## Technical Details

- **Algorithm**: TOTP (RFC 6238)
- **Hash**: SHA1
- **Digits**: 6
- **Period**: 30 seconds
- **QR Format**: PNG, base64-encoded data URI
- **Recovery Codes**: 10 codes, 8 chars, single-use, bcrypt hashed
- **Temp Token**: 10-minute expiry

---

Need help? Check `docs/MFA_GUIDE.md` for detailed documentation.