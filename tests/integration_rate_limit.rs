mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use chalkbyte::config::cors::CorsConfig;
use chalkbyte::config::email::EmailConfig;
use chalkbyte::config::jwt::JwtConfig;
use chalkbyte::config::rate_limit::RateLimitConfig;
use chalkbyte::router::init_router;
use chalkbyte::state::AppState;
use common::{create_test_user, generate_unique_email};
use serde_json::json;
use sqlx::PgPool;
use tower::ServiceExt;

/// Setup test app with custom rate limit config for testing
async fn setup_test_app_with_rate_limit(
    pool: PgPool,
    rate_limit_config: RateLimitConfig,
) -> axum::Router {
    dotenvy::dotenv().ok();
    let state = AppState {
        db: pool,
        jwt_config: JwtConfig::from_env(),
        email_config: EmailConfig::from_env(),
        cors_config: CorsConfig::from_env(),
        rate_limit_config,
    };
    init_router(state)
}

/// Create a strict rate limit config for testing (1 request burst)
fn strict_rate_limit_config() -> RateLimitConfig {
    RateLimitConfig {
        general_per_second: 60,
        general_burst_size: 2,
        auth_per_second: 60,
        auth_burst_size: 1,
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn test_auth_rate_limit_exceeded(pool: PgPool) {
    // Use strict rate limiting: only 1 request allowed for auth
    let config = strict_rate_limit_config();
    let app = setup_test_app_with_rate_limit(pool.clone(), config).await;

    // First request should succeed (even with invalid credentials, it's processed)
    let request1 = Request::builder()
        .method("POST")
        .uri("/api/auth/login")
        .header("content-type", "application/json")
        .header("x-forwarded-for", "192.168.1.100")
        .body(Body::from(
            serde_json::to_string(&json!({
                "email": "test@example.com",
                "password": "password123"
            }))
            .unwrap(),
        ))
        .unwrap();

    let response1 = app.clone().oneshot(request1).await.unwrap();
    // Should get 401 (unauthorized) not 429 (rate limited)
    assert_eq!(response1.status(), StatusCode::UNAUTHORIZED);

    // Second request should be rate limited
    let request2 = Request::builder()
        .method("POST")
        .uri("/api/auth/login")
        .header("content-type", "application/json")
        .header("x-forwarded-for", "192.168.1.100")
        .body(Body::from(
            serde_json::to_string(&json!({
                "email": "test@example.com",
                "password": "password123"
            }))
            .unwrap(),
        ))
        .unwrap();

    let response2 = app.oneshot(request2).await.unwrap();
    assert_eq!(response2.status(), StatusCode::TOO_MANY_REQUESTS);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_different_ips_have_separate_limits(pool: PgPool) {
    let config = strict_rate_limit_config();
    let app = setup_test_app_with_rate_limit(pool.clone(), config).await;

    // Request from IP 1
    let request1 = Request::builder()
        .method("POST")
        .uri("/api/auth/login")
        .header("content-type", "application/json")
        .header("x-forwarded-for", "10.0.0.1")
        .body(Body::from(
            serde_json::to_string(&json!({
                "email": "test@example.com",
                "password": "password123"
            }))
            .unwrap(),
        ))
        .unwrap();

    let response1 = app.clone().oneshot(request1).await.unwrap();
    assert_eq!(response1.status(), StatusCode::UNAUTHORIZED);

    // Request from different IP should not be rate limited
    let request2 = Request::builder()
        .method("POST")
        .uri("/api/auth/login")
        .header("content-type", "application/json")
        .header("x-forwarded-for", "10.0.0.2")
        .body(Body::from(
            serde_json::to_string(&json!({
                "email": "test@example.com",
                "password": "password123"
            }))
            .unwrap(),
        ))
        .unwrap();

    let response2 = app.oneshot(request2).await.unwrap();
    // Should get 401 (processed), not 429 (rate limited)
    assert_eq!(response2.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_general_rate_limit_on_health_endpoint(pool: PgPool) {
    let config = RateLimitConfig {
        general_per_second: 60,
        general_burst_size: 2,
        auth_per_second: 60,
        auth_burst_size: 5,
    };
    let app = setup_test_app_with_rate_limit(pool.clone(), config).await;

    // Health endpoint is not under /api, so no rate limiting
    let request1 = Request::builder()
        .method("GET")
        .uri("/health")
        .header("x-forwarded-for", "172.16.0.1")
        .body(Body::empty())
        .unwrap();

    let response1 = app.clone().oneshot(request1).await.unwrap();
    assert_eq!(response1.status(), StatusCode::OK);

    // Second request should also succeed (health is outside rate limited routes)
    let request2 = Request::builder()
        .method("GET")
        .uri("/health")
        .header("x-forwarded-for", "172.16.0.1")
        .body(Body::empty())
        .unwrap();

    let response2 = app.clone().oneshot(request2).await.unwrap();
    assert_eq!(response2.status(), StatusCode::OK);

    // Third request
    let request3 = Request::builder()
        .method("GET")
        .uri("/health")
        .header("x-forwarded-for", "172.16.0.1")
        .body(Body::empty())
        .unwrap();

    let response3 = app.oneshot(request3).await.unwrap();
    assert_eq!(response3.status(), StatusCode::OK);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_successful_login_still_counts_toward_rate_limit(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();
    let email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &email, password, "student", None).await;
    tx.commit().await.unwrap();

    let config = strict_rate_limit_config();
    let app = setup_test_app_with_rate_limit(pool.clone(), config).await;

    // First request - successful login
    let request1 = Request::builder()
        .method("POST")
        .uri("/api/auth/login")
        .header("content-type", "application/json")
        .header("x-forwarded-for", "203.0.113.50")
        .body(Body::from(
            serde_json::to_string(&json!({
                "email": email,
                "password": password
            }))
            .unwrap(),
        ))
        .unwrap();

    let response1 = app.clone().oneshot(request1).await.unwrap();
    assert_eq!(response1.status(), StatusCode::OK);

    // Second request should be rate limited even though first was successful
    let request2 = Request::builder()
        .method("POST")
        .uri("/api/auth/login")
        .header("content-type", "application/json")
        .header("x-forwarded-for", "203.0.113.50")
        .body(Body::from(
            serde_json::to_string(&json!({
                "email": email,
                "password": password
            }))
            .unwrap(),
        ))
        .unwrap();

    let response2 = app.oneshot(request2).await.unwrap();
    assert_eq!(response2.status(), StatusCode::TOO_MANY_REQUESTS);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_mfa_endpoint_uses_auth_rate_limit(pool: PgPool) {
    let config = strict_rate_limit_config();
    let app = setup_test_app_with_rate_limit(pool.clone(), config).await;

    // First MFA verify request
    let request1 = Request::builder()
        .method("POST")
        .uri("/api/auth/mfa/verify")
        .header("content-type", "application/json")
        .header("x-forwarded-for", "198.51.100.1")
        .body(Body::from(
            serde_json::to_string(&json!({
                "mfa_token": "fake_token",
                "code": "123456"
            }))
            .unwrap(),
        ))
        .unwrap();

    let response1 = app.clone().oneshot(request1).await.unwrap();
    // Will fail auth but not be rate limited
    assert_ne!(response1.status(), StatusCode::TOO_MANY_REQUESTS);

    // Second request should be rate limited
    let request2 = Request::builder()
        .method("POST")
        .uri("/api/auth/mfa/verify")
        .header("content-type", "application/json")
        .header("x-forwarded-for", "198.51.100.1")
        .body(Body::from(
            serde_json::to_string(&json!({
                "mfa_token": "fake_token",
                "code": "123456"
            }))
            .unwrap(),
        ))
        .unwrap();

    let response2 = app.oneshot(request2).await.unwrap();
    assert_eq!(response2.status(), StatusCode::TOO_MANY_REQUESTS);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_forgot_password_rate_limited(pool: PgPool) {
    let config = strict_rate_limit_config();
    let app = setup_test_app_with_rate_limit(pool.clone(), config).await;

    // First forgot password request
    let request1 = Request::builder()
        .method("POST")
        .uri("/api/auth/forgot-password")
        .header("content-type", "application/json")
        .header("x-forwarded-for", "192.0.2.1")
        .body(Body::from(
            serde_json::to_string(&json!({
                "email": "test@example.com"
            }))
            .unwrap(),
        ))
        .unwrap();

    let response1 = app.clone().oneshot(request1).await.unwrap();
    assert_ne!(response1.status(), StatusCode::TOO_MANY_REQUESTS);

    // Second request should be rate limited
    let request2 = Request::builder()
        .method("POST")
        .uri("/api/auth/forgot-password")
        .header("content-type", "application/json")
        .header("x-forwarded-for", "192.0.2.1")
        .body(Body::from(
            serde_json::to_string(&json!({
                "email": "another@example.com"
            }))
            .unwrap(),
        ))
        .unwrap();

    let response2 = app.oneshot(request2).await.unwrap();
    assert_eq!(response2.status(), StatusCode::TOO_MANY_REQUESTS);
}
