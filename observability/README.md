# Chalkbyte API Observability Stack

This document describes the observability setup for the Chalkbyte API, including tracing, logging, and metrics collection.

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              Chalkbyte API                                  │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐        │
│  │   Tracing   │  │   Logging   │  │   Metrics   │  │   Spans     │        │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘        │
└─────────┼────────────────┼────────────────┼────────────────┼────────────────┘
          │                │                │                │
          ▼                ▼                ▼                ▼
┌─────────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────────┐
│  OTLP Collector │ │   Promtail  │ │  Prometheus │ │  Tempo          │
└────────┬────────┘ └──────┬──────┘ └──────┬──────┘ └────────┬────────┘
         │                 │               │                 │
         │                 ▼               │                 │
         │          ┌─────────────┐        │                 │
         │          │    Loki     │        │                 │
         │          └──────┬──────┘        │                 │
         │                 │               │                 │
         └─────────────────┴───────────────┴─────────────────┘
                                   │
                                   ▼
                          ┌─────────────────┐
                          │     Grafana     │
                          └─────────────────┘
```

## Components

### OpenTelemetry Collector
Receives trace data from the application via OTLP protocol and exports to Tempo.

**Configuration:** `otel-collector-config.yaml`

### Tempo
Distributed tracing backend that stores and queries traces.

**Configuration:** `tempo.yaml`

### Promtail
Log collector that reads log files and pushes them to Loki.

**Configuration:** `promtail.yaml`

### Loki
Log aggregation system designed for storing and querying logs.

**Configuration:** `loki.yaml`

### Prometheus
Metrics collection and storage system.

**Configuration:** `prometheus.yml.template`

### Grafana
Visualization platform for metrics, logs, and traces.

**Dashboards:** `grafana/dashboards/`
**Datasources:** `grafana/provisioning/datasources/`

## Environment Variables

Configure the following environment variables for the Chalkbyte API:

```bash
# OpenTelemetry Configuration
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
OTEL_SERVICE_NAME=chalkbyte-api
OTEL_TRACES_SAMPLER=always_on  # Options: always_on, always_off, trace_id_ratio
OTEL_TRACES_SAMPLER_ARG=1.0    # For trace_id_ratio: sampling rate (0.0-1.0)

# Logging Configuration
LOG_LEVEL=info                  # Options: trace, debug, info, warn, error
LOG_DIR=storage/logs            # Directory for log files
ENVIRONMENT=development         # Environment name for resource attributes

# Metrics Configuration
METRICS_PORT=3001               # Port for Prometheus metrics endpoint
```

## Log Files

The application writes to three log outputs:

1. **Console** - Compact human-readable format
2. **Plain text file** - `storage/logs/chalkbyte.log`
3. **JSON file** - `storage/logs/chalkbyte-json.log` (for Loki ingestion)

## Tracing Implementation

### Span Hierarchy

```
http_request (root span)
├── auth.middleware
├── service.operation
│   ├── db.query (SELECT)
│   ├── db.query (INSERT)
│   └── external.call (email-service)
└── response
```

### Instrumentation Patterns

#### Controller/Handler Level
```rust
#[instrument(skip(state, auth_user), fields(
    user.id = %auth_user.0.sub,
    user.role = %auth_user.0.role,
    resource.id = %id
))]
pub async fn get_resource(...) -> Result<Json<Resource>, AppError> {
    debug!("Fetching resource");
    // ... implementation
    info!(resource.id = %id, "Resource fetched successfully");
    Ok(Json(resource))
}
```

#### Service Level
```rust
#[instrument(skip(db), fields(
    db.operation = "SELECT",
    db.table = "resources"
))]
pub async fn get_resource_by_id(db: &PgPool, id: Uuid) -> Result<Resource, AppError> {
    debug!(resource.id = %id, "Querying database");
    // ... implementation
}
```

### Span Attributes (Semantic Conventions)

| Attribute | Description | Example |
|-----------|-------------|---------|
| `http.method` | HTTP method | GET, POST |
| `http.route` | Matched route pattern | /api/users/{id} |
| `http.status_code` | Response status code | 200, 404, 500 |
| `http.client_ip` | Client IP address | 192.168.1.1 |
| `user.id` | Authenticated user ID | UUID |
| `user.role` | User's role | admin, teacher |
| `db.system` | Database type | postgresql |
| `db.operation` | SQL operation | SELECT, INSERT |
| `db.sql.table` | Table name | users, schools |
| `auth.event` | Authentication event | login_attempt, logout |
| `otel.kind` | Span kind | server, client, internal |
| `otel.status_code` | Operation status | OK, ERROR |
| `error.message` | Error description | User not found |

### Request ID Correlation

Every HTTP request receives a unique request ID:
- Extracted from `X-Request-Id` header if present
- Generated as UUID if not provided
- Included in all log entries for the request
- Can be used to correlate logs across services

### Trace ID Propagation

Trace IDs are:
- Generated by the OpenTelemetry SDK
- Propagated via W3C Trace Context headers
- Logged with each request for correlation
- Available in Tempo for distributed tracing

## Metrics

### Exposed Metrics

Available at `http://localhost:3001/metrics`:

| Metric | Type | Description |
|--------|------|-------------|
| `http_requests_total` | Counter | Total HTTP requests by method, path, status |
| `http_request_duration_seconds` | Histogram | Request latency distribution |
| `users_created_total` | Counter | Users created by role |
| `schools_created_total` | Counter | Schools created |
| `auth_attempts_total` | Counter | Login attempts by result |
| `mfa_verifications_total` | Counter | MFA verification attempts |

## Grafana Dashboards

### API Overview Dashboard
- Request rate and latency
- Error rate by endpoint
- Active users
- Database query performance

### Authentication Dashboard
- Login success/failure rate
- MFA adoption and usage
- Token refresh patterns
- Security events

### Infrastructure Dashboard
- Resource utilization
- Database connections
- Memory and CPU usage

## Running the Stack

### Start all services:
```bash
docker-compose -f docker-compose.observability.yml up -d
```

### Access points:
- **Grafana**: http://localhost:3000 (admin/admin)
- **Prometheus**: http://localhost:9090
- **Tempo**: http://localhost:3200
- **Loki**: http://localhost:3100

## Querying

### Loki (LogQL)
```logql
# All error logs
{job="chalkbyte"} |= "error"

# Logs for a specific request ID
{job="chalkbyte"} | json | request_id="abc-123"

# Authentication events
{job="chalkbyte"} | json | auth_event != ""

# Slow requests (>1s)
{job="chalkbyte"} | json | latency_ms > 1000
```

### Tempo (TraceQL)
```
# Find traces with errors
{ status = error }

# Find slow database queries
{ span.db.system = "postgresql" && duration > 100ms }

# Find authentication traces
{ span.auth.event != "" }

# Find traces for a specific user
{ resource.service.name = "chalkbyte-api" && span.user.id = "uuid-here" }
```

### Prometheus (PromQL)
```promql
# Request rate
rate(http_requests_total[5m])

# 95th percentile latency
histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m]))

# Error rate
rate(http_requests_total{status=~"5.."}[5m]) / rate(http_requests_total[5m])
```

## Troubleshooting

### No traces appearing in Tempo
1. Check OTLP endpoint is reachable
2. Verify `OTEL_EXPORTER_OTLP_ENDPOINT` environment variable
3. Check OpenTelemetry collector logs

### No logs in Loki
1. Verify Promtail can access log files
2. Check Promtail configuration paths
3. Verify JSON log format is correct

### Missing metrics
1. Ensure metrics server is running on configured port
2. Check Prometheus scrape configuration
3. Verify network connectivity

## Security Considerations

1. **Metrics endpoint** should not be publicly exposed
2. **Log files** may contain sensitive information - restrict access
3. **Trace data** can reveal system internals - secure Tempo/Grafana
4. **PII in logs** - avoid logging passwords, tokens, or personal data
5. **Sampling** - use trace_id_ratio in production to reduce data volume

## Best Practices

1. **Use structured logging** - Always use key-value pairs
2. **Include context** - Add user IDs, request IDs, resource IDs
3. **Log at appropriate levels** - debug for development, info for production
4. **Avoid sensitive data** - Never log passwords or tokens
5. **Use spans for operations** - Create child spans for significant operations
6. **Record outcomes** - Log both success and failure cases
7. **Add business context** - Include domain-specific attributes