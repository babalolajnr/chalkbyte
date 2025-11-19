# Logging System

## Overview

The application uses a structured logging system built on `tracing` and `tracing-subscriber` that provides clean, production-ready logs with consistent formatting.

## Log Levels

Logs are categorized into different levels based on the HTTP response status:

- **INFO** (200-299): Successful requests
- **WARN** (400-499): Client errors (bad requests, unauthorized, forbidden, not found)
- **ERROR** (500-599): Server errors

## Log Format

Each log entry includes:

```
timestamp level fields message
```

Example:
```
2025-11-19T10:15:32.123456Z INFO request_id=a1b2c3d4 method=GET path=/api/users status=200 latency_ms=45 Request completed
```

## Configuration

### Environment Variables

Control log verbosity using the `RUST_LOG` environment variable:

```bash
# Default (info level for application, warn for tower_http)
RUST_LOG=chalkbyte=info,tower_http=warn

# Debug mode (verbose logging)
RUST_LOG=chalkbyte=debug,tower_http=debug

# Trace mode (very verbose)
RUST_LOG=chalkbyte=trace,tower_http=trace,sqlx=trace

# Production (minimal logging)
RUST_LOG=chalkbyte=warn,tower_http=warn
```

### Default Configuration

When `RUST_LOG` is not set, the application defaults to:
- Application logs: `info` level
- Tower HTTP logs: `warn` level

## Request Logging

Every HTTP request is logged with the following information:

### Incoming Request
```
request_id=<uuid> method=<HTTP_METHOD> path=<MATCHED_PATH> "Incoming request"
```

### Completed Request
```
request_id=<uuid> method=<HTTP_METHOD> path=<MATCHED_PATH> status=<STATUS_CODE> latency_ms=<DURATION> "Request completed"
```

### Client Error (4xx)
```
request_id=<uuid> method=<HTTP_METHOD> path=<MATCHED_PATH> status=<STATUS_CODE> latency_ms=<DURATION> "Client error"
```

### Server Error (5xx)
```
request_id=<uuid> method=<HTTP_METHOD> path=<MATCHED_PATH> status=<STATUS_CODE> latency_ms=<DURATION> "Server error"
```

## Error Logging

### Client Errors (4xx)

Client errors return the actual error message to help with debugging:

```json
{
  "error": "User not found"
}
```

### Server Errors (5xx)

Server errors return a generic message to avoid leaking sensitive information:

```json
{
  "error": "Internal server error"
}
```

The detailed error is logged server-side with full context:

```
ERROR status=500 error="database connection failed" error_chain=[...] Internal server error occurred
```

## Service-Level Logging

Use the `#[instrument]` attribute on service functions for automatic tracing:

```rust
use tracing::instrument;

impl UserService {
    #[instrument(skip(db))]
    pub async fn create_user(db: &PgPool, dto: CreateUserDto) -> Result<User, AppError> {
        // Function parameters are automatically logged
        // Use skip() to exclude sensitive data like passwords or database pools
    }
}
```

## Manual Logging

For custom logging within functions:

```rust
use tracing::{info, warn, error, debug, trace};

// Info: General information
info!(user_id = %user.id, "User logged in");

// Warn: Warning conditions
warn!(attempt_count = attempts, "Multiple failed login attempts");

// Error: Error conditions
error!(error = %e, "Failed to process payment");

// Debug: Debug information (only in debug mode)
debug!(query = %sql, "Executing database query");

// Trace: Very detailed tracing (only in trace mode)
trace!(bytes = data.len(), "Received data");
```

## Best Practices

### DO

✅ Use structured logging with key-value pairs
```rust
info!(user_id = %id, role = %role, "User created");
```

✅ Log important business events
```rust
info!(school_id = %school.id, "New school registered");
```

✅ Use appropriate log levels
```rust
error!("Database connection failed");  // System issues
warn!("Rate limit exceeded");          // Warning conditions
info!("User logged in");               // Normal events
```

✅ Skip sensitive data in instrumentation
```rust
#[instrument(skip(password, db))]
pub async fn authenticate(db: &PgPool, email: String, password: String) {}
```

### DON'T

❌ Don't log sensitive information
```rust
// BAD
error!("Login failed for password: {}", password);

// GOOD
error!(email = %email, "Login failed");
```

❌ Don't log in tight loops
```rust
// BAD
for item in items {
    debug!("Processing item: {:?}", item);  // Too verbose
}

// GOOD
info!(item_count = items.len(), "Processing items");
```

❌ Don't use println! or eprintln!
```rust
// BAD
println!("User created");

// GOOD
info!("User created");
```

## Correlation

Each request is assigned a unique `request_id` (UUID) that appears in all logs related to that request. Use this to trace the flow of a single request through the system.

Example:
```
INFO request_id=abc123 method=POST path=/api/users Incoming request
INFO request_id=abc123 Creating new user
INFO request_id=abc123 method=POST path=/api/users status=201 latency_ms=67 Request completed
```

## Production Recommendations

For production environments:

1. Set `RUST_LOG=chalkbyte=info,tower_http=warn`
2. Use log aggregation tools (ELK, Splunk, CloudWatch, etc.)
3. Set up alerts for ERROR level logs
4. Monitor latency metrics from request logs
5. Regularly review server error logs for patterns

## Troubleshooting

### Too Many Logs

Reduce log verbosity:
```bash
RUST_LOG=chalkbyte=warn
```

### Missing Logs

Increase log verbosity:
```bash
RUST_LOG=chalkbyte=debug
```

### Database Query Logging

Enable SQLx query logging:
```bash
RUST_LOG=chalkbyte=debug,sqlx=debug
```

### Request/Response Body Logging

For debugging, you can temporarily add logging in controllers:
```rust
debug!(body = ?dto, "Request body received");
```

Remember to remove or gate these behind debug builds to avoid logging sensitive data in production.