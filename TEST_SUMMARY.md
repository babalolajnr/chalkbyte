# Test Summary

Comprehensive test suite for Chalkbyte API with unit and integration tests.

## Test Coverage

### Unit Tests (24 tests) ✅
- **Password Utilities** (10 tests) - Hash generation, verification, special chars, unicode
- **JWT Utilities** (14 tests) - Token creation, validation, roles, expiry

### Integration Tests (6 tests) - Requires test database
- **Authentication** (6 tests) - Login success/failure, validation, credentials

## Running Tests

```bash
# All tests
cargo test

# Unit tests only
cargo test unit_

# Integration tests (requires test DB)
cargo test integration_

# Specific test suite
cargo test --test unit_password
cargo test --test unit_jwt
cargo test --test integration_auth

# With output
cargo test -- --nocapture

# Using justfile
just test
just test-unit
just test-integration
```

## Test Database Setup

```bash
# Create test database
just test-db-setup

# Or manually
psql -U postgres -c "CREATE DATABASE chalkbyte_test;"
DATABASE_URL=postgresql://postgres:postgres@localhost:5432/chalkbyte_test sqlx migrate run

# Reset test database
just test-db-reset
```

## Test Results

✅ **Unit Tests: 24/24 passing**
- Password hashing and verification: 10/10
- JWT token operations: 14/14

⚠️ **Integration Tests: 6 tests (Requires test database)**
- Authentication: 6 tests
- Note: Integration tests require a running PostgreSQL test database

## Test Structure

```
tests/
├── common/
│   └── mod.rs              # Test helpers, DB setup, cleanup
├── integration_auth.rs     # Auth endpoint tests (6 tests)
├── unit_password.rs        # Password utility tests (10 tests)
├── unit_jwt.rs            # JWT utility tests (14 tests)
└── README.md              # Detailed test documentation
```

## Key Features

- Serial test execution to prevent DB conflicts
- Unique test data generation (emails, school names)
- Automatic cleanup after tests
- Role-based authorization testing
- School data isolation verification
- Comprehensive error scenario coverage

## Dependencies

```toml
[dev-dependencies]
reqwest = "0.12"          # HTTP client
tokio-test = "0.4"        # Tokio test utilities
serial_test = "3.2"       # Serial test execution
tower = "0.5"             # Tower service utilities
hyper = "1.6"             # HTTP implementation
http-body-util = "0.1"    # HTTP body utilities
```

## CI/CD Integration

```bash
# Format check, lint, and test
just ci

# Or manually
cargo fmt -- --check
cargo clippy -- -D warnings
cargo test
```
