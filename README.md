# Chalkbyte

A modern REST API built with Rust, Axum, and PostgreSQL featuring JWT-based authentication.

## Features

- 🔐 JWT-based authentication with bcrypt password hashing
- 🗄️ PostgreSQL database with SQLx migrations
- 🚀 Fast and type-safe with Rust and Axum
- 🔒 Protected routes with authentication middleware
- ✅ Request validation using the validator crate
- 📚 **Interactive Swagger UI documentation**
- 🐳 Docker and Docker Compose support
- 📊 pgAdmin for database management

## Quick Start

See [QUICKSTART.md](./QUICKSTART.md) for a detailed step-by-step guide!

### TL;DR

```bash
# 1. Start database
docker-compose up -d postgres

# 2. Setup environment
cp .env.example .env

# 3. Run migrations
cargo sqlx migrate run

# 4. Start server
cargo run
```

Open `http://localhost:3000/swagger-ui` to explore the API! 🚀

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

## 📚 Documentation

- **[QUICKSTART.md](./QUICKSTART.md)** - Get started in 5 minutes
- **[AUTHENTICATION.md](./AUTHENTICATION.md)** - Authentication guide and API reference
- **[SWAGGER.md](./SWAGGER.md)** - Swagger UI usage and customization
- **[IMPLEMENTATION_SUMMARY.md](./IMPLEMENTATION_SUMMARY.md)** - Technical implementation details

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## 📄 License

This project is licensed under the MIT License.
