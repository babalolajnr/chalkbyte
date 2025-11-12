mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use chalkbyte::db::init_app_state;
use chalkbyte::router::init_router;
use common::{cleanup_test_data, create_test_user, generate_unique_email, get_test_pool};
use http_body_util::BodyExt;
use serde_json::json;
use serial_test::serial;
use tower::ServiceExt;

async fn setup_test_app() -> axum::Router {
    dotenvy::dotenv().ok();
    let state = init_app_state().await;
    init_router(state)
}

#[tokio::test]
#[serial]
async fn test_login_success() {
    let pool = get_test_pool().await;
    let mut tx = pool.begin().await.unwrap();

    let email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &email, password, "student", None).await;

    tx.commit().await.unwrap();

    let app = setup_test_app().await;

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

    cleanup_test_data(&pool).await;
}

#[tokio::test]
#[serial]
async fn test_login_invalid_credentials() {
    let app = setup_test_app().await;

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

#[tokio::test]
#[serial]
async fn test_login_invalid_email_format() {
    let app = setup_test_app().await;

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

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
#[serial]
async fn test_login_missing_password() {
    let app = setup_test_app().await;

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

#[tokio::test]
#[serial]
async fn test_login_wrong_password() {
    let pool = get_test_pool().await;
    let mut tx = pool.begin().await.unwrap();

    let email = generate_unique_email();
    let password = "correctpass";
    create_test_user(&mut tx, &email, password, "student", None).await;

    tx.commit().await.unwrap();

    let app = setup_test_app().await;

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

    cleanup_test_data(&pool).await;
}

#[tokio::test]
#[serial]
async fn test_login_returns_correct_role() {
    let pool = get_test_pool().await;
    let mut tx = pool.begin().await.unwrap();

    let email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &email, password, "admin", None).await;

    tx.commit().await.unwrap();

    let app = setup_test_app().await;

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

    cleanup_test_data(&pool).await;
}
