# List all available commands
default:
    @just --list

# Run the server locally
run:
    cargo run --bin chalkbyte

# Run the server in release mode
run-release:
    cargo run --release --bin chalkbyte

# Run CLI (show help)
cli:
    cargo run --bin chalkbyte-cli -- --help

# Create system admin via CLI (interactive mode)
create-sysadmin-interactive:
    cargo run --bin chalkbyte-cli -- create-sysadmin

# Create system admin via CLI (non-interactive mode)
create-sysadmin first_name last_name email password:
    cargo run --bin chalkbyte-cli -- create-sysadmin --first-name {{first_name}} --last-name {{last_name}} --email {{email}} --password {{password}}

# Seed database with fake data (default: 5 schools, 2 admins, 5 teachers, 20 students per school)
seed:
    cargo run --bin chalkbyte-cli -- seed

# Seed database with custom values
seed-custom schools admins teachers students:
    cargo run --bin chalkbyte-cli -- seed -s {{schools}} --admins {{admins}} --teachers {{teachers}} --students {{students}}

# Seed database with minimal data for quick testing
seed-minimal:
    cargo run --bin chalkbyte-cli -- seed -s 2 --admins 1 --teachers 2 --students 5

# Clear all seeded data (keeps system admins)
clear-seed:
    cargo run --bin chalkbyte-cli -- clear-seed

# Build all binaries
build:
    cargo build --bins

# Build all binaries in release mode
build-release:
    cargo build --release --bins

# Build only the server
build-server:
    cargo build --bin chalkbyte

# Build only the CLI
build-cli:
    cargo build --bin chalkbyte-cli

# Run tests
test:
    cargo test

# Run tests with output
test-verbose:
    cargo test -- --nocapture

# Run integration tests only
test-integration:
    cargo test integration_

# Run unit tests only
test-unit:
    cargo test unit_

# Run authentication tests
test-auth:
    cargo test integration_auth

# Run user tests
test-users:
    cargo test integration_users

# Run school tests
test-schools:
    cargo test integration_schools

# Setup test database
test-db-setup:
    psql -U postgres -c "CREATE DATABASE chalkbyte_test;" || true
    DATABASE_URL=postgresql://postgres:postgres@localhost:5432/chalkbyte_test sqlx migrate run

# Clean test database
test-db-clean:
    psql -U postgres -c "DROP DATABASE IF EXISTS chalkbyte_test;"

# Reset test database
test-db-reset: test-db-clean test-db-setup

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
