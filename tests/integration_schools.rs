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
use chalkbyte_core::file_storage::LocalFileStorage;
use common::{
    create_test_school, create_test_user, generate_unique_email, generate_unique_school_name,
};
use http_body_util::BodyExt;
use serde_json::json;
use sqlx::PgPool;
use std::path::PathBuf;
use std::sync::Arc;
use tower::ServiceExt;

async fn setup_test_app(pool: PgPool) -> axum::Router {
    dotenvy::dotenv().ok();
    
    // Create a temporary uploads directory for tests
    let test_uploads_dir = PathBuf::from("./test_uploads");
    let _ = tokio::fs::create_dir_all(&test_uploads_dir).await;
    
    let file_storage = Arc::new(LocalFileStorage::new(
        test_uploads_dir,
        "http://localhost:3000/files".to_string(),
    ));
    
    let state = AppState {
        db: pool,
        jwt_config: JwtConfig::from_env(),
        email_config: EmailConfig::from_env(),
        cors_config: CorsConfig::from_env(),
        rate_limit_config: RateLimitConfig::default(),
        cache_config: CacheConfig::default(),
        cache: None,
        file_storage,
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
    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8_lossy(&body);
    
    if !status.is_success() {
        panic!("Login failed with status {}: {}", status, body_str);
    }
    
    let body: serde_json::Value = serde_json::from_slice(&body)
        .unwrap_or_else(|e| panic!("Failed to parse login response: {}. Body: {}", e, body_str));
    body["access_token"].as_str().unwrap().to_string()
}

#[sqlx::test(migrations = "./migrations")]
async fn test_create_school_as_system_admin(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &email, password, "system_admin", None).await;

    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &email, password).await;

    let app = setup_test_app(pool).await;
    let school_name = generate_unique_school_name();

    let request = Request::builder()
        .method("POST")
        .uri("/api/schools")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", token))
        .body(Body::from(
            serde_json::to_string(&json!({
                "name": school_name,
                "address": "123 Test St"
            }))
            .unwrap(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(body["name"], school_name);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_create_school_as_admin_forbidden(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let email = generate_unique_email();
    let password = "testpass123";
    let school = create_test_school(&mut tx, &generate_unique_school_name()).await;
    create_test_user(&mut tx, &email, password, "admin", Some(school.id)).await;

    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &email, password).await;

    let app = setup_test_app(pool).await;
    let school_name = generate_unique_school_name();

    let request = Request::builder()
        .method("POST")
        .uri("/api/schools")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", token))
        .body(Body::from(
            serde_json::to_string(&json!({
                "name": school_name,
                "address": "123 Test St"
            }))
            .unwrap(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_create_school_duplicate_name(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &email, password, "system_admin", None).await;

    let school_name = generate_unique_school_name();
    create_test_school(&mut tx, &school_name).await;

    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &email, password).await;

    let app = setup_test_app(pool).await;

    let request = Request::builder()
        .method("POST")
        .uri("/api/schools")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", token))
        .body(Body::from(
            serde_json::to_string(&json!({
                "name": school_name,
                "address": "123 Test St"
            }))
            .unwrap(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_get_all_schools_as_system_admin(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &email, password, "system_admin", None).await;

    let school1 = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let school2 = create_test_school(&mut tx, &generate_unique_school_name()).await;

    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &email, password).await;

    let app = setup_test_app(pool).await;

    let request = Request::builder()
        .method("GET")
        .uri("/api/schools")
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let schools = body["data"].as_array().unwrap();

    assert!(schools.iter().any(|s| s["id"] == school1.id.to_string()));
    assert!(schools.iter().any(|s| s["id"] == school2.id.to_string()));
}

#[sqlx::test(migrations = "./migrations")]
async fn test_get_school_by_id(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &email, password, "system_admin", None).await;

    let school = create_test_school(&mut tx, &generate_unique_school_name()).await;

    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &email, password).await;

    let app = setup_test_app(pool).await;

    let request = Request::builder()
        .method("GET")
        .uri(format!("/api/schools/{}", school.id))
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(body["name"], school.name);
    assert_eq!(body["id"], school.id.to_string());
}

#[sqlx::test(migrations = "./migrations")]
async fn test_get_nonexistent_school(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &email, password, "system_admin", None).await;

    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &email, password).await;

    let app = setup_test_app(pool).await;
    let fake_id = uuid::Uuid::new_v4();

    let request = Request::builder()
        .method("GET")
        .uri(format!("/api/schools/{}", fake_id))
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_delete_school_as_system_admin(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &email, password, "system_admin", None).await;

    let school = create_test_school(&mut tx, &generate_unique_school_name()).await;

    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &email, password).await;

    let app = setup_test_app(pool).await;

    let request = Request::builder()
        .method("DELETE")
        .uri(format!("/api/schools/{}", school.id))
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_delete_school_as_admin_forbidden(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let school1 = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let school2 = create_test_school(&mut tx, &generate_unique_school_name()).await;

    let email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &email, password, "admin", Some(school1.id)).await;

    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &email, password).await;

    let app = setup_test_app(pool).await;

    let request = Request::builder()
        .method("DELETE")
        .uri(format!("/api/schools/{}", school2.id))
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_get_school_students(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &email, password, "system_admin", None).await;

    let school = create_test_school(&mut tx, &generate_unique_school_name()).await;

    let student1_email = generate_unique_email();
    create_test_user(
        &mut tx,
        &student1_email,
        "pass123",
        "student",
        Some(school.id),
    )
    .await;

    let student2_email = generate_unique_email();
    create_test_user(
        &mut tx,
        &student2_email,
        "pass123",
        "student",
        Some(school.id),
    )
    .await;

    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &email, password).await;

    let app = setup_test_app(pool).await;

    let request = Request::builder()
        .method("GET")
        .uri(format!("/api/schools/{}/students", school.id))
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let students = body["data"].as_array().unwrap();

    assert!(students.len() >= 2);
    assert!(students.iter().any(|s| s["email"] == student1_email));
    assert!(students.iter().any(|s| s["email"] == student2_email));
}

