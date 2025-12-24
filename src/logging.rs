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
use tracing::{error, info, warn};
use tracing_subscriber::{EnvFilter, Layer, layer::SubscriberExt, util::SubscriberInitExt};

pub async fn logging_middleware(req: Request, next: Next) -> Response {
    let start = Instant::now();
    let method = req.method().clone();
    let uri = req.uri().clone();
    let matched_path = req
        .extensions()
        .get::<MatchedPath>()
        .map(|p| p.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string());

    let request_id = uuid::Uuid::new_v4().to_string();

    info!(
        request_id = %request_id,
        method = %method,
        path = %matched_path,
        "Incoming request"
    );

    let response = next.run(req).await;
    let latency = start.elapsed();
    let status = response.status();

    match status.as_u16() {
        200..=299 => {
            info!(
                request_id = %request_id,
                method = %method,
                path = %matched_path,
                status = %status.as_u16(),
                latency_ms = %latency.as_millis(),
                "Request completed"
            );
        }
        400..=499 => {
            warn!(
                request_id = %request_id,
                method = %method,
                path = %matched_path,
                status = %status.as_u16(),
                latency_ms = %latency.as_millis(),
                "Client error"
            );
        }
        500..=599 => {
            error!(
                request_id = %request_id,
                method = %method,
                path = %matched_path,
                status = %status.as_u16(),
                latency_ms = %latency.as_millis(),
                "Server error"
            );
        }
        _ => {
            info!(
                request_id = %request_id,
                method = %method,
                path = %matched_path,
                status = %status.as_u16(),
                latency_ms = %latency.as_millis(),
                "Request completed"
            );
        }
    }

    response
}

fn init_tracer() -> Result<Tracer, TraceError> {
    // Get OTLP endpoint from environment or use default
    let otlp_endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:4317".to_string());

    info!(
        "Initializing OpenTelemetry tracer with endpoint: {}",
        otlp_endpoint
    );

    // Set up trace context propagator for distributed tracing
    global::set_text_map_propagator(TraceContextPropagator::new());

    // Configure resource with service information
    let resource = Resource::new(vec![
        KeyValue::new(SERVICE_NAME, env!("CARGO_PKG_NAME")),
        KeyValue::new(SERVICE_VERSION, env!("CARGO_PKG_VERSION")),
        KeyValue::new(
            "environment",
            std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string()),
        ),
    ]);

    // Build OTLP exporter
    let otlp_exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint(otlp_endpoint);

    // Build and install tracer provider, returning the tracer
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(otlp_exporter)
        .with_trace_config(
            opentelemetry_sdk::trace::Config::default()
                .with_sampler(Sampler::AlwaysOn)
                .with_id_generator(RandomIdGenerator::default())
                .with_resource(resource),
        )
        .install_batch(runtime::Tokio)?;

    Ok(tracer)
}

pub fn init_tracing() {
    use std::fs;
    use tracing_appender::rolling::{RollingFileAppender, Rotation};
    use tracing_subscriber::fmt;

    let log_dir = "storage/logs";
    fs::create_dir_all(log_dir).expect("Failed to create logs directory");

    // Console layer with filtering
    let console_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new(format!(
            "{}=info,tower_http=warn,hyper=info,tonic=info",
            env!("CARGO_CRATE_NAME")
        ))
    });

    let console_layer = fmt::layer()
        .with_target(false)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_file(true)
        .with_line_number(true)
        .compact()
        .with_filter(console_filter);

    // File layer for errors
    let file_appender = RollingFileAppender::new(Rotation::DAILY, log_dir, "chalkbyte.log");

    let file_layer = fmt::layer()
        .with_writer(file_appender)
        .with_target(false)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_file(true)
        .with_line_number(true)
        .with_ansi(false)
        .with_filter(EnvFilter::new("error"));

    // JSON file layer for structured logs (can be ingested by Loki)
    let json_appender = RollingFileAppender::new(Rotation::DAILY, log_dir, "chalkbyte.json");

    let json_layer = fmt::layer()
        .json()
        .with_writer(json_appender)
        .with_current_span(true)
        .with_span_list(true)
        .with_filter(EnvFilter::new("info"));

    // Try to initialize OpenTelemetry tracer
    match init_tracer() {
        Ok(tracer) => {
            info!("OpenTelemetry tracer initialized successfully");

            // OpenTelemetry layer
            let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

            // Build the subscriber with OpenTelemetry
            tracing_subscriber::registry()
                .with(console_layer)
                .with(file_layer)
                .with(json_layer)
                .with(otel_layer)
                .init();

            info!("Tracing initialized with OpenTelemetry and file logging");
        }
        Err(e) => {
            // If OpenTelemetry fails to initialize, continue without it
            eprintln!(
                "⚠️  Failed to initialize OpenTelemetry: {}. Continuing without tracing...",
                e
            );

            // Build the subscriber without OpenTelemetry
            tracing_subscriber::registry()
                .with(console_layer)
                .with(file_layer)
                .with(json_layer)
                .init();

            warn!("Tracing initialized without OpenTelemetry (file logging only)");
        }
    }
}

pub async fn shutdown_tracer() {
    info!("Shutting down OpenTelemetry tracer...");

    // Shutdown the global tracer provider
    global::shutdown_tracer_provider();

    info!("OpenTelemetry tracer shutdown complete");
}
