# Chalkbyte

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
