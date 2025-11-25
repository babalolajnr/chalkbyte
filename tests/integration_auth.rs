mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use chalkbyte::config::cors::CorsConfig;
use chalkbyte::config::email::EmailConfig;
use chalkbyte::config::jwt::JwtConfig;
use chalkbyte::router::init_router;
use chalkbyte::state::AppState;
use common::{create_test_user, generate_unique_email};
use http_body_util::BodyExt;
use serde_json::json;
use sqlx::PgPool;
use tower::ServiceExt;

async fn setup_test_app(pool: PgPool) -> axum::Router {
    dotenvy::dotenv().ok();
    let state = AppState {
        db: pool,
        jwt_config: JwtConfig::from_env(),
        email_config: EmailConfig::from_env(),
        cors_config: CorsConfig::from_env(),
    };
    init_router(state)
}

#[sqlx::test(migrations = "./migrations")]
async fn test_login_success(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &email, password, "student", None).await;

    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;

    let request = Request::builder()
        .method("POST")
        .uri("/api/auth/login")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_string(&json!({
                "email": email,
                "password": password
            }))
            .unwrap(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(body.get("access_token").is_some());
    assert!(body.get("user").is_some());
    assert_eq!(body["user"]["email"], email);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_login_invalid_credentials(pool: PgPool) {
    let app = setup_test_app(pool.clone()).await;

    let request = Request::builder()
        .method("POST")
        .uri("/api/auth/login")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_string(&json!({
                "email": "nonexistent@test.com",
                "password": "wrongpass"
            }))
            .unwrap(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_login_invalid_email_format(pool: PgPool) {
    let app = setup_test_app(pool.clone()).await;

    let request = Request::builder()
        .method("POST")
        .uri("/api/auth/login")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_string(&json!({
                "email": "not-an-email",
                "password": "password123"
            }))
            .unwrap(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_login_missing_password(pool: PgPool) {
    let app = setup_test_app(pool.clone()).await;

    let request = Request::builder()
        .method("POST")
        .uri("/api/auth/login")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_string(&json!({
                "email": "test@test.com"
            }))
            .unwrap(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_login_wrong_password(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let email = generate_unique_email();
    let password = "correctpass";
    create_test_user(&mut tx, &email, password, "student", None).await;

    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;

    let request = Request::builder()
        .method("POST")
        .uri("/api/auth/login")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_string(&json!({
                "email": email,
                "password": "wrongpassword"
            }))
            .unwrap(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_login_returns_correct_role(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &email, password, "admin", None).await;

    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;

    let request = Request::builder()
        .method("POST")
        .uri("/api/auth/login")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_string(&json!({
                "email": email,
                "password": password
            }))
            .unwrap(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(body["user"]["role"], "admin");
}
