# Chalkbyte Tests

This directory contains comprehensive tests for the Chalkbyte API.

## Test Structure

```
tests/
├── common/                     # Shared test utilities
│   └── mod.rs                 # Test helpers and setup functions
├── integration_auth.rs        # Authentication endpoint tests (6 tests)
├── integration_mfa.rs         # MFA endpoint tests
├── integration_roles.rs       # Roles & permissions endpoint tests (26 tests)
├── integration_schools.rs     # Schools endpoint tests
├── integration_students.rs    # Students endpoint tests
├── integration_users.rs       # Users endpoint tests
└── integration_levels.rs      # Levels endpoint tests (18 tests)

Note: All unit tests are located in their respective source files using `#[cfg(test)]` modules:
- `src/utils/jwt.rs` - 20 JWT tests
- `src/utils/password.rs` - 10 password tests
- `src/middleware/role.rs` - 16 role middleware tests
- `src/modules/levels/service.rs` - 24 levels service tests
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
cargo test integration_levels
cargo test integration_roles

# Unit tests (in source files)
cargo test --lib jwt::tests
cargo test --lib password::tests
cargo test --lib middleware::role::tests
cargo test --lib levels::service::tests
```

### Run Single Test

```bash
cargo test test_login_success
```

### Run Tests with Output

```bash
cargo test -- --nocapture
```

### Rate Limiting in Tests

Tests automatically disable rate limiting through Rust's `#[cfg(test)]` conditional compilation.
The rate limiter uses `PeerIpKeyExtractor` which requires socket connection info that isn't available 
in test environments using `tower::ServiceExt::oneshot`.

```bash
# Run integration tests normally - rate limiting is disabled automatically
cargo test --test integration_roles -- --test-threads=1
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

### Unit Tests (in source files with `#[cfg(test)]`)

- **src/utils/password.rs** (10 tests): Password hashing, verification, special characters, unicode, case sensitivity
- **src/utils/jwt.rs** (20 tests): JWT creation, validation, expiry, role encoding, token security, MFA tokens, refresh tokens
- **src/middleware/role.rs** (16 tests): Role-based access control middleware, role hierarchy, role checking functions
- **src/modules/levels/service.rs** (24 tests): Level service methods, CRUD operations, student assignments, validation, error handling

### Integration Tests (requires test database)

- **integration_auth.rs** (6 tests): Login success/failure, validation, credentials, wrong password, email format
- **integration_mfa.rs**: Multi-factor authentication flows
- **integration_schools.rs**: School management endpoints
- **integration_students.rs**: Student management endpoints
- **integration_users.rs**: User management endpoints
- **integration_levels.rs** (18 tests): Level CRUD, student assignments, authorization, school isolation

## Key Test Patterns

### SQLx Test Framework

Integration and unit tests use `#[sqlx::test]` for automatic database setup:

```rust
#[sqlx::test(migrations = "./migrations")]
async fn test_name(pool: PgPool) {
    // Test code with automatic DB setup and cleanup
}
```

Tests should run with `--test-threads=1` for stability:

```bash
cargo test --test integration_levels -- --test-threads=1
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

## Test Organization

All unit tests follow Rust best practices:
- **Unit tests** are embedded in source files using `#[cfg(test)]` modules
- **Integration tests** are in the `tests/` directory and test the full HTTP stack
- This organization keeps tests close to the code they test and follows idiomatic Rust conventions

## Coverage

Tests cover:
- ✅ Authentication (login, token generation) - 6 integration tests
- ✅ Password hashing and verification - 10 unit tests
- ✅ JWT token creation and validation - 20 unit tests (includes MFA and refresh tokens)
- ✅ Role middleware and authorization - 16 unit tests
- ✅ Levels module (CRUD, assignments, authorization) - 42 tests (18 integration + 24 unit)
- ✅ Roles & permissions module (CRUD, assignments, authorization) - 26 integration tests
- ✅ MFA flows and verification
- ✅ Schools management
- ✅ Students management
- ✅ Users management
- ✅ Role-based middleware
- ✅ Validation errors (email format, missing fields)
- ✅ Invalid credentials handling
- ✅ Role encoding in JWT tokens
- ✅ Password security (hashing, unicode, special chars)
- ✅ School isolation and cross-school access prevention
- ✅ Bulk operations with partial success handling

## Detailed Test Documentation

For comprehensive test coverage details, see:
- **docs/LEVELS_TESTS.md** - Complete documentation of all 42 levels module tests

## Notes

- All unit tests use `#[cfg(test)]` modules embedded in source files (Rust best practice)
- Tests use SQLx's `#[sqlx::test]` macro for automatic database setup/teardown (for tests that need DB)
- Each test runs in isolation with its own database state
- Test helpers in `tests/common/mod.rs` provide utility functions for integration tests
- Integration tests use the full HTTP stack via Axum's test helpers
- Unit tests call functions/methods directly without HTTP layer
- Run with `--test-threads=1` for stable database test execution
- Integration tests require PostgreSQL test database with migrations applied
- Rate limiting is automatically disabled during test runs via `#[cfg(test)]`
- Total unit tests: 70 (JWT: 20, Password: 10, Role: 16, Levels: 24)
- Total integration tests for roles: 26 (permissions, role CRUD, role-permission mapping, user-role assignments, authorization checks)