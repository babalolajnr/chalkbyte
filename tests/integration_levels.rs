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
use common::{
    create_test_school, create_test_user, generate_unique_email, generate_unique_school_name,
};
use http_body_util::BodyExt;
use serde_json::json;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

async fn setup_test_app(pool: PgPool) -> axum::Router {
    dotenvy::dotenv().ok();
    let state = AppState {
        db: pool.clone(),
        jwt_config: JwtConfig::from_env(),
        email_config: EmailConfig::from_env(),
        cors_config: CorsConfig::from_env(),
        rate_limit_config: RateLimitConfig::default(),
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

async fn create_level(
    app: axum::Router,
    token: &str,
    name: &str,
    description: Option<&str>,
) -> (StatusCode, serde_json::Value) {
    let request = Request::builder()
        .method("POST")
        .uri("/api/levels")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", token))
        .body(Body::from(
            serde_json::to_string(&json!({
                "name": name,
                "description": description
            }))
            .unwrap(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    (status, body)
}

#[sqlx::test(migrations = "./migrations")]
async fn test_create_level_as_admin(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();
    let school = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let admin_email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &admin_email, password, "admin", Some(school.id)).await;
    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &admin_email, password).await;

    let app = setup_test_app(pool.clone()).await;
    let (status, body) = create_level(app, &token, "Grade 10", Some("Tenth grade level")).await;

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["name"], "Grade 10");
    assert_eq!(body["description"], "Tenth grade level");
    assert!(body["id"].is_string());
}

#[sqlx::test(migrations = "./migrations")]
async fn test_create_level_as_student_forbidden(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();
    let school = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let student_email = generate_unique_email();
    let password = "testpass123";
    create_test_user(
        &mut tx,
        &student_email,
        password,
        "student",
        Some(school.id),
    )
    .await;
    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &student_email, password).await;

    let app = setup_test_app(pool.clone()).await;
    let (status, _) = create_level(app, &token, "Grade 10", None).await;

    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_create_duplicate_level_same_school(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();
    let school = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let admin_email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &admin_email, password, "admin", Some(school.id)).await;
    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &admin_email, password).await;

    let app = setup_test_app(pool.clone()).await;
    let (status, _) = create_level(app, &token, "Grade 10", None).await;
    assert_eq!(status, StatusCode::CREATED);

    let app = setup_test_app(pool.clone()).await;
    let (status, _) = create_level(app, &token, "Grade 10", None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_create_same_level_name_different_schools(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();
    let school1 = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let school2 = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let admin1_email = generate_unique_email();
    let admin2_email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &admin1_email, password, "admin", Some(school1.id)).await;
    create_test_user(&mut tx, &admin2_email, password, "admin", Some(school2.id)).await;
    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token1 = get_auth_token(app, &admin1_email, password).await;

    let app = setup_test_app(pool.clone()).await;
    let token2 = get_auth_token(app, &admin2_email, password).await;

    let app = setup_test_app(pool.clone()).await;
    let (status1, _) = create_level(app, &token1, "Grade 10", None).await;
    assert_eq!(status1, StatusCode::CREATED);

    let app = setup_test_app(pool.clone()).await;
    let (status2, _) = create_level(app, &token2, "Grade 10", None).await;
    assert_eq!(status2, StatusCode::CREATED);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_get_levels_by_school(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();
    let school = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let admin_email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &admin_email, password, "admin", Some(school.id)).await;
    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &admin_email, password).await;

    let app = setup_test_app(pool.clone()).await;
    create_level(app, &token, "Grade 9", None).await;
    let app = setup_test_app(pool.clone()).await;
    create_level(app, &token, "Grade 10", None).await;

    let app = setup_test_app(pool.clone()).await;
    let request = Request::builder()
        .method("GET")
        .uri("/api/levels")
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(body["data"].is_array());
    assert_eq!(body["data"].as_array().unwrap().len(), 2);
    assert!(body["meta"]["total"].as_i64().unwrap() >= 2);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_get_levels_scoped_by_school(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();
    let school1 = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let school2 = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let admin1_email = generate_unique_email();
    let admin2_email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &admin1_email, password, "admin", Some(school1.id)).await;
    create_test_user(&mut tx, &admin2_email, password, "admin", Some(school2.id)).await;
    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token1 = get_auth_token(app, &admin1_email, password).await;
    let app = setup_test_app(pool.clone()).await;
    let token2 = get_auth_token(app, &admin2_email, password).await;

    let app = setup_test_app(pool.clone()).await;
    create_level(app, &token1, "School1 Level", None).await;
    let app = setup_test_app(pool.clone()).await;
    create_level(app, &token2, "School2 Level", None).await;

    let app = setup_test_app(pool.clone()).await;
    let request = Request::builder()
        .method("GET")
        .uri("/api/levels")
        .header("authorization", format!("Bearer {}", token1))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    let levels = body["data"].as_array().unwrap();
    assert_eq!(levels.len(), 1);
    assert_eq!(levels[0]["name"], "School1 Level");
}

