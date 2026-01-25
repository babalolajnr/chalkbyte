//! Observability stub middleware for when observability feature is disabled

use axum::{extract::Request, middleware::Next, response::Response};

/// Stub: Check if observability is enabled
pub fn is_observability_enabled() -> bool {
    false
}

/// Stub: No-op logging middleware
pub async fn logging_middleware(req: Request, next: Next) -> Response {
    next.run(req).await
}

/// Stub: No-op metrics middleware
pub async fn metrics_middleware(req: Request, next: Next) -> Response {
    next.run(req).await
}