#[sqlx::test(migrations = "./migrations")]
async fn test_get_school_admins(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &email, password, "system_admin", None).await;

    let school = create_test_school(&mut tx, &generate_unique_school_name()).await;

    let admin_email = generate_unique_email();
    create_test_user(&mut tx, &admin_email, "pass123", "admin", Some(school.id)).await;

    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &email, password).await;

    let app = setup_test_app(pool).await;

    let request = Request::builder()
        .method("GET")
        .uri(format!("/api/schools/{}/admins", school.id))
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let admins = body["data"].as_array().unwrap();

    assert!(admins.iter().any(|a| a["email"] == admin_email));
}

#[sqlx::test(migrations = "./migrations")]
async fn test_unauthorized_access_to_schools(pool: PgPool) {
    let app = setup_test_app(pool).await;

    let request = Request::builder()
        .method("GET")
        .uri("/api/schools")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_upload_school_logo_as_system_admin(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &email, password, "system_admin", None).await;
    let school = create_test_school(&mut tx, "Test School Logo").await;

    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &email, password).await;

    let app = setup_test_app(pool).await;

    // Create a simple PNG file (1x1 transparent PNG)
    let png_data = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48,
        0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00,
        0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78,
        0x9C, 0x63, 0x00, 0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00,
        0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];

    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/schools/{}/logo", school.id))
        .header("authorization", format!("Bearer {}", token))
        .header("content-type", "image/png")
        .body(Body::from(png_data))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(body["logo_path"].is_string());
    assert!(body["logo_path"].as_str().unwrap().starts_with("schools/"));
}

