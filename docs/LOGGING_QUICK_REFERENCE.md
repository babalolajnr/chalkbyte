# Logging Quick Reference

## Environment Variables

```bash
# Default (recommended for development)
RUST_LOG=chalkbyte=info,tower_http=warn

# Debug mode
RUST_LOG=chalkbyte=debug

# Production mode
RUST_LOG=chalkbyte=warn

# Trace everything (very verbose)
RUST_LOG=trace
```

## Log Levels by HTTP Status

| Status Code | Log Level | Description |
|-------------|-----------|-------------|
| 200-299     | INFO      | Success |
| 400-499     | WARN      | Client error |
| 500-599     | ERROR     | Server error |

## Request Log Format

```
timestamp level request_id=<uuid> method=<METHOD> path=<PATH> status=<CODE> latency_ms=<MS> message
```

## Usage in Code

### Import Statements

```rust
use tracing::{info, warn, error, debug, trace, instrument};
```

### Service Functions

```rust
#[instrument(skip(db, password))]
pub async fn login(db: &PgPool, email: String, password: String) -> Result<User, AppError> {
    info!("User login attempt");
    // implementation
}
```

### Manual Logging

```rust
// Info - normal operations
info!(user_id = %user.id, "User logged in");

// Warn - warning conditions
warn!(attempt = attempts, "Multiple login failures");

// Error - error conditions
error!(error = %e, "Database connection failed");

// Debug - debugging info (debug mode only)
debug!(query = %sql, "Executing query");

// Trace - detailed tracing (trace mode only)
trace!(bytes = data.len(), "Data received");
```

## Error Responses

### Client Errors (4xx) - Detailed

```json
{
  "error": "User not found"
}
```

### Server Errors (5xx) - Generic

```json
{
  "error": "Internal server error"
}
```

Server logs contain full error details.

## Common Patterns

### Structured Fields

```rust
info!(
    user_id = %user.id,
    role = %user.role,
    school_id = ?user.school_id,
    "User created"
);
```

### Error with Context

```rust
error!(
    error = %e,
    user_id = %id,
    "Failed to update user"
);
```

### Skip Sensitive Data

```rust
#[instrument(skip(db, password, api_key))]
pub async fn authenticate(
    db: &PgPool,
    email: String,
    password: String,
    api_key: String
) -> Result<Token, AppError> {
    // password and api_key won't be logged
}
```

## Field Formatting

| Format | Usage | Example |
|--------|-------|---------|
| `%` | Display | `user_id = %id` |
| `?` | Debug | `headers = ?req.headers()` |
| `=` | Default | `count = items.len()` |

## Testing Logs

```bash
# Start server
cargo run

# In another terminal, make requests
curl http://localhost:3000/api/users

# Watch logs with filtering
cargo run 2>&1 | grep "ERROR"
cargo run 2>&1 | grep "request_id=abc123"
```

## Production Checklist

- [ ] Set `RUST_LOG=chalkbyte=warn`
- [ ] Configure log aggregation (ELK, CloudWatch, etc.)
- [ ] Set up alerts for ERROR logs
- [ ] Monitor latency trends
- [ ] Review error patterns weekly
- [ ] Never log passwords or tokens
- [ ] Use request_id for correlation

## Troubleshooting

| Problem | Solution |
|---------|----------|
| Too many logs | `RUST_LOG=chalkbyte=warn` |
| Missing logs | `RUST_LOG=chalkbyte=debug` |
| Need SQL queries | `RUST_LOG=chalkbyte=debug,sqlx=debug` |
| HTTP details | `RUST_LOG=chalkbyte=debug,tower_http=debug` |

## Example Log Output

```
2025-11-19T10:15:32Z  INFO request_id=abc123 method=POST path=/api/users Incoming request
2025-11-19T10:15:32Z  INFO request_id=abc123 method=POST path=/api/users status=201 latency_ms=45 Request completed
```

## Best Practices

✅ **DO**
- Use structured logging with key-value pairs
- Log important business events
- Use appropriate log levels
- Skip sensitive data in instrumentation
- Include context (user_id, request_id, etc.)

❌ **DON'T**
- Don't log passwords or API keys
- Don't log in tight loops
- Don't use `println!` or `eprintln!`
- Don't log full request/response bodies in production
- Don't duplicate information

## Security

**Never log:**
- Passwords
- API keys
- Tokens
- Credit card numbers
- Social security numbers
- Private health information
- Encryption keys

**Always:**
- Use generic error messages for 5xx responses
- Log actual errors server-side
- Sanitize user input in logs
- Review logs for PII before sharing