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
    system_roles,
};
use http_body_util::BodyExt;
use serde_json::json;
use sqlx::PgPool;
use tower::ServiceExt;

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

#[sqlx::test(migrations = "./migrations")]

async fn test_get_profile(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &email, password, "admin", None).await;

    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &email, password).await;

    let app = setup_test_app(pool.clone()).await;

    let request = Request::builder()
        .method("GET")
        .uri("/api/users/profile")
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(body["user"]["email"], email);
    // Role is no longer directly on user; check user exists
    assert!(body["user"]["id"].is_string());
}

#[sqlx::test(migrations = "./migrations")]

async fn test_update_profile(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &email, password, "admin", None).await;

    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &email, password).await;

    let app = setup_test_app(pool.clone()).await;

    let request = Request::builder()
        .method("PUT")
        .uri("/api/users/profile")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", token))
        .body(Body::from(
            serde_json::to_string(&json!({
                "first_name": "Updated",
                "last_name": "Name"
            }))
            .unwrap(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(body["user"]["first_name"], "Updated");
    assert_eq!(body["user"]["last_name"], "Name");
}

#[sqlx::test(migrations = "./migrations")]

async fn test_change_password(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let email = generate_unique_email();
    let old_password = "oldpass123";
    create_test_user(&mut tx, &email, old_password, "admin", None).await;

    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &email, old_password).await;

    let app = setup_test_app(pool.clone()).await;
    let new_password = "newpass456";

    let request = Request::builder()
        .method("POST")
        .uri("/api/users/profile/change-password")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", token))
        .body(Body::from(
            serde_json::to_string(&json!({
                "old_password": old_password,
                "new_password": new_password
            }))
            .unwrap(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let app = setup_test_app(pool.clone()).await;
    let new_token = get_auth_token(app, &email, new_password).await;
    assert!(!new_token.is_empty());
}

#[sqlx::test(migrations = "./migrations")]

async fn test_change_password_wrong_old_password(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &email, password, "admin", None).await;

    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &email, password).await;

    let app = setup_test_app(pool.clone()).await;

    let request = Request::builder()
        .method("POST")
        .uri("/api/users/profile/change-password")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", token))
        .body(Body::from(
            serde_json::to_string(&json!({
                "old_password": "wrongpass",
                "new_password": "newpass456"
            }))
            .unwrap(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    // Wrong password returns 401 Unauthorized
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "./migrations")]

async fn test_create_user_as_system_admin(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let admin_email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &admin_email, password, "system_admin", None).await;

    let school = create_test_school(&mut tx, &generate_unique_school_name()).await;

    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &admin_email, password).await;

    let app = setup_test_app(pool.clone()).await;
    let new_user_email = generate_unique_email();

    let request = Request::builder()
        .method("POST")
        .uri("/api/users")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", token))
        .body(Body::from(
            serde_json::to_string(&json!({
                "first_name": "New",
                "last_name": "User",
                "email": new_user_email,
                "password": "newpass123",
                "role_ids": [system_roles::ADMIN.to_string()],
                "school_id": school.id
            }))
            .unwrap(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(body["email"], new_user_email);
    // Role is assigned via role_ids, user response doesn't include role directly
    assert!(body["id"].is_string());
}

#[sqlx::test(migrations = "./migrations")]

async fn test_create_user_as_school_admin(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let school = create_test_school(&mut tx, &generate_unique_school_name()).await;

    let admin_email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &admin_email, password, "admin", Some(school.id)).await;

    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &admin_email, password).await;

    let app = setup_test_app(pool.clone()).await;
    let new_user_email = generate_unique_email();

    let request = Request::builder()
        .method("POST")
        .uri("/api/users")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", token))
        .body(Body::from(
            serde_json::to_string(&json!({
                "first_name": "New",
                "last_name": "Teacher",
                "email": new_user_email,
                "password": "newpass123",
                "role_ids": [system_roles::TEACHER.to_string()]
            }))
            .unwrap(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(body["email"], new_user_email);
    // Role is assigned via role_ids, user response doesn't include role directly
    assert_eq!(body["school_id"], school.id.to_string());
}

#[sqlx::test(migrations = "./migrations")]

async fn test_create_user_as_teacher_forbidden(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let school = create_test_school(&mut tx, &generate_unique_school_name()).await;

    let teacher_email = generate_unique_email();
    let password = "testpass123";
    create_test_user(
        &mut tx,
        &teacher_email,
        password,
        "teacher",
        Some(school.id),
    )
    .await;

    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &teacher_email, password).await;

    let app = setup_test_app(pool.clone()).await;
    let new_user_email = generate_unique_email();

    let request = Request::builder()
        .method("POST")
        .uri("/api/users")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", token))
        .body(Body::from(
            serde_json::to_string(&json!({
                "first_name": "New",
                "last_name": "User",
                "email": new_user_email,
                "password": "newpass123",
                "role_ids": [system_roles::STUDENT.to_string()]
            }))
            .unwrap(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[sqlx::test(migrations = "./migrations")]

async fn test_create_user_duplicate_email(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let admin_email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &admin_email, password, "system_admin", None).await;

    let existing_email = generate_unique_email();
    create_test_user(&mut tx, &existing_email, "pass123", "student", None).await;

    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &admin_email, password).await;

    let app = setup_test_app(pool.clone()).await;

    let request = Request::builder()
        .method("POST")
        .uri("/api/users")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", token))
        .body(Body::from(
            serde_json::to_string(&json!({
                "first_name": "New",
                "last_name": "User",
                "email": existing_email,
                "password": "newpass123",
                "role": "student"
            }))
            .unwrap(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[sqlx::test(migrations = "./migrations")]

async fn test_get_users_as_system_admin(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let admin_email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &admin_email, password, "system_admin", None).await;

    let user1_email = generate_unique_email();
    create_test_user(&mut tx, &user1_email, "pass123", "student", None).await;

    let user2_email = generate_unique_email();
    create_test_user(&mut tx, &user2_email, "pass123", "teacher", None).await;

    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &admin_email, password).await;

    let app = setup_test_app(pool.clone()).await;

    let request = Request::builder()
        .method("GET")
        .uri("/api/users")
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let users = body["data"].as_array().unwrap();

    assert!(users.iter().any(|u| u["email"] == user1_email));
    assert!(users.iter().any(|u| u["email"] == user2_email));
}

#[sqlx::test(migrations = "./migrations")]

async fn test_get_users_as_school_admin_scoped(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let school1 = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let school2 = create_test_school(&mut tx, &generate_unique_school_name()).await;

    let admin_email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &admin_email, password, "admin", Some(school1.id)).await;

    let user1_email = generate_unique_email();
    create_test_user(
        &mut tx,
        &user1_email,
        "pass123",
        "student",
        Some(school1.id),
    )
    .await;

    let user2_email = generate_unique_email();
    create_test_user(
        &mut tx,
        &user2_email,
        "pass123",
        "student",
        Some(school2.id),
    )
    .await;

    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &admin_email, password).await;

    let app = setup_test_app(pool.clone()).await;

    let request = Request::builder()
        .method("GET")
        .uri("/api/users")
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let users = body["data"].as_array().unwrap();

    assert!(users.iter().any(|u| u["email"] == user1_email));
    assert!(!users.iter().any(|u| u["email"] == user2_email));
}

#[sqlx::test(migrations = "./migrations")]

async fn test_get_users_with_pagination(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let admin_email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &admin_email, password, "system_admin", None).await;

    for _ in 0..5 {
        let user_email = generate_unique_email();
        create_test_user(&mut tx, &user_email, "pass123", "student", None).await;
    }

    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &admin_email, password).await;

    let app = setup_test_app(pool.clone()).await;

    let request = Request::builder()
        .method("GET")
        .uri("/api/users?page=1&limit=2")
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(body["meta"]["page"], 1);
    assert_eq!(body["meta"]["limit"], 2);
    assert!(body["meta"]["total"].as_i64().unwrap() >= 5);
}

#[sqlx::test(migrations = "./migrations")]

async fn test_unauthorized_access_to_profile(pool: PgPool) {
    let app = setup_test_app(pool.clone()).await;

    let request = Request::builder()
        .method("GET")
        .uri("/api/users/profile")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "./migrations")]

async fn test_create_user_invalid_email(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let admin_email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &admin_email, password, "system_admin", None).await;

    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &admin_email, password).await;

    let app = setup_test_app(pool.clone()).await;

    let request = Request::builder()
        .method("POST")
        .uri("/api/users")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", token))
        .body(Body::from(
            serde_json::to_string(&json!({
                "first_name": "New",
                "last_name": "User",
                "email": "not-an-email",
                "password": "newpass123",
                "role": "student"
            }))
            .unwrap(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}
