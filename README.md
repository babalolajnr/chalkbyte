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

# 4. Create system admin (CLI - interactive mode)
cargo run --bin chalkbyte-cli -- create-sysadmin

# 5. Start server
cargo run --bin chalkbyte
```

Open `http://localhost:3000/swagger-ui` to explore the API! 🚀

See [docs/SETUP_GUIDE.md](./docs/SETUP_GUIDE.md) for complete setup instructions.

## 📊 Observability & Monitoring

Chalkbyte includes a comprehensive observability stack with Grafana, Loki, Tempo, and Prometheus - fully configured and ready to use.

### Quick Start

```bash
# Start with observability enabled
docker compose --profile observability up -d

# Add to your .env file
echo "OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317" >> .env
echo "ENVIRONMENT=development" >> .env

# Start the API
cargo run
```

### What's Included

- **📝 Distributed Tracing** (OpenTelemetry + Tempo): Full request tracing with OTLP export
- **📊 Log Aggregation** (Loki + Promtail): Centralized, searchable logs with trace correlation
- **📈 Metrics Collection** (Prometheus): System and application metrics
- **📊 Unified Dashboard** (Grafana): Pre-configured dashboards for traces, logs, and metrics
- **🔍 Trace-to-Log Correlation**: Click a trace to see all related logs

### Access Points

- **Grafana**: http://localhost:3001 (admin/admin123)
- **Prometheus**: http://localhost:9090
- **Tempo**: http://localhost:3200
- **Loki**: http://localhost:3100
- **API Health**: http://localhost:3000/health
- **OTLP Collector**: http://localhost:4317 (gRPC)

### Current Features

- ✅ **Distributed Tracing**: Every HTTP request automatically traced end-to-end
- ✅ **Structured Logging**: JSON logs with automatic trace ID correlation
- ✅ **Performance Monitoring**: Request latency, error rates, and endpoint performance
- ✅ **Log Correlation**: Click any trace to see all related logs
- ✅ **System Metrics**: CPU, memory, disk, and network monitoring
- ✅ **Pre-built Dashboards**: Ready-to-use Grafana dashboards

### Testing the Setup

Run the test script to verify everything is working:
```bash
./scripts/test-observability.sh
```

### Docker Compose Profiles

The observability stack uses Docker Compose profiles for easy management:

```bash
# Start without observability (default)
docker compose up -d

# Start with observability
docker compose --profile observability up -d

# Stop everything
docker compose --profile observability down
```

See [OBSERVABILITY_QUICK_START.md](./OBSERVABILITY_QUICK_START.md) for detailed setup guide and [docs/OBSERVABILITY.md](./docs/OBSERVABILITY.md) for complete documentation.

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

## CLI Tool

The project includes a separate CLI binary for administrative tasks with support for both interactive and non-interactive modes:

### Create System Admin

```bash
# Show CLI help
cargo run --bin chalkbyte-cli -- --help
cargo run --bin chalkbyte-cli -- create-sysadmin --help

# Interactive mode - prompts for all inputs with secure password entry
cargo run --bin chalkbyte-cli -- create-sysadmin

# Non-interactive mode - provide all arguments
cargo run --bin chalkbyte-cli -- create-sysadmin \
  --first-name John \
  --last-name Doe \
  --email john@example.com \
  --password secure123

# Mixed mode - provide some arguments, prompted for the rest
cargo run --bin chalkbyte-cli -- create-sysadmin \
  --first-name John \
  --last-name Doe

# Using justfile shortcuts
just cli                                                    # Show help
just create-sysadmin John Doe john@example.com secure123   # Create admin
```

### Database Seeders

Populate your database with fake data for development and testing:

```bash
# Seed with default values (5 schools, 2 admins, 5 teachers, 20 students per school)
cargo run --bin chalkbyte-cli -- seed

# Custom seed
cargo run --bin chalkbyte-cli -- seed -s 10 --admins 3 --teachers 8 --students 30

# Minimal seed for quick testing
cargo run --bin chalkbyte-cli -- seed -s 2 --admins 1 --teachers 2 --students 5

# Clear all seeded data (keeps system admins)
cargo run --bin chalkbyte-cli -- clear-seed

# Using justfile shortcuts
just seed                    # Default seed
just seed-minimal            # Quick testing
just seed-custom 50 3 10 25  # Custom values
just clear-seed              # Cleanup
```

**Default password for seeded users: `password123`**

**Performance:** Highly optimized with Rayon parallelization and batch inserts
- 12,000 users in ~1.4s
- 24,000 users in ~2.5s
- Parallel data generation across all CPU cores
- Batch inserts (500 schools, 1000 users per batch)

See [docs/SEEDERS.md](./docs/SEEDERS.md) for detailed seeder documentation.

The CLI binary is separate from the main server application for better separation of concerns and reduced binary size.

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
- **[docs/CLI_GUIDE.md](./docs/CLI_GUIDE.md)** - CLI tool usage and examples
- **[docs/QUICK_REFERENCE.md](./docs/QUICK_REFERENCE.md)** - Quick command reference
- **[docs/USER_ROLES.md](./docs/USER_ROLES.md)** - Role system and permissions
- **[docs/AUTHENTICATION.md](./docs/AUTHENTICATION.md)** - Authentication guide
- **[docs/OBSERVABILITY_SETUP.md](./docs/OBSERVABILITY_SETUP.md)** - Observability quick start
- **[docs/OBSERVABILITY.md](./docs/OBSERVABILITY.md)** - Complete observability guide
- **[docs/OBSERVABILITY_INTEGRATION_STATUS.md](./docs/OBSERVABILITY_INTEGRATION_STATUS.md)** - Current status
- **[docs/SYSTEM_ADMIN_IMPLEMENTATION.md](./docs/SYSTEM_ADMIN_IMPLEMENTATION.md)** - Technical details

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## 📄 License

This project is licensed under the MIT License.
