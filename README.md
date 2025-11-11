# Chalkbyte

A modern REST API built with Rust, Axum, and PostgreSQL featuring JWT-based authentication.

## Features

- 🔐 JWT-based authentication with bcrypt password hashing
- 🗄️ PostgreSQL database with SQLx migrations
- 🚀 Fast and type-safe with Rust and Axum
- 🔒 Protected routes with authentication middleware
- ✅ Request validation using the validator crate
- 🐳 Docker and Docker Compose support
- 📊 pgAdmin for database management

## Quick Start

1. **Start the database**:
```bash
docker-compose up -d postgres pgadmin
```

2. **Configure environment**:
```bash
cp .env.example .env
# Edit .env and set JWT_SECRET to a secure value
```

3. **Run migrations**:
```bash
cargo sqlx migrate run
```

4. **Start the server**:
```bash
cargo run
```

The API will be available at `http://localhost:3000`.

## Authentication

See [AUTHENTICATION.md](./AUTHENTICATION.md) for detailed documentation on authentication endpoints and usage.

### Quick Example

```bash
# Register
curl -X POST http://localhost:3000/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{"first_name":"John","last_name":"Doe","email":"john@example.com","password":"password123"}'

# Login
curl -X POST http://localhost:3000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"john@example.com","password":"password123"}'

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
