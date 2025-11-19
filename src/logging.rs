use axum::{
    extract::{MatchedPath, Request},
    middleware::Next,
    response::Response,
};
use std::time::Instant;
use tracing::{error, info, warn};

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

pub fn init_tracing() {
    use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

    if std::env::var("RUST_BACKTRACE").is_err() {
        unsafe {
            std::env::set_var("RUST_BACKTRACE", "1");
        }
    }

    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new(format!("{}=info,tower_http=warn", env!("CARGO_CRATE_NAME")))
    });

    tracing_subscriber::registry()
        .with(env_filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(false)
                .with_thread_ids(false)
                .with_thread_names(false)
                .with_file(true)
                .with_line_number(true)
                .compact(),
        )
        .init();
}
