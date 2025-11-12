# Chalkbyte

A modern REST API built with Rust, Axum, and PostgreSQL featuring hierarchical role-based access control, school management, and JWT-based authentication.

## Features

- 🔐 JWT-based authentication with bcrypt password hashing
- 👥 Hierarchical role system (System Admin, School Admin, Teacher, Student)
- 🏫 Multi-school management with school isolation
- 🗄️ PostgreSQL database with SQLx migrations
- 🚀 Fast and type-safe with Rust and Axum
- 🔒 Protected routes with role-based authorization
- ✅ Request validation using the validator crate
- 📚 Interactive Swagger UI documentation
- 🐳 Docker and Docker Compose support
- 🛡️ CLI-only system admin creation for enhanced security

## Quick Start

### TL;DR

```bash
# 1. Start database
docker-compose up -d postgres

# 2. Setup environment
cp .env.example .env

# 3. Run migrations
cargo sqlx migrate run

# 4. Create system admin (CLI only)
cargo run -- create-sysadmin FirstName LastName admin@domain.com password

# 5. Start server
cargo run
```

Open `http://localhost:3000/swagger-ui` to explore the API! 🚀

See [docs/SETUP_GUIDE.md](./docs/SETUP_GUIDE.md) for complete setup instructions.

## API Documentation

Interactive Swagger UI documentation is available at:
```
http://localhost:3000/swagger-ui
```

The OpenAPI specification can be accessed at:
```
http://localhost:3000/api-docs/openapi.json
```

### Using Swagger UI

1. Open your browser and navigate to `http://localhost:3000/swagger-ui`
2. Browse all available endpoints organized by tags (Authentication, Users)
3. Click on any endpoint to see request/response schemas
4. Try out endpoints directly from the browser:
   - Click "Try it out"
   - Fill in the request body
   - Click "Execute"
5. For protected endpoints, click "Authorize" button and enter your JWT token

## Role-Based Access Control

The system implements a hierarchical role structure:

- **System Admin** (CLI-created) - Full system access, creates schools and school admins
- **School Admin** - Manages their assigned school, creates teachers and students
- **Teacher** - School staff with elevated permissions
- **Student** - Default role with basic access

See [docs/USER_ROLES.md](./docs/USER_ROLES.md) for detailed role documentation.

## Authentication

See [docs/AUTHENTICATION.md](./docs/AUTHENTICATION.md) for detailed documentation.

### Quick Example

```bash
# Login (no public registration)
curl -X POST http://localhost:3000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"admin@domain.com","password":"password123"}'

# Access protected route
curl http://localhost:3000/api/users/profile \
  -H "Authorization: Bearer YOUR_TOKEN_HERE"
```

## Structure

```bash
my_axum_api/
├── Cargo.toml
├── src/
│   ├── config/                  # Configuration (e.g., database, environment)
│   │   ├── mod.rs
│   │   └── database.rs
│   ├── modules/                # Feature-based modules (like NestJS modules)
│   │   ├── users/             # Users module
│   │   │   ├── mod.rs
│   │   │   ├── controller.rs  # Route handlers (like NestJS controllers)
│   │   │   ├── service.rs    # Business logic (like NestJS services)
│   │   │   ├── model.rs      # Data models and DTOs
│   │   │   └── router.rs     # Route definitions
│   │   ├── posts/            # Posts module (example of another feature)
│   │   │   ├── mod.rs
│   │   │   ├── controller.rs
│   │   │   ├── service.rs
│   │   │   ├── model.rs
│   │   │   └── router.rs
│   ├── utils/                 # Shared utilities (e.g., custom extractors, error handling)
│   │   ├── mod.rs
│   │   ├── errors.rs
│   │   └── extractors.rs
│   ├── db.rs                  # Database connection setup
│   ├── main.rs                # Application entry point
│   └── router.rs             # Root router to combine module routers
├── .env                       # Environment variables
└── README.md
```

## 📚 Documentation

- **[docs/SETUP_GUIDE.md](./docs/SETUP_GUIDE.md)** - Complete setup walkthrough
- **[docs/QUICK_REFERENCE.md](./docs/QUICK_REFERENCE.md)** - Quick command reference
- **[docs/USER_ROLES.md](./docs/USER_ROLES.md)** - Role system and permissions
- **[docs/AUTHENTICATION.md](./docs/AUTHENTICATION.md)** - Authentication guide
- **[docs/SYSTEM_ADMIN_IMPLEMENTATION.md](./docs/SYSTEM_ADMIN_IMPLEMENTATION.md)** - Technical details

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## 📄 License

This project is licensed under the MIT License.
