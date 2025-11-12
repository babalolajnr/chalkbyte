# Quick Reference Card

## Quick Start

```bash
# 1. Run migrations
cargo sqlx migrate run

# 2. Create system admin (CLI only)
cargo run -- create-sysadmin FirstName LastName email@domain.com password

# 3. Start server
cargo run

# Server runs on http://localhost:3000
```

## Role Hierarchy

```
System Admin (CLI-created)
    ↓ can create
Schools + School Admins (tied to schools)
    ↓ can create
Teachers + Students (tied to their school)
```

## Common Commands

### Create System Admin
```bash
cargo run -- create-sysadmin John Doe admin@system.com SecurePass123
```

### Login
```bash
curl -X POST http://localhost:3000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"admin@system.com","password":"SecurePass123"}'
```

### Create School (System Admin)
```bash
curl -X POST http://localhost:3000/api/schools \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name":"School Name","address":"Address"}'
```

### Create School Admin (System Admin)
```bash
curl -X POST http://localhost:3000/api/users \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "first_name":"Principal",
    "last_name":"Smith",
    "email":"principal@school.edu",
    "role":"admin",
    "school_id":"SCHOOL_UUID"
  }'
```

### Create Teacher/Student (School Admin)
```bash
curl -X POST http://localhost:3000/api/users \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "first_name":"Jane",
    "last_name":"Doe",
    "email":"user@school.edu",
    "role":"teacher"
  }'
```

### List Schools (System Admin)
```bash
curl -X GET http://localhost:3000/api/schools \
  -H "Authorization: Bearer TOKEN"
```

### List Users (Scoped by Role)
```bash
curl -X GET http://localhost:3000/api/users \
  -H "Authorization: Bearer TOKEN"
```

## Permission Matrix

| Action | System Admin | School Admin | Teacher | Student |
|--------|:------------:|:------------:|:-------:|:-------:|
| Create system admin | CLI | ❌ | ❌ | ❌ |
| Create schools | ✅ | ❌ | ❌ | ❌ |
| Create any user | ✅ | ❌ | ❌ | ❌ |
| Create school users | ✅ | ✅* | ❌ | ❌ |
| View all users | ✅ | ❌ | ❌ | ❌ |
| View school users | ✅ | ✅* | ❌ | ❌ |

*Only for their own school

## Key Features

✅ **No Public Registration** - Users created by admins only  
✅ **Unique School Names** - Prevents duplicates  
✅ **CLI System Admin** - Secure admin creation  
✅ **School Isolation** - School admins limited to their school  
✅ **Role-Based Access** - Enforced at API level  

## API Endpoints

**Authentication**
- `POST /api/auth/login` - Login

**Schools** (System Admin only)
- `POST /api/schools` - Create school
- `GET /api/schools` - List schools
- `GET /api/schools/{id}` - Get school
- `DELETE /api/schools/{id}` - Delete school

**Users** (Admin only)
- `POST /api/users` - Create user
- `GET /api/users` - List users (scoped)
- `GET /api/users/profile` - Get profile

## Documentation

- Swagger UI: http://localhost:3000/swagger-ui
- Scalar UI: http://localhost:3000/scalar

## Troubleshooting

**Cannot create users**
→ Must be logged in as admin or system_admin

**School name already exists**
→ School names must be unique

**Forbidden error**
→ Check role has permission for action

**404 on /register**
→ Public registration removed (by design)

**School admin cannot create users**
→ Verify school_id is assigned to admin

## Database Queries

```sql
-- View all schools
SELECT * FROM schools;

-- View all users with their schools
SELECT u.email, u.role, s.name as school 
FROM users u 
LEFT JOIN schools s ON u.school_id = s.id;

-- View users by school
SELECT * FROM users WHERE school_id = 'SCHOOL_UUID';

-- Create system admin manually
INSERT INTO users (first_name, last_name, email, password, role)
VALUES ('Name', 'Surname', 'email@domain.com', 'hashed_password', 'system_admin');
```

## Environment Variables

```env
DATABASE_URL=postgresql://user:password@localhost:5432/dbname
JWT_SECRET=your-secret-key
JWT_ACCESS_TOKEN_EXPIRY=3600
```

## File Structure

```
src/
├── cli/              # CLI commands
├── modules/
│   ├── auth/        # Authentication
│   ├── schools/     # School management
│   └── users/       # User management
├── middleware/      # Auth middleware
└── utils/           # Utilities

migrations/          # Database migrations
├── 20251111195802_add_user_roles.sql
├── 20251112081331_add_system_admin_and_schools.sql
└── 20251112083030_add_unique_school_name.sql
```

## Notes

- Users created via API don't have passwords initially
- Implement password invitation flow separately
- System admin CLI command requires database access
- School names are case-sensitive
- JWT tokens expire based on configuration

## Support

See detailed documentation:
- `SETUP_GUIDE.md` - Complete setup walkthrough
- `USER_ROLES.md` - Role system details
- `SYSTEM_ADMIN_IMPLEMENTATION.md` - Technical details
- `CHANGES_SUMMARY.md` - All changes made
