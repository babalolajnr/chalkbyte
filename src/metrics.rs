use axum::{
    Router,
    extract::{MatchedPath, Request},
    middleware::Next,
    response::Response,
    routing::get,
};
use metrics::{counter, gauge, histogram};
use metrics_exporter_prometheus::{Matcher, PrometheusBuilder, PrometheusHandle};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

static OBSERVABILITY_ENABLED: OnceLock<bool> = OnceLock::new();

/// Check if observability is enabled via OBSERVABILITY_ENABLED env var
pub fn is_observability_enabled() -> bool {
    *OBSERVABILITY_ENABLED.get_or_init(|| {
        std::env::var("OBSERVABILITY_ENABLED")
            .map(|v| v.to_lowercase() != "false" && v != "0")
            .unwrap_or(true) // Enabled by default
    })
}

/// Initialize Prometheus metrics exporter with upkeep task
/// Returns None if observability is disabled
pub fn init_metrics() -> Option<PrometheusHandle> {
    if !is_observability_enabled() {
        return None;
    }

    let handle = PrometheusBuilder::new()
        .set_buckets_for_metric(
            Matcher::Full("http_request_duration_seconds".to_string()),
            &[
                0.001, 0.005, 0.01, 0.025, 0.05, 0.075, 0.1, 0.25, 0.5, 0.75, 1.0, 2.5, 5.0, 7.5,
                10.0,
            ],
        )
        .expect("Failed to set buckets")
        .install_recorder()
        .expect("Failed to install Prometheus recorder");

    // Spawn upkeep task to clean stale metrics
    let upkeep_handle = handle.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(5)).await;
            upkeep_handle.run_upkeep();
        }
    });

    Some(handle)
}

/// Metrics middleware to track HTTP requests
pub async fn metrics_middleware(req: Request, next: Next) -> Response {
    if !is_observability_enabled() {
        return next.run(req).await;
    }

    let start = Instant::now();
    let method = req.method().as_str().to_owned();
    let uri_path = req.uri().path().to_owned();

    let path = req
        .extensions()
        .get::<MatchedPath>()
        .map(|p| p.as_str().to_owned())
        .unwrap_or(uri_path);

    // Increment active requests
    gauge!("http_requests_active").increment(1.0);

    let response = next.run(req).await;

    let latency = start.elapsed().as_secs_f64();
    let status = response.status().as_u16();
    let status_str = status.to_string();

    // Record metrics
    counter!("http_requests_total", "method" => method.clone(), "path" => path.clone(), "status" => status_str).increment(1);

    histogram!("http_request_duration_seconds", "method" => method, "path" => path).record(latency);

    // Track by status code category
    let status_category = match status {
        200..=299 => "2xx",
        400..=499 => "4xx",
        500..=599 => "5xx",
        _ => "other",
    };
    counter!("http_requests_by_status", "status_category" => status_category).increment(1);

    // Decrement active requests
    gauge!("http_requests_active").decrement(1.0);

    response
}

/// Router for metrics server
pub fn metrics_app(handle: PrometheusHandle) -> Router {
    Router::new().route("/metrics", get(move || async move { handle.render() }))
}

// Business metrics helpers

/// Increment user-related metrics
pub fn track_user_created(role: &str) {
    if !is_observability_enabled() {
        return;
    }
    counter!("users_created_total", "role" => role.to_string()).increment(1);
}

pub fn track_user_login_success(role: &str) {
    if !is_observability_enabled() {
        return;
    }
    counter!("user_logins_total", "role" => role.to_string(), "status" => "success").increment(1);
}

pub fn track_user_login_failure(reason: &str) {
    if !is_observability_enabled() {
        return;
    }
    counter!("user_logins_total", "role" => "unknown", "status" => "failure", "reason" => reason.to_string()).increment(1);
}

/// Track database operations
#[allow(dead_code)]
pub fn track_db_query(operation: &str, success: bool) {
    if !is_observability_enabled() {
        return;
    }
    let status = if success { "success" } else { "error" };
    counter!("database_queries_total", "operation" => operation.to_string(), "status" => status)
        .increment(1);
}

#[allow(dead_code)]
pub fn track_db_query_duration(operation: &str, duration_secs: f64) {
    if !is_observability_enabled() {
        return;
    }
    histogram!("database_query_duration_seconds", "operation" => operation.to_string())
        .record(duration_secs);
}

/// Track school operations
pub fn track_school_created() {
    if !is_observability_enabled() {
        return;
    }
    counter!("schools_created_total").increment(1);
}

#[allow(dead_code)]
pub fn track_school_operation(operation: &str) {
    if !is_observability_enabled() {
        return;
    }
    counter!("school_operations_total", "operation" => operation.to_string()).increment(1);
}

/// Set gauge metrics for current state
#[allow(dead_code)]
pub fn set_active_users(count: i64) {
    if !is_observability_enabled() {
        return;
    }
    gauge!("active_users_total").set(count as f64);
}

#[allow(dead_code)]
pub fn set_total_schools(count: i64) {
    if !is_observability_enabled() {
        return;
    }
    gauge!("schools_total").set(count as f64);
}

#[allow(dead_code)]
pub fn set_total_users_by_role(role: &str, count: i64) {
    if !is_observability_enabled() {
        return;
    }
    gauge!("users_by_role_total", "role" => role.to_string()).set(count as f64);
}

/// Track authentication events
pub fn track_jwt_issued() {
    if !is_observability_enabled() {
        return;
    }
    counter!("jwt_tokens_issued_total").increment(1);
}

#[allow(dead_code)]
pub fn track_jwt_validation(success: bool) {
    if !is_observability_enabled() {
        return;
    }
    let status = if success { "valid" } else { "invalid" };
    counter!("jwt_validations_total", "status" => status).increment(1);
}

/// Track API errors
#[allow(dead_code)]
pub fn track_api_error(error_type: &str, endpoint: &str) {
    if !is_observability_enabled() {
        return;
    }
    counter!("api_errors_total", "error_type" => error_type.to_string(), "endpoint" => endpoint.to_string()).increment(1);
}

/// Track authorization events
#[allow(dead_code)]
pub fn track_authorization_check(allowed: bool, role: &str) {
    if !is_observability_enabled() {
        return;
    }
    let status = if allowed { "allowed" } else { "denied" };
    counter!("authorization_checks_total", "role" => role.to_string(), "status" => status)
        .increment(1);
}
