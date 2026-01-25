# Observability Configuration Guide

Chalkbyte API provides configurable observability features including tracing, logging, and metrics collection. **Observability is opt-in and disabled by default** to provide a lean, fast binary with zero overhead.

## Overview

The observability system in Chalkbyte is split into:
- **Compile-time configuration**: Control whether observability code is included in the binary (via Cargo features)
- **Runtime configuration**: Control whether observability is active when the binary runs (via environment variables)

This dual approach provides:
- **Lean binary by default**: ~15% smaller without observability (~38MB vs ~45MB)
- **Zero overhead by default**: No observability penalty unless explicitly enabled
- **Flexibility**: Enable observability when needed without rebuilding
- **Better for edge/embedded**: Minimal footprint for resource-constrained deployments

## Compile-Time Configuration

### Feature Flags

Observability is controlled through Cargo features. **It is opt-in (disabled by default)**:

```toml
# Default features (observability DISABLED)
default = []

# Feature flags
[features]
observability = ["chalkbyte-observability"]  # Enable observability
no-observability = []                        # Explicitly disable (redundant, but available)
```

### Default Build (Observability Disabled)

```bash
# Build without observability (default)
cargo build

# Binary size: ~38MB (debug)
# Runtime overhead: Zero
# Includes: Core API only, no metrics/tracing infrastructure
# /metrics endpoint: NOT available
```

### Building with Observability Enabled

```bash
# Build WITH observability enabled
cargo build --features observability

# Binary size: ~45MB (debug) - ~15% larger
# Runtime overhead: 1-5% per request
# Includes: OpenTelemetry, metrics exporter, tracing
# /metrics endpoint: Available (Prometheus format)
```

### Explicitly Disabling Observability

```bash
# Both of these produce the same lean binary (no observability)
cargo build
cargo build --features no-observability
```

## Runtime Configuration

When observability code is included (compiled with `--features observability`), you can toggle it at runtime:

### Environment Variables

```bash
# Enable observability at runtime (when feature is compiled)
OBSERVABILITY_ENABLED=true cargo run

# Disable observability at runtime (when feature is compiled)
# Note: Still has metric function call overhead, but no collection
OBSERVABILITY_ENABLED=false cargo run

# Default behavior (no environment variable)
# When feature enabled: OBSERVABILITY_ENABLED defaults to true
# When feature disabled: No observability at all (zero overhead)
cargo run
```

### Behavior

**When compiled WITHOUT observability (default):**
- No observability code in binary
- Zero runtime overhead
- No metrics collection possible
- No tracing/distributed logging
- `/metrics` endpoint not available
- Startup shows warning with enable instructions

**When compiled WITH observability and OBSERVABILITY_ENABLED=true:**
- All tracing spans are created and recorded
- Metrics are collected and exposed on the metrics endpoint
- Structured logging is active
- `/metrics` endpoint responds with Prometheus metrics

**When compiled WITH observability but OBSERVABILITY_ENABLED=false:**
- Tracing middleware is replaced with stub implementations
- Metric collection calls become no-ops
- Structured logging still works
- `/metrics` endpoint returns empty response
- Startup shows message about observability being disabled

## Startup Messages

### Default Build (No Observability)

```
⚠️  OBSERVABILITY IS DISABLED
   Observability (metrics, tracing) is not available.
   To enable, rebuild with: cargo build --features observability

🚀 Server running on http://localhost:3000
📚 Swagger UI available at http://localhost:3000/swagger-ui
📖 Scalar UI available at http://localhost:3000/scalar
```

Users see this warning immediately (on stderr), making it clear observability is not available.

### With Observability Enabled

```
🚀 Server running on http://localhost:3000
📚 Swagger UI available at http://localhost:3000/swagger-ui
📖 Scalar UI available at http://localhost:3000/scalar
📊 Metrics server running on http://localhost:3001/metrics
```

## What Gets Removed/Disabled

### Compile-Time Removal (Default Build)

The following are completely removed from the binary:

1. **Observability Crate**: `chalkbyte-observability` is not included
2. **Metric Types**: `metrics::*()` functions are not available
3. **Tracing Infrastructure**: OpenTelemetry setup is not compiled
4. **Metrics Endpoint**: `/metrics` HTTP endpoint is not registered
5. **Prometheus Exporter**: Metrics export functionality is not built
6. **OpenTelemetry Dependencies**: Not linked (saves ~7MB)

### Runtime Disabling (OBSERVABILITY_ENABLED=false)

With the feature enabled but runtime flag disabled:

1. **Tracing Spans**: Not recorded (stub implementation)
2. **Metric Collection**: No-ops (counters/gauges not incremented)
3. **Metrics Endpoint**: Returns empty response (HTTP 200 with empty body)
4. **Structured Logging**: Still active (unaffected by observability flag)

## Code Examples

### How Metrics Calls are Guarded

In service files, metric calls are wrapped with feature gates:

```rust
#[cfg(feature = "observability")]
use chalkbyte_observability::metrics;

// In service methods:
#[cfg(feature = "observability")]
metrics::track_user_created(&role_name);

// Or in authentication:
#[cfg(feature = "observability")]
metrics::track_jwt_issued();
```

When compiled WITHOUT observability (default):
- The `#[cfg(feature = "observability")]` guard prevents the code from being compiled
- No runtime overhead from the metric call
- Binary is smaller

When compiled with features but `OBSERVABILITY_ENABLED=false`:
- Code is compiled but calls metric functions
- Metric functions are stub implementations that do nothing
- Minimal overhead from function call dispatch

### Middleware Configuration

In `router.rs`, middleware is conditionally added:

```rust
#[cfg(feature = "observability")]
{
    let observability = chalkbyte_observability::observability_middleware();
    router = router.layer(observability);
}
```

## Testing with Different Configurations

### Test Default Build (Observability Disabled)

```bash
# Compilation check
cargo check --workspace

# Build
cargo build

# Run binary
cargo run

# Expected: Startup warning about observability being disabled
```

### Test with Observability Enabled

```bash
# Compilation check
cargo check --features observability

# Build with observability
cargo build --features observability

# Run with observability
cargo run --features observability

# Expected: Metrics server running on port 3001
```

### Feature Flag Combinations

```bash
# Default (observability disabled)
cargo check

# Explicit disable
cargo check --features no-observability

# Enable observability
cargo check --features observability

# Test observability enabled but disabled at runtime
OBSERVABILITY_ENABLED=false cargo run --features observability
```

## Docker Deployment

### Lean Deployment (Default - No Observability)

```bash
# Build image without observability
docker build -t chalkbyte:latest .

# Run without observability services
docker-compose up

# Expected:
# - App runs on port 3000
# - No observability overhead
# - Startup shows observability disabled warning
# - Smaller image size
```

### Full Observability Deployment

For full observability with Grafana, Prometheus, and traces:

1. **Rebuild image WITH observability** (requires `Dockerfile` modification):
   ```dockerfile
   # In Dockerfile, build with features
   RUN cargo build --release --features observability
   ```

2. **Run with observability profile**:
   ```bash
   docker-compose --profile observability up
   ```

3. **Access observability stack**:
   - Grafana: http://localhost:3001 (admin/admin123)
   - Prometheus: http://localhost:9090
   - Tempo (traces): Configured in Grafana
   - Loki (logs): Configured in Grafana

## Performance Characteristics

### Default Build (No Observability)

- Binary size: ~38MB (debug)
- Startup time: ~500ms
- Runtime overhead: 0% (no observability code present)
- Memory overhead: Minimal (no observability infrastructure)
- Suitable for: Edge, embedded, lightweight deployments

### With Observability Enabled

**When OBSERVABILITY_ENABLED=true:**
- Binary size: ~45MB (debug)
- Startup time: ~1000ms (initialization overhead)
- Runtime overhead: ~1-5% per request (span creation, metric recording)
- Memory overhead: ~20-50MB (metrics storage, tracer buffer)
- Metrics endpoint: O(n) where n = number of metric series
- Suitable for: Production monitoring, debugging, performance analysis

**When OBSERVABILITY_ENABLED=false (but compiled):**
- Binary size: ~45MB (debug)
- Startup time: ~500ms (no initialization)
- Runtime overhead: <1% (function call dispatch only)
- Memory overhead: Minimal (no active collection)
- Useful for: Testing observability toggle without rebuild

## Troubleshooting

### Observability Disabled Warning on Startup

**Problem**: Application shows warning about observability being disabled.

**Solution**: This is expected behavior by default.
- If you want observability: `cargo build --features observability && cargo run`
- If you don't want observability: No action needed, this is the default

### Metrics Endpoint Returns Empty

**Problem**: `/metrics` returns empty response or 404.

**Solution**: Observability is opt-in.
1. Check if built with observability: `cargo build --features observability`
2. Verify `OBSERVABILITY_ENABLED` is not set to `false`
3. Rebuild and run: `cargo run --features observability`

### Tracing Not Appearing in Logs

**Problem**: Tracing spans not in logs.

**Solution**:
- Confirm observability feature is enabled: `cargo build --features observability`
- Check `RUST_LOG` environment variable: `RUST_LOG=debug cargo run --features observability`
- Verify `OBSERVABILITY_ENABLED=true` (or not set)

### Build Size Larger Than Expected

**Problem**: Binary is larger than expected (~45MB instead of ~38MB).

**Solution**:
1. Check which build you used: `cargo build --features observability` includes observability
2. Use `cargo build` (default) for lean binary: `cargo build && ls -lh target/debug/chalkbyte`
3. Check that dependencies aren't duplicated
4. Use `cargo build --release` for optimized builds

### `/metrics` Endpoint Not Available

**Problem**: Getting 404 when accessing `/metrics`.

**Solution**:
- `/metrics` endpoint is only available when observability is enabled
- Build with: `cargo build --features observability`
- Run with: `cargo run --features observability`
- Check that you're using the correct binary (not the default one)

## Architecture Details

The observability system is implemented in:

```
crates/chalkbyte-observability/
├── src/
│   ├── lib.rs              # Feature gate exports
│   ├── logging.rs          # Tracing setup
│   ├── metrics.rs          # Metrics collection
│   └── tracing_utils.rs    # Tracing helpers
```

When `#[cfg(feature = "observability")]` is false:
- Module is not compiled
- All imports/usage are guarded with `#[cfg(...)]`
- Stub middleware is provided via `middleware/observability_stubs.rs`
- Zero runtime impact

## Summary: Default Behavior

| Feature | Default Build | With `--features observability` |
|---------|---------------|--------------------------------|
| Binary size | ~38MB | ~45MB |
| Runtime overhead | 0% | 1-5% |
| Metrics available | ❌ No | ✅ Yes |
| Tracing available | ❌ No | ✅ Yes |
| `/metrics` endpoint | ❌ No | ✅ Yes |
| Startup message | ⚠️ Warning | ℹ️ Info |

**Default is lean and fast. Enable observability when you need it.**

## Related Documentation

- [Caching Guide](./CACHING.md) - Redis caching configuration
- [Roles & Permissions](./ROLES_PERMISSIONS_API.md) - Authorization system
- [Permission Based Access](./PERMISSION_BASED_ACCESS.md) - Access control details
