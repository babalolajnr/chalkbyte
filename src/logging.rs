use axum::{
    extract::{MatchedPath, Request},
    middleware::Next,
    response::Response,
};
use opentelemetry::{KeyValue, global, trace::TraceError};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    Resource,
    propagation::TraceContextPropagator,
    runtime,
    trace::{RandomIdGenerator, Sampler, Tracer},
};
use opentelemetry_semantic_conventions::resource::{SERVICE_NAME, SERVICE_VERSION};
use std::time::Instant;
use tracing::{Instrument, Span, error, field, info, info_span, warn};
use tracing_opentelemetry::OpenTelemetrySpanExt;
use tracing_subscriber::{EnvFilter, Layer, layer::SubscriberExt, util::SubscriberInitExt};

/// Extract trace ID from the current span context for correlation
fn get_trace_id() -> String {
    use opentelemetry::trace::TraceContextExt;
    let context = Span::current().context();
    let span_ref = context.span();
    let span_context = span_ref.span_context();
    if span_context.is_valid() {
        span_context.trace_id().to_string()
    } else {
        uuid::Uuid::new_v4().to_string()
    }
}

/// HTTP request/response logging middleware with full observability context
pub async fn logging_middleware(req: Request, next: Next) -> Response {
    let start = Instant::now();
    let method = req.method().clone();
    let uri = req.uri().clone();
    let version = format!("{:?}", req.version());

    // Extract the matched path for better route identification
    let matched_path = req
        .extensions()
        .get::<MatchedPath>()
        .map(|p| p.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string());

    // Generate request ID for correlation
    let request_id = req
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    // Extract user agent and content type
    let user_agent = req
        .headers()
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    let content_type = req
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("none")
        .to_string();

    // Extract client IP from various headers
    let client_ip = req
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .or_else(|| req.headers().get("x-real-ip").and_then(|v| v.to_str().ok()))
        .unwrap_or("unknown")
        .to_string();

    // Create a span for this request with semantic conventions
    let span = info_span!(
        "http_request",
        http.method = %method,
        http.route = %matched_path,
        http.url = %uri,
        http.version = %version,
        http.user_agent = %user_agent,
        http.client_ip = %client_ip,
        http.status_code = field::Empty,
        http.response_content_length = field::Empty,
        request.id = %request_id,
        trace.id = field::Empty,
        otel.kind = "server",
        otel.status_code = field::Empty,
        error.message = field::Empty,
        latency_ms = field::Empty,
    );

    // Execute the request within the span
    let response = async move {
        info!(
            request_id = %request_id,
            method = %method,
            path = %matched_path,
            client_ip = %client_ip,
            user_agent = %user_agent,
            content_type = %content_type,
            "Request started"
        );

        let response = next.run(req).await;
        let latency = start.elapsed();
        let status = response.status();
        let status_code = status.as_u16();

        // Get response content length if available
        let content_length = response
            .headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);

        // Get trace ID for correlation
        let trace_id = get_trace_id();

        // Record span attributes
        Span::current().record("http.status_code", status_code);
        Span::current().record("http.response_content_length", content_length);
        Span::current().record("latency_ms", latency.as_millis() as i64);
        Span::current().record("trace.id", &trace_id);

        // Set OTEL status based on HTTP status
        let otel_status = if status.is_success() {
            "OK"
        } else if status.is_client_error() {
            "ERROR"
        } else if status.is_server_error() {
            "ERROR"
        } else {
            "UNSET"
        };
        Span::current().record("otel.status_code", otel_status);

        // Log based on status code range with appropriate level
        match status_code {
            200..=299 => {
                info!(
                    request_id = %request_id,
                    trace_id = %trace_id,
                    method = %method,
                    path = %matched_path,
                    status = %status_code,
                    latency_ms = %latency.as_millis(),
                    content_length = %content_length,
                    "Request completed successfully"
                );
            }
            400..=499 => {
                warn!(
                    request_id = %request_id,
                    trace_id = %trace_id,
                    method = %method,
                    path = %matched_path,
                    status = %status_code,
                    latency_ms = %latency.as_millis(),
                    "Client error"
                );
            }
            500..=599 => {
                error!(
                    request_id = %request_id,
                    trace_id = %trace_id,
                    method = %method,
                    path = %matched_path,
                    status = %status_code,
                    latency_ms = %latency.as_millis(),
                    "Server error"
                );
                Span::current().record("error.message", "Internal server error");
            }
            _ => {
                info!(
                    request_id = %request_id,
                    trace_id = %trace_id,
                    method = %method,
                    path = %matched_path,
                    status = %status_code,
                    latency_ms = %latency.as_millis(),
                    "Request completed"
                );
            }
        }

        response
    }
    .instrument(span)
    .await;

    response
}