#[sqlx::test(migrations = "./migrations")]
async fn test_get_level_by_id(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();
    let school = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let admin_email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &admin_email, password, "admin", Some(school.id)).await;
    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &admin_email, password).await;

    let app = setup_test_app(pool.clone()).await;
    let (_, create_body) = create_level(app, &token, "Grade 10", Some("Test level")).await;
    let level_id = create_body["id"].as_str().unwrap();

    let app = setup_test_app(pool.clone()).await;
    let request = Request::builder()
        .method("GET")
        .uri(format!("/api/levels/{}", level_id))
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(body["id"], level_id);
    assert_eq!(body["name"], "Grade 10");
    assert_eq!(body["student_count"], 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_get_level_from_different_school_not_found(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();
    let school1 = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let school2 = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let admin1_email = generate_unique_email();
    let admin2_email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &admin1_email, password, "admin", Some(school1.id)).await;
    create_test_user(&mut tx, &admin2_email, password, "admin", Some(school2.id)).await;
    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token1 = get_auth_token(app, &admin1_email, password).await;
    let app = setup_test_app(pool.clone()).await;
    let token2 = get_auth_token(app, &admin2_email, password).await;

    let app = setup_test_app(pool.clone()).await;
    let (_, create_body) = create_level(app, &token1, "Grade 10", None).await;
    let level_id = create_body["id"].as_str().unwrap();

    let app = setup_test_app(pool.clone()).await;
    let request = Request::builder()
        .method("GET")
        .uri(format!("/api/levels/{}", level_id))
        .header("authorization", format!("Bearer {}", token2))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_update_level(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();
    let school = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let admin_email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &admin_email, password, "admin", Some(school.id)).await;
    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &admin_email, password).await;

    let app = setup_test_app(pool.clone()).await;
    let (_, create_body) = create_level(app, &token, "Grade 10", None).await;
    let level_id = create_body["id"].as_str().unwrap();

    let app = setup_test_app(pool.clone()).await;
    let request = Request::builder()
        .method("PUT")
        .uri(format!("/api/levels/{}", level_id))
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", token))
        .body(Body::from(
            serde_json::to_string(&json!({
                "name": "Grade 11",
                "description": "Updated description"
            }))
            .unwrap(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(body["name"], "Grade 11");
    assert_eq!(body["description"], "Updated description");
}

#[sqlx::test(migrations = "./migrations")]
async fn test_delete_level(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();
    let school = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let admin_email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &admin_email, password, "admin", Some(school.id)).await;
    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &admin_email, password).await;

    let app = setup_test_app(pool.clone()).await;
    let (_, create_body) = create_level(app, &token, "Grade 10", None).await;
    let level_id = create_body["id"].as_str().unwrap();

    let app = setup_test_app(pool.clone()).await;
    let request = Request::builder()
        .method("DELETE")
        .uri(format!("/api/levels/{}", level_id))
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    let app = setup_test_app(pool.clone()).await;
    let request = Request::builder()
        .method("GET")
        .uri(format!("/api/levels/{}", level_id))
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_assign_students_to_level(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();
    let school = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let admin_email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &admin_email, password, "admin", Some(school.id)).await;

    let student1 = create_test_user(
        &mut tx,
        &generate_unique_email(),
        "pass123",
        "student",
        Some(school.id),
    )
    .await;
    let student2 = create_test_user(
        &mut tx,
        &generate_unique_email(),
        "pass123",
        "student",
        Some(school.id),
    )
    .await;
    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &admin_email, password).await;

    let app = setup_test_app(pool.clone()).await;
    let (_, create_body) = create_level(app, &token, "Grade 10", None).await;
    let level_id = create_body["id"].as_str().unwrap();

    let app = setup_test_app(pool.clone()).await;
    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/levels/{}/students", level_id))
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", token))
        .body(Body::from(
            serde_json::to_string(&json!({
                "student_ids": [student1.id, student2.id]
            }))
            .unwrap(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(body["assigned_count"], 2);
    assert_eq!(body["failed_ids"].as_array().unwrap().len(), 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_assign_students_with_invalid_ids(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();
    let school = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let admin_email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &admin_email, password, "admin", Some(school.id)).await;

    let student1 = create_test_user(
        &mut tx,
        &generate_unique_email(),
        "pass123",
        "student",
        Some(school.id),
    )
    .await;
    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &admin_email, password).await;

    let app = setup_test_app(pool.clone()).await;
    let (_, create_body) = create_level(app, &token, "Grade 10", None).await;
    let level_id = create_body["id"].as_str().unwrap();

    let invalid_id = Uuid::new_v4();

    let app = setup_test_app(pool.clone()).await;
    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/levels/{}/students", level_id))
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", token))
        .body(Body::from(
            serde_json::to_string(&json!({
                "student_ids": [student1.id, invalid_id]
            }))
            .unwrap(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(body["assigned_count"], 1);
    assert_eq!(body["failed_ids"].as_array().unwrap().len(), 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_move_student_to_level(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();
    let school = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let admin_email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &admin_email, password, "admin", Some(school.id)).await;

    let student = create_test_user(
        &mut tx,
        &generate_unique_email(),
        "pass123",
        "student",
        Some(school.id),
    )
    .await;
    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &admin_email, password).await;

    let app = setup_test_app(pool.clone()).await;
    let (_, level1_body) = create_level(app, &token, "Grade 9", None).await;
    let level1_id = level1_body["id"].as_str().unwrap();

    let app = setup_test_app(pool.clone()).await;
    let (_, level2_body) = create_level(app, &token, "Grade 10", None).await;
    let level2_id = level2_body["id"].as_str().unwrap();

    let app = setup_test_app(pool.clone()).await;
    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/levels/{}/students", level1_id))
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", token))
        .body(Body::from(
            serde_json::to_string(&json!({
                "student_ids": [student.id]
            }))
            .unwrap(),
        ))
        .unwrap();

    app.oneshot(request).await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let request = Request::builder()
        .method("PATCH")
        .uri(format!("/api/levels/students/{}/move", student.id))
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", token))
        .body(Body::from(
            serde_json::to_string(&json!({
                "level_id": level2_id
            }))
            .unwrap(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_remove_student_from_level(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();
    let school = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let admin_email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &admin_email, password, "admin", Some(school.id)).await;

    let student = create_test_user(
        &mut tx,
        &generate_unique_email(),
        "pass123",
        "student",
        Some(school.id),
    )
    .await;
    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &admin_email, password).await;

    let app = setup_test_app(pool.clone()).await;
    let (_, create_body) = create_level(app, &token, "Grade 10", None).await;
    let level_id = create_body["id"].as_str().unwrap();

    let app = setup_test_app(pool.clone()).await;
    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/levels/{}/students", level_id))
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", token))
        .body(Body::from(
            serde_json::to_string(&json!({
                "student_ids": [student.id]
            }))
            .unwrap(),
        ))
        .unwrap();

    app.oneshot(request).await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let request = Request::builder()
        .method("DELETE")
        .uri(format!("/api/levels/students/{}", student.id))
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_get_students_in_level(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();
    let school = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let admin_email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &admin_email, password, "admin", Some(school.id)).await;

    let student1 = create_test_user(
        &mut tx,
        &generate_unique_email(),
        "pass123",
        "student",
        Some(school.id),
    )
    .await;
    let student2 = create_test_user(
        &mut tx,
        &generate_unique_email(),
        "pass123",
        "student",
        Some(school.id),
    )
    .await;
    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &admin_email, password).await;

    let app = setup_test_app(pool.clone()).await;
    let (_, create_body) = create_level(app, &token, "Grade 10", None).await;
    let level_id = create_body["id"].as_str().unwrap();

    let app = setup_test_app(pool.clone()).await;
    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/levels/{}/students", level_id))
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", token))
        .body(Body::from(
            serde_json::to_string(&json!({
                "student_ids": [student1.id, student2.id]
            }))
            .unwrap(),
        ))
        .unwrap();

    app.oneshot(request).await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let request = Request::builder()
        .method("GET")
        .uri(format!("/api/levels/{}/students", level_id))
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(body.is_array());
    assert_eq!(body.as_array().unwrap().len(), 2);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_level_with_student_count(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();
    let school = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let admin_email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &admin_email, password, "admin", Some(school.id)).await;

    let student1 = create_test_user(
        &mut tx,
        &generate_unique_email(),
        "pass123",
        "student",
        Some(school.id),
    )
    .await;
    let student2 = create_test_user(
        &mut tx,
        &generate_unique_email(),
        "pass123",
        "student",
        Some(school.id),
    )
    .await;
    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &admin_email, password).await;

    let app = setup_test_app(pool.clone()).await;
    let (_, create_body) = create_level(app, &token, "Grade 10", None).await;
    let level_id = create_body["id"].as_str().unwrap();

    let app = setup_test_app(pool.clone()).await;
    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/levels/{}/students", level_id))
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", token))
        .body(Body::from(
            serde_json::to_string(&json!({
                "student_ids": [student1.id, student2.id]
            }))
            .unwrap(),
        ))
        .unwrap();

    app.oneshot(request).await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let request = Request::builder()
        .method("GET")
        .uri(format!("/api/levels/{}", level_id))
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(body["student_count"], 2);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_cannot_assign_teacher_to_level(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();
    let school = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let admin_email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &admin_email, password, "admin", Some(school.id)).await;

    let teacher = create_test_user(
        &mut tx,
        &generate_unique_email(),
        "pass123",
        "teacher",
        Some(school.id),
    )
    .await;
    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &admin_email, password).await;

    let app = setup_test_app(pool.clone()).await;
    let (_, create_body) = create_level(app, &token, "Grade 10", None).await;
    let level_id = create_body["id"].as_str().unwrap();

    let app = setup_test_app(pool.clone()).await;
    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/levels/{}/students", level_id))
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", token))
        .body(Body::from(
            serde_json::to_string(&json!({
                "student_ids": [teacher.id]
            }))
            .unwrap(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(body["assigned_count"], 0);
    assert_eq!(body["failed_ids"].as_array().unwrap().len(), 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_unauthorized_access_to_levels(pool: PgPool) {
    let app = setup_test_app(pool.clone()).await;
    let request = Request::builder()
        .method("GET")
        .uri("/api/levels")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
