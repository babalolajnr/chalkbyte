# List all available commands
default:
    @just --list

# Run the project locally
run:
    cargo run

# Run the project in release mode
run-release:
    cargo run --release

# Build the project
build:
    cargo build

# Build the project in release mode
build-release:
    cargo build --release

# Run tests
test:
    cargo test

# Run tests with output
test-verbose:
    cargo test -- --nocapture

# Check code without building
check:
    cargo check

# Format code
fmt:
    cargo fmt

# Check code formatting
fmt-check:
    cargo fmt -- --check

# Run clippy linter
lint:
    cargo clippy -- -D warnings

# Start all Docker Compose services
up:
    docker compose up -d

# Start services and show logs
up-logs:
    docker compose up

# Stop all Docker Compose services
down:
    docker compose down

# Stop services and remove volumes
down-volumes:
    docker compose down -v

# Restart all services
restart:
    docker compose restart

# Show service logs
logs service="":
    @if [ -z "{{service}}" ]; then \
        docker compose logs -f; \
    else \
        docker compose logs -f {{service}}; \
    fi

# Show Postgres logs
logs-db:
    docker compose logs -f postgres

# Show running services status
ps:
    docker compose ps

# Run database migrations
migrate:
    sqlx migrate run

# Revert last database migration
migrate-revert:
    sqlx migrate revert

# Create a new migration
migrate-new name:
    sqlx migrate add {{name}}

# Start only Postgres database
db-up:
    docker compose up -d postgres

# Stop only Postgres database
db-down:
    docker compose stop postgres

# Access Postgres CLI
db-shell:
    docker compose exec postgres psql -U chalkbyte -d chalkbyte_db

# Clean build artifacts
clean:
    cargo clean

# Clean and rebuild
rebuild: clean build

# Full setup: start services, wait, and run migrations
setup: up
    @echo "Waiting for database to be ready..."
    @sleep 5
    @just migrate

# Full restart: down, up, and migrate
reset: down-volumes up
    @echo "Waiting for database to be ready..."
    @sleep 5
    @just migrate

# Development workflow: format, lint, test, and run
dev: fmt lint test run

# CI workflow: format check, lint, test, and build
ci: fmt-check lint test build-release