fn init_tracer() -> Result<Tracer, TraceError> {
    // Get OTLP endpoint from environment or use default
    let otlp_endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:4317".to_string());

    let service_name =
        std::env::var("OTEL_SERVICE_NAME").unwrap_or_else(|_| env!("CARGO_PKG_NAME").to_string());

    let environment = std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string());

    info!(
        "Initializing OpenTelemetry tracer with endpoint: {}, service: {}, environment: {}",
        otlp_endpoint, service_name, environment
    );

    // Set up trace context propagator for distributed tracing
    global::set_text_map_propagator(TraceContextPropagator::new());

    // Configure resource with service information following semantic conventions
    let resource = Resource::new(vec![
        KeyValue::new(SERVICE_NAME, service_name.clone()),
        KeyValue::new(SERVICE_VERSION, env!("CARGO_PKG_VERSION")),
        KeyValue::new("environment", environment.clone()),
        KeyValue::new("service.namespace", "chalkbyte"),
        KeyValue::new("deployment.environment", environment),
    ]);

    // Build OTLP exporter with timeout
    let otlp_exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint(otlp_endpoint)
        .with_timeout(std::time::Duration::from_secs(5));

    // Build and install tracer provider with sampling configuration
    let sampler = match std::env::var("OTEL_TRACES_SAMPLER")
        .unwrap_or_else(|_| "always_on".to_string())
        .as_str()
    {
        "always_off" => Sampler::AlwaysOff,
        "trace_id_ratio" => {
            let ratio: f64 = std::env::var("OTEL_TRACES_SAMPLER_ARG")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(1.0);
            Sampler::TraceIdRatioBased(ratio)
        }
        _ => Sampler::AlwaysOn,
    };

    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(otlp_exporter)
        .with_trace_config(
            opentelemetry_sdk::trace::Config::default()
                .with_sampler(sampler)
                .with_id_generator(RandomIdGenerator::default())
                .with_max_events_per_span(64)
                .with_max_attributes_per_span(128)
                .with_resource(resource),
        )
        .install_batch(runtime::Tokio)?;

    Ok(tracer)
}

pub fn init_tracing() {
    use std::fs::OpenOptions;
    use tracing_subscriber::fmt;

    let log_dir = std::env::var("LOG_DIR").unwrap_or_else(|_| "storage/logs".to_string());
    std::fs::create_dir_all(&log_dir).expect("Failed to create logs directory");

    // Determine log level from environment
    let log_level = std::env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string());

    // Console layer with filtering
    let console_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new(format!(
            "{}={},tower_http=warn,hyper=warn,tonic=warn,h2=warn,sqlx=warn",
            env!("CARGO_CRATE_NAME"),
            log_level
        ))
    });

    let console_layer = fmt::layer()
        .with_target(true)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_file(true)
        .with_line_number(true)
        .with_level(true)
        .compact()
        .with_filter(console_filter);

    // Plain text log file with non-blocking writer
    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(format!("{}/chalkbyte.log", log_dir))
        .expect("Failed to open log file");

    let (non_blocking_file, _guard1) = tracing_appender::non_blocking(log_file);

    let file_filter = EnvFilter::new(format!(
        "{}={},tower_http=warn,sqlx=warn",
        env!("CARGO_CRATE_NAME"),
        log_level
    ));

    let file_layer = fmt::layer()
        .with_writer(non_blocking_file)
        .with_target(true)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_file(true)
        .with_line_number(true)
        .with_ansi(false)
        .with_filter(file_filter);

    // JSON log file for Loki/Grafana with non-blocking writer
    let json_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(format!("{}/chalkbyte-json.log", log_dir))
        .expect("Failed to open JSON log file");

    let (non_blocking_json, _guard2) = tracing_appender::non_blocking(json_file);

    let json_filter = EnvFilter::new(format!(
        "{}={},tower_http=info,sqlx=warn",
        env!("CARGO_CRATE_NAME"),
        log_level
    ));

    let json_layer = fmt::layer()
        .json()
        .with_writer(non_blocking_json)
        .with_current_span(true)
        .with_span_list(true)
        .with_target(true)
        .flatten_event(true)
        .with_filter(json_filter);

    // Try to initialize OpenTelemetry tracer
    match init_tracer() {
        Ok(tracer) => {
            // OpenTelemetry layer
            let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

            // Build the subscriber with OpenTelemetry
            tracing_subscriber::registry()
                .with(console_layer)
                .with(file_layer)
                .with(json_layer)
                .with(otel_layer)
                .init();

            info!(
                service.name = env!("CARGO_PKG_NAME"),
                service.version = env!("CARGO_PKG_VERSION"),
                "Tracing initialized with OpenTelemetry and file logging"
            );

            // Keep guards alive
            std::mem::forget(_guard1);
            std::mem::forget(_guard2);
        }
        Err(e) => {
            // If OpenTelemetry fails to initialize, continue without it
            eprintln!(
                "⚠️  Failed to initialize OpenTelemetry: {}. Continuing without distributed tracing...",
                e
            );

            // Build the subscriber without OpenTelemetry
            tracing_subscriber::registry()
                .with(console_layer)
                .with(file_layer)
                .with(json_layer)
                .init();

            warn!(
                error = %e,
                "Tracing initialized without OpenTelemetry (file logging only)"
            );

            // Keep guards alive
            std::mem::forget(_guard1);
            std::mem::forget(_guard2);
        }
    }
}

pub async fn shutdown_tracer() {
    info!("Shutting down OpenTelemetry tracer...");

    // Shutdown the global tracer provider with timeout
    tokio::time::timeout(
        std::time::Duration::from_secs(5),
        tokio::task::spawn_blocking(|| {
            global::shutdown_tracer_provider();
        }),
    )
    .await
    .ok();

    info!("OpenTelemetry tracer shutdown complete");
}

/// Create a span for database operations
#[macro_export]
macro_rules! db_span {
    ($operation:expr, $table:expr) => {
        tracing::info_span!(
            "db.query",
            db.system = "postgresql",
            db.operation = $operation,
            db.sql.table = $table,
            otel.kind = "client"
        )
    };
    ($operation:expr, $table:expr, $($field:tt)*) => {
        tracing::info_span!(
            "db.query",
            db.system = "postgresql",
            db.operation = $operation,
            db.sql.table = $table,
            otel.kind = "client",
            $($field)*
        )
    };
}

/// Create a span for external service calls
#[macro_export]
macro_rules! external_span {
    ($service:expr, $operation:expr) => {
        tracing::info_span!(
            "external.call",
            service.name = $service,
            operation = $operation,
            otel.kind = "client"
        )
    };
}