#[sqlx::test(migrations = "./migrations")]
async fn test_upload_school_logo_invalid_mime_type(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &email, password, "system_admin", None).await;
    let school = create_test_school(&mut tx, "Test School Logo").await;

    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &email, password).await;

    let app = setup_test_app(pool).await;

    // Try to upload with invalid MIME type
    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/schools/{}/logo", school.id))
        .header("authorization", format!("Bearer {}", token))
        .header("content-type", "text/plain")
        .body(Body::from("This is not an image"))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_upload_school_logo_oversized_file(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &email, password, "system_admin", None).await;
    let school = create_test_school(&mut tx, "Test School Logo").await;

    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &email, password).await;

    let app = setup_test_app(pool).await;

    // Create a file larger than 5MB
    let oversized_data = vec![0u8; 6 * 1024 * 1024];

    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/schools/{}/logo", school.id))
        .header("authorization", format!("Bearer {}", token))
        .header("content-type", "image/png")
        .body(Body::from(oversized_data))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_school_admin_can_upload_own_logo(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let admin_email = generate_unique_email();
    let password = "testpass123";
    let school = create_test_school(&mut tx, "Test School Logo").await;
    create_test_user(&mut tx, &admin_email, password, "admin", Some(school.id)).await;

    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &admin_email, password).await;

    let app = setup_test_app(pool).await;

    let png_data = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48,
        0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00,
        0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78,
        0x9C, 0x63, 0x00, 0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00,
        0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];

    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/schools/{}/logo", school.id))
        .header("authorization", format!("Bearer {}", token))
        .header("content-type", "image/png")
        .body(Body::from(png_data))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_school_admin_cannot_upload_other_school_logo(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let admin_email = generate_unique_email();
    let password = "testpass123";
    let school1 = create_test_school(&mut tx, "School 1").await;
    let school2 = create_test_school(&mut tx, "School 2").await;
    create_test_user(&mut tx, &admin_email, password, "admin", Some(school1.id)).await;

    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &admin_email, password).await;

    let app = setup_test_app(pool).await;

    let png_data = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48,
        0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00,
        0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78,
        0x9C, 0x63, 0x00, 0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00,
        0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];

    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/schools/{}/logo", school2.id))
        .header("authorization", format!("Bearer {}", token))
        .header("content-type", "image/png")
        .body(Body::from(png_data))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_delete_school_logo(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &email, password, "system_admin", None).await;
    let school = create_test_school(&mut tx, "Test School Logo").await;

    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &email, password).await;

    // First upload a logo
    let app = setup_test_app(pool.clone()).await;
    let png_data = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48,
        0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00,
        0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78,
        0x9C, 0x63, 0x00, 0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00,
        0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];

    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/schools/{}/logo", school.id))
        .header("authorization", format!("Bearer {}", token))
        .header("content-type", "image/png")
        .body(Body::from(png_data))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Now delete the logo
    let app = setup_test_app(pool).await;
    let request = Request::builder()
        .method("DELETE")
        .uri(format!("/api/schools/{}/logo", school.id))
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_upload_logo_replaces_existing(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &email, password, "system_admin", None).await;
    let school = create_test_school(&mut tx, "Test School Logo").await;

    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &email, password).await;

    let png_data = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48,
        0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00,
        0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78,
        0x9C, 0x63, 0x00, 0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00,
        0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];

    // Upload first logo
    let app = setup_test_app(pool.clone()).await;
    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/schools/{}/logo", school.id))
        .header("authorization", format!("Bearer {}", token.clone()))
        .header("content-type", "image/png")
        .body(Body::from(png_data.clone()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let first_response: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let first_logo_path = first_response["logo_path"].as_str().unwrap().to_string();

    // Upload second logo - should replace
    let app = setup_test_app(pool).await;
    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/schools/{}/logo", school.id))
        .header("authorization", format!("Bearer {}", token))
        .header("content-type", "image/png")
        .body(Body::from(png_data))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let second_response: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let second_logo_path = second_response["logo_path"].as_str().unwrap().to_string();

    // Paths should be different (new upload, new timestamp)
    assert_ne!(first_logo_path, second_logo_path);
}

