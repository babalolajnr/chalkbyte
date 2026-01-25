//! Chalkbyte Observability Module
//!
//! Provides configurable observability features including:
//! - Tracing and distributed tracing via OpenTelemetry
//! - Metrics collection via Prometheus
//! - HTTP request/response logging
//!
//! This module can be enabled or disabled at compile time via the `observability` feature flag.
//! At runtime, observability can be further controlled via the `OBSERVABILITY_ENABLED` environment variable.
//!
//! # Features
//!
//! - `observability` (default): Enables all observability features including tracing, logging, and metrics
//!
//! # Examples
//!
//! ```no_run
//! use chalkbyte_observability::{init_tracing, shutdown_tracer};
//!
//! #[tokio::main]
//! async fn main() {
//!     init_tracing();
//!     // ... application code ...
//!     shutdown_tracer().await;
//! }
//! ```

#![allow(dead_code)]

#[cfg(feature = "observability")]
pub mod logging;
#[cfg(feature = "observability")]
pub mod metrics;
#[cfg(feature = "observability")]
pub mod tracing_utils;

// Re-export PrometheusHandle type when observability is enabled
#[cfg(feature = "observability")]
pub use metrics_exporter_prometheus::PrometheusHandle;

// Public exports when observability is enabled
#[cfg(feature = "observability")]
pub use logging::{is_observability_enabled as is_logging_enabled, logging_middleware, init_tracing, shutdown_tracer};
#[cfg(feature = "observability")]
pub use metrics::{is_observability_enabled as is_metrics_enabled, metrics_middleware, init_metrics, track_user_created, track_user_login_success, track_user_login_failure, track_jwt_issued, track_school_created};

// Common re-exports when observability is enabled
#[cfg(feature = "observability")]
pub use logging::is_observability_enabled;

// No-op stubs when observability is disabled
#[cfg(not(feature = "observability"))]
pub mod stubs {
    use axum::{extract::Request, middleware::Next, response::Response};

    /// No-op observability check when feature disabled
    pub fn is_observability_enabled() -> bool {
        false
    }

    /// No-op logging middleware when feature disabled
    pub async fn logging_middleware(req: Request, next: Next) -> Response {
        next.run(req).await
    }

    /// No-op metrics middleware when feature disabled
    pub async fn metrics_middleware(req: Request, next: Next) -> Response {
        next.run(req).await
    }

    /// No-op tracing initialization when feature disabled
    pub fn init_tracing() {}

    /// No-op tracer shutdown when feature disabled
    pub async fn shutdown_tracer() {}

    /// No-op metrics initialization when feature disabled
    pub fn init_metrics() -> Option<()> {
        None
    }

    // No-op tracking functions
    pub fn track_user_created(_role: &str) {}
    pub fn track_user_login_success(_role: &str) {}
    pub fn track_user_login_failure(_reason: &str) {}
    pub fn track_jwt_issued() {}
    pub fn track_school_created() {}
}

#[cfg(not(feature = "observability"))]
pub use stubs::*;
