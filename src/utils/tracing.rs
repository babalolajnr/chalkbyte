//! Tracing utilities and helper macros for observability
//!
//! This module provides utilities for creating consistent spans and logging
//! across the application. It includes macros for database operations, external
//! service calls, and business logic operations.

use tracing::{Level, Span};

/// Create a span for database operations with semantic conventions
///
/// # Example
/// ```ignore
/// let span = db_operation_span!("SELECT", "users", user_id = %id);
/// async {
///     // database operation
/// }.instrument(span).await;
/// ```
#[macro_export]
macro_rules! db_operation_span {
    ($operation:expr, $table:expr) => {
        tracing::info_span!(
            "db.query",
            db.system = "postgresql",
            db.operation = $operation,
            db.sql.table = $table,
            otel.kind = "client",
            otel.status_code = tracing::field::Empty,
            error.message = tracing::field::Empty
        )
    };
    ($operation:expr, $table:expr, $($field:tt)*) => {
        tracing::info_span!(
            "db.query",
            db.system = "postgresql",
            db.operation = $operation,
            db.sql.table = $table,
            otel.kind = "client",
            otel.status_code = tracing::field::Empty,
            error.message = tracing::field::Empty,
            $($field)*
        )
    };
}

/// Create a span for external HTTP service calls
///
/// # Example
/// ```ignore
/// let span = external_http_span!("email-service", "POST", "/send");
/// ```
#[macro_export]
macro_rules! external_http_span {
    ($service:expr, $method:expr, $path:expr) => {
        tracing::info_span!(
            "http.client",
            service.name = $service,
            http.method = $method,
            http.url = $path,
            http.status_code = tracing::field::Empty,
            otel.kind = "client",
            otel.status_code = tracing::field::Empty,
            error.message = tracing::field::Empty
        )
    };
}

/// Create a span for business logic operations
///
/// # Example
/// ```ignore
/// let span = business_span!("user.registration", user.email = %email);
/// ```
#[macro_export]
macro_rules! business_span {
    ($operation:expr) => {
        tracing::info_span!(
            $operation,
            otel.kind = "internal",
            otel.status_code = tracing::field::Empty,
            error.message = tracing::field::Empty
        )
    };
    ($operation:expr, $($field:tt)*) => {
        tracing::info_span!(
            $operation,
            otel.kind = "internal",
            otel.status_code = tracing::field::Empty,
            error.message = tracing::field::Empty,
            $($field)*
        )
    };
}

/// Create a span for authentication/authorization operations
///
/// # Example
/// ```ignore
/// let span = auth_span!("login", user.email = %email);
/// ```
#[macro_export]
macro_rules! auth_span {
    ($event:expr) => {
        tracing::info_span!(
            "auth",
            auth.event = $event,
            auth.success = tracing::field::Empty,
            user.id = tracing::field::Empty,
            otel.kind = "internal"
        )
    };
    ($event:expr, $($field:tt)*) => {
        tracing::info_span!(
            "auth",
            auth.event = $event,
            auth.success = tracing::field::Empty,
            otel.kind = "internal",
            $($field)*
        )
    };
}

/// Record a successful operation on the current span
pub fn record_success() {
    Span::current().record("otel.status_code", "OK");
}

/// Record a failed operation on the current span with an error message
pub fn record_error(message: &str) {
    let span = Span::current();
    span.record("otel.status_code", "ERROR");
    span.record("error.message", message);
}

/// Record a user ID on the current span
pub fn record_user_id(user_id: &str) {
    Span::current().record("user.id", user_id);
}

/// Record authentication success/failure on the current span
pub fn record_auth_result(success: bool) {
    Span::current().record("auth.success", success);
}

/// Record HTTP status code on the current span
pub fn record_http_status(status: u16) {
    let span = Span::current();
    span.record("http.status_code", status);
    if status >= 400 {
        span.record("otel.status_code", "ERROR");
    } else {
        span.record("otel.status_code", "OK");
    }
}

/// Log a security-relevant event at WARN level
///
/// Use this for events like failed authentication, authorization denials,
/// suspicious activity, etc.
#[macro_export]
macro_rules! security_event {
    ($event:expr, $($field:tt)*) => {
        tracing::warn!(
            security.event = $event,
            $($field)*
        )
    };
}

/// Log an audit event at INFO level
///
/// Use this for tracking important business operations like
/// user creation, role changes, data modifications, etc.
#[macro_export]
macro_rules! audit_event {
    ($action:expr, $resource:expr, $($field:tt)*) => {
        tracing::info!(
            audit.action = $action,
            audit.resource = $resource,
            $($field)*
        )
    };
}

/// Trait extension for adding context to errors before logging
pub trait ErrorExt {
    /// Log the error and return it unchanged
    fn log_error(self, context: &str) -> Self;
}

impl<T, E: std::fmt::Display> ErrorExt for Result<T, E> {
    fn log_error(self, context: &str) -> Self {
        if let Err(ref e) = self {
            tracing::error!(error = %e, context = context, "Operation failed");
        }
        self
    }
}

/// Create structured fields for a user context
#[macro_export]
macro_rules! user_context {
    ($user_id:expr, $role:expr) => {
        user.id = %$user_id,
        user.role = %$role
    };
}

/// Helper to create a span with standard service fields
pub fn service_span(service_name: &str, operation: &str) -> Span {
    tracing::span!(
        Level::INFO,
        "service.operation",
        service.name = service_name,
        service.operation = operation,
        otel.kind = "internal",
        otel.status_code = tracing::field::Empty
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_success() {
        // Just ensure it doesn't panic when there's no span
        record_success();
    }

    #[test]
    fn test_record_error() {
        // Just ensure it doesn't panic when there's no span
        record_error("test error");
    }

    #[test]
    fn test_record_http_status_success() {
        record_http_status(200);
    }

    #[test]
    fn test_record_http_status_error() {
        record_http_status(500);
    }
}
