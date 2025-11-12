# Chalkbyte Tests

This directory contains comprehensive tests for the Chalkbyte API.

## Test Structure

```
tests/
├── common/                  # Shared test utilities
│   └── mod.rs              # Test helpers and setup functions
├── integration_auth.rs     # Authentication endpoint tests (6 tests)
├── unit_password.rs        # Password hashing/verification tests (10 tests)
└── unit_jwt.rs            # JWT token generation/validation tests (14 tests)
```

## Running Tests

### Prerequisites

1. PostgreSQL test database running
2. Environment variables set (use `.env.test`)

### Run All Tests

```bash
cargo test
```

### Run Specific Test Suite

```bash
# Integration tests (requires test database)
cargo test integration_auth

# Unit tests
cargo test unit_password
cargo test unit_jwt
```

### Run Single Test

```bash
cargo test test_login_success
```

### Run Tests with Output

```bash
cargo test -- --nocapture
```

## Test Database Setup

Create a separate test database:

```sql
CREATE DATABASE chalkbyte_test;
```

Run migrations on test database:

```bash
DATABASE_URL=postgresql://postgres:postgres@localhost:5432/chalkbyte_test sqlx migrate run
```

## Test Configuration

Tests use `.env.test` for configuration. Never use production credentials.

## Test Categories

### Unit Tests (24 tests total)

- **unit_password.rs** (10 tests): Password hashing, verification, special characters, unicode, case sensitivity
- **unit_jwt.rs** (14 tests): JWT creation, validation, expiry, role encoding, token security

### Integration Tests (6 tests total - requires test database)

- **integration_auth.rs** (6 tests): Login success/failure, validation, credentials, wrong password, email format

## Key Test Patterns

### Serial Execution

Tests use `#[serial]` to prevent database conflicts:

```rust
#[tokio::test]
#[serial]
async fn test_name() { }
```

### Test Helpers

Common module provides:
- `create_test_user()` - Create user in transaction
- `create_test_school()` - Create school in transaction
- `cleanup_test_data()` - Remove test data
- `get_test_pool()` - Get database connection
- `generate_unique_email()` - Generate unique test email
- `generate_unique_school_name()` - Generate unique school name

### Authorization Testing

Tests verify role-based access control:
- System admin can access all endpoints
- School admins see only their school data
- Students/teachers have limited access
- Unauthorized requests return 401
- Forbidden requests return 403

## Coverage

Tests cover:
- ✅ Authentication (login, token generation) - 6 integration tests
- ✅ Password hashing and verification - 10 unit tests
- ✅ JWT token creation and validation - 14 unit tests
- ✅ Validation errors (email format, missing fields)
- ✅ Invalid credentials handling
- ✅ Role encoding in JWT tokens
- ✅ Password security (hashing, unicode, special chars)

## Notes

- Tests clean up their data using transactions or cleanup functions
- Each test creates unique data to avoid conflicts
- Integration tests require PostgreSQL test database with migrations run
- Unit tests are isolated and don't require database