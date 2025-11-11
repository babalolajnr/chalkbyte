# Chalkbyte API - Quick Start Guide

## 🚀 Getting Started in 5 Minutes

### 1. Start the Database
```bash
docker-compose up -d postgres
```

### 2. Setup Environment
```bash
cp .env.example .env
```

### 3. Run Migrations
```bash
cargo sqlx migrate run
```

### 4. Start the Server
```bash
cargo run
```

You should see:
```
🚀 Server running on http://localhost:3000
📚 Swagger UI available at http://localhost:3000/swagger-ui
```

## 📚 Interactive Documentation

Open your browser and go to:
```
http://localhost:3000/swagger-ui
```

## 🎯 Try the API (Using Swagger UI)

### Step 1: Register a User

1. In Swagger UI, find **Authentication** section
2. Click on `POST /api/auth/register`
3. Click **"Try it out"** button
4. You'll see a pre-filled request body:
```json
{
  "first_name": "John",
  "last_name": "Doe",
  "email": "john@example.com",
  "password": "password123"
}
```
5. Click **"Execute"**
6. See the response with your new user!

### Step 2: Login to Get JWT Token

1. Click on `POST /api/auth/login`
2. Click **"Try it out"**
3. Enter your credentials:
```json
{
  "email": "john@example.com",
  "password": "password123"
}
```
4. Click **"Execute"**
5. Copy the `access_token` from the response

### Step 3: Authorize Protected Endpoints

1. Click the 🔒 **Authorize** button at the top right
2. In the dialog, paste your token:
```
Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...
```
3. Click **"Authorize"**
4. Click **"Close"**

### Step 4: Access Protected Endpoints

1. Click on `GET /api/users/profile`
2. Click **"Try it out"**
3. Click **"Execute"**
4. See your user profile from the JWT token! 🎉

## 📝 Alternative: Using cURL

If you prefer command line:

### Register
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

### Login
```bash
curl -X POST http://localhost:3000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "john@example.com",
    "password": "password123"
  }'
```

### Get Profile (replace TOKEN with your actual token)
```bash
curl http://localhost:3000/api/users/profile \
  -H "Authorization: Bearer YOUR_TOKEN_HERE"
```

## 🔍 Available Endpoints

### Public (No Authentication Required)
- `POST /api/auth/register` - Create a new account
- `POST /api/auth/login` - Login and get JWT token

### Protected (Requires JWT Token)
- `GET /api/users` - List all users
- `GET /api/users/profile` - Get your profile from token

### Development
- `GET /swagger-ui` - Interactive API documentation
- `GET /api-docs/openapi.json` - OpenAPI specification

## 🛠️ Configuration

### Environment Variables

Edit your `.env` file:

```env
# Database
DATABASE_URL=postgresql://chalkbyte:chalkbyte_password@localhost:5432/chalkbyte_db

# JWT Settings
JWT_SECRET=your-secret-key-change-in-production
JWT_ACCESS_EXPIRY=3600        # 1 hour
JWT_REFRESH_EXPIRY=604800     # 7 days

# Server
PORT=3000

# Logging
RUST_LOG=chalkbyte=debug,tower_http=debug,sqlx=info
```

**Important**: Change `JWT_SECRET` to a strong, random value in production!

## 🎨 Swagger UI Features

### What You Can Do:
- ✅ Browse all endpoints organized by category
- ✅ See request/response schemas with examples
- ✅ Test endpoints directly in the browser
- ✅ Authenticate once, test all protected endpoints
- ✅ See validation rules and field constraints
- ✅ Export OpenAPI spec for client generation

### Tips:
- 🔒 **Authorize button**: Use this once to add your JWT to all requests
- 💡 **Try it out**: Click this to edit and send requests
- 📋 **Example values**: Pre-filled with working examples
- ⚠️ **Required fields**: Marked with a red asterisk (*)

## 📚 Learn More

- [AUTHENTICATION.md](./AUTHENTICATION.md) - Detailed authentication guide
- [SWAGGER.md](./SWAGGER.md) - Swagger customization and advanced features
- [IMPLEMENTATION_SUMMARY.md](./IMPLEMENTATION_SUMMARY.md) - Technical implementation details

## 🐳 Using Docker

To run the entire stack (app + database):

```bash
# Start everything
docker-compose up -d

# View logs
docker-compose logs -f chalkbyte

# Stop everything
docker-compose down
```

## 🧪 Testing

### Manual Testing Checklist
- [ ] Register a new user
- [ ] Login with correct credentials
- [ ] Try login with wrong password (should fail)
- [ ] Access protected endpoint without token (should fail)
- [ ] Access protected endpoint with valid token (should work)
- [ ] Access Swagger UI
- [ ] Test endpoints through Swagger UI

### Automated Testing (Future)
```bash
cargo test
```

## ❓ Troubleshooting

### Can't connect to database
```bash
# Check if PostgreSQL is running
docker ps | grep postgres

# Check database connection
docker exec chalkbyte-postgres-1 psql -U chalkbyte -d chalkbyte_db -c "SELECT 1"
```

### Swagger UI not loading
- Ensure server is running: `http://localhost:3000`
- Check browser console for errors
- Try accessing OpenAPI spec directly: `http://localhost:3000/api-docs/openapi.json`

### JWT token expired
- Tokens expire after 1 hour (default)
- Login again to get a new token
- Update the JWT_ACCESS_EXPIRY environment variable if needed

### Port 3000 already in use
```bash
# Find what's using the port
lsof -i :3000

# Kill the process
kill -9 <PID>

# Or change the port in .env
PORT=8080
```

## 🎓 Next Steps

1. ✅ Complete the Quick Start above
2. 📖 Read [AUTHENTICATION.md](./AUTHENTICATION.md) for security details
3. 🎨 Explore [SWAGGER.md](./SWAGGER.md) to customize documentation
4. 🔧 Check [IMPLEMENTATION_SUMMARY.md](./IMPLEMENTATION_SUMMARY.md) for architecture
5. 🚀 Start building your features!

## 💡 Tips for Development

- Use **Swagger UI** for quick testing during development
- Keep your **JWT_SECRET** secure and complex
- Check server logs for detailed request/response information
- Use pgAdmin (`http://localhost:8080`) to inspect database
- Enable RUST_LOG for detailed debugging: `RUST_LOG=debug`

Happy coding! 🎉
