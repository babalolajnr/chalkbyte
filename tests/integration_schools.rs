mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use chalkbyte::config::cors::CorsConfig;
use chalkbyte::config::email::EmailConfig;
use chalkbyte::config::jwt::JwtConfig;
use chalkbyte::config::rate_limit::RateLimitConfig;
use chalkbyte::router::init_router;
use chalkbyte::state::AppState;
use common::{
    create_test_school, create_test_user, generate_unique_email, generate_unique_school_name,
};
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
        rate_limit_config: RateLimitConfig::default(),
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
        .uri(&format!("/api/schools/{}", school.id))
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
        .uri(&format!("/api/schools/{}", fake_id))
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
        .uri(&format!("/api/schools/{}", school.id))
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
        .uri(&format!("/api/schools/{}", school2.id))
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
        .uri(&format!("/api/schools/{}/students", school.id))
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
        .uri(&format!("/api/schools/{}/admins", school.id))
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
