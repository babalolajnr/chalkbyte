mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use chalkbyte::config::cors::CorsConfig;
use chalkbyte::config::email::EmailConfig;
use chalkbyte::config::jwt::JwtConfig;
use chalkbyte::config::rate_limit::RateLimitConfig;
use chalkbyte::router::init_router;
use chalkbyte::state::AppState;
use chalkbyte_cache::CacheConfig;
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
        rate_limit_config: RateLimitConfig::from_env(),
        cache_config: CacheConfig::default(),
        cache: None,
    };
    init_router(state)
}

async fn get_auth_token(app: axum::Router, email: &str, password: &str) -> String {
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
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    body["access_token"].as_str().unwrap().to_string()
}

#[sqlx::test(migrations = "./migrations")]
async fn test_get_mfa_status_disabled(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &email, password, "student", None).await;

    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &email, password).await;

    let app = setup_test_app(pool).await;

    let request = Request::builder()
        .method("GET")
        .uri("/api/mfa/status")
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(body["mfa_enabled"], false);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_enable_mfa_generates_secret(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &email, password, "student", None).await;

    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &email, password).await;

    let app = setup_test_app(pool).await;

    let request = Request::builder()
        .method("POST")
        .uri("/api/mfa/enable")
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(body.get("secret").is_some());
    assert!(body.get("qr_code_url").is_some() || body.get("qr_code").is_some());
}

#[sqlx::test(migrations = "./migrations")]
async fn test_verify_mfa_invalid_code(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &email, password, "student", None).await;

    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &email, password).await;

    let app = setup_test_app(pool.clone()).await;
    let request = Request::builder()
        .method("POST")
        .uri("/api/mfa/enable")
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    app.oneshot(request).await.unwrap();

    let app = setup_test_app(pool).await;

    let request = Request::builder()
        .method("POST")
        .uri("/api/mfa/verify")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", token))
        .body(Body::from(
            serde_json::to_string(&json!({
                "code": "000000"
            }))
            .unwrap(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_disable_mfa_wrong_password(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &email, password, "student", None).await;

    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &email, password).await;

    let app = setup_test_app(pool).await;

    let request = Request::builder()
        .method("POST")
        .uri("/api/mfa/disable")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", token))
        .body(Body::from(
            serde_json::to_string(&json!({
                "password": "wrongpassword"
            }))
            .unwrap(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_mfa_status_unauthorized(pool: PgPool) {
    let app = setup_test_app(pool).await;

    let request = Request::builder()
        .method("GET")
        .uri("/api/mfa/status")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_enable_mfa_unauthorized(pool: PgPool) {
    let app = setup_test_app(pool).await;

    let request = Request::builder()
        .method("POST")
        .uri("/api/mfa/enable")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_regenerate_recovery_codes_mfa_not_enabled(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &email, password, "student", None).await;

    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &email, password).await;

    let app = setup_test_app(pool).await;

    let request = Request::builder()
        .method("POST")
        .uri("/api/mfa/recovery-codes/regenerate")
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_verify_mfa_validation_empty_code(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &email, password, "student", None).await;

    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &email, password).await;

    let app = setup_test_app(pool).await;

    let request = Request::builder()
        .method("POST")
        .uri("/api/mfa/verify")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", token))
        .body(Body::from(
            serde_json::to_string(&json!({
                "code": ""
            }))
            .unwrap(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_disable_mfa_validation_empty_password(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &email, password, "student", None).await;

    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &email, password).await;

    let app = setup_test_app(pool).await;

    let request = Request::builder()
        .method("POST")
        .uri("/api/mfa/disable")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", token))
        .body(Body::from(
            serde_json::to_string(&json!({
                "password": ""
            }))
            .unwrap(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_mfa_login_verification_returns_all_user_fields(pool: PgPool) {
    use chalkbyte::modules::users::model::User;
    use chalkbyte_models::ids::UserId;

    let mut tx = pool.begin().await.unwrap();

    let email = generate_unique_email();
    let password = "testpass123";
    let test_user = create_test_user(&mut tx, &email, password, "student", None).await;
    let user_id = test_user.id;

    // Enable MFA for the user and set secret
    sqlx::query("UPDATE users SET mfa_enabled = true, mfa_secret = $1 WHERE id = $2")
        .bind("JBSWY3DPEHPK3PXP")
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .unwrap();

    tx.commit().await.unwrap();

    // Test that the SQL query used in verify_mfa_login returns all required User fields
    // This is the exact query from src/modules/auth/service.rs line 151
    let user_result = sqlx::query_as::<_, User>(
        "SELECT id, first_name, last_name, email, school_id, level_id, branch_id, date_of_birth, grade_level, created_at, updated_at FROM users WHERE id = $1"
    )
    .bind(user_id)
    .fetch_one(&pool)
    .await;

    assert!(
        user_result.is_ok(),
        "User query for MFA verification should return all fields without 'level_id' error: {:?}",
        user_result.as_ref().err()
    );

    let user = user_result.unwrap();
    assert_eq!(user.email, email);
    assert_eq!(user.id, UserId::from(user_id));
    assert_eq!(user.first_name, "Test");
    assert_eq!(user.last_name, "User");
    assert!(user.school_id.is_none());
    assert!(user.level_id.is_none());
    assert!(user.branch_id.is_none());
    assert!(user.date_of_birth.is_none());
    assert!(user.grade_level.is_none());
    assert!(user.created_at <= chrono::Utc::now());
    assert!(user.updated_at <= chrono::Utc::now());
}
