mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use chalkbyte::config::cors::CorsConfig;
use chalkbyte::config::email::EmailConfig;
use chalkbyte::config::jwt::JwtConfig;
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
        db: pool.clone(),
        jwt_config: JwtConfig::from_env(),
        email_config: EmailConfig::from_env(),
        cors_config: CorsConfig::from_env(),
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

async fn test_create_student_as_admin(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let school = create_test_school(&mut tx, &generate_unique_school_name()).await;

    let admin_email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &admin_email, password, "admin", Some(school.id)).await;

    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &admin_email, password).await;

    let app = setup_test_app(pool.clone()).await;
    let student_email = generate_unique_email();

    let request = Request::builder()
        .method("POST")
        .uri("/api/students")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", token))
        .body(Body::from(
            serde_json::to_string(&json!({
                "first_name": "Student",
                "last_name": "Test",
                "email": student_email,
                "password": "studentpass123",
                "date_of_birth": "2010-01-15",
                "grade_level": "10"
            }))
            .unwrap(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(body["email"], student_email);
    assert_eq!(body["grade_level"], "10");
}

#[sqlx::test(migrations = "./migrations")]

async fn test_create_student_as_student_forbidden(pool: PgPool) {
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
    let new_student_email = generate_unique_email();

    let request = Request::builder()
        .method("POST")
        .uri("/api/students")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", token))
        .body(Body::from(
            serde_json::to_string(&json!({
                "first_name": "Student",
                "last_name": "Test",
                "email": new_student_email,
                "password": "studentpass123",
                "date_of_birth": "2010-01-15",
                "grade_level": "10"
            }))
            .unwrap(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[sqlx::test(migrations = "./migrations")]

async fn test_get_students_as_admin(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let school = create_test_school(&mut tx, &generate_unique_school_name()).await;

    let admin_email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &admin_email, password, "admin", Some(school.id)).await;

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
    let token = get_auth_token(app, &admin_email, password).await;

    let app = setup_test_app(pool.clone()).await;

    let request = Request::builder()
        .method("GET")
        .uri("/api/students")
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let students = body["data"].as_array().unwrap();

    assert!(students.iter().any(|s| s["email"] == student1_email));
    assert!(students.iter().any(|s| s["email"] == student2_email));
}

#[sqlx::test(migrations = "./migrations")]

async fn test_get_students_scoped_by_school(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let school1 = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let school2 = create_test_school(&mut tx, &generate_unique_school_name()).await;

    let admin_email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &admin_email, password, "admin", Some(school1.id)).await;

    let student1_email = generate_unique_email();
    create_test_user(
        &mut tx,
        &student1_email,
        "pass123",
        "student",
        Some(school1.id),
    )
    .await;

    let student2_email = generate_unique_email();
    create_test_user(
        &mut tx,
        &student2_email,
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
        .uri("/api/students")
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let students = body["data"].as_array().unwrap();

    assert!(students.iter().any(|s| s["email"] == student1_email));
    assert!(!students.iter().any(|s| s["email"] == student2_email));
}

#[sqlx::test(migrations = "./migrations")]

async fn test_get_student_by_id(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let school = create_test_school(&mut tx, &generate_unique_school_name()).await;

    let admin_email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &admin_email, password, "admin", Some(school.id)).await;

    let student_email = generate_unique_email();
    let student = create_test_user(
        &mut tx,
        &student_email,
        "pass123",
        "student",
        Some(school.id),
    )
    .await;

    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &admin_email, password).await;

    let app = setup_test_app(pool.clone()).await;

    let request = Request::builder()
        .method("GET")
        .uri(&format!("/api/students/{}", student.id))
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(body["email"], student_email);
    assert_eq!(body["id"], student.id.to_string());
}

#[sqlx::test(migrations = "./migrations")]

async fn test_get_student_from_different_school_forbidden(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let school1 = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let school2 = create_test_school(&mut tx, &generate_unique_school_name()).await;

    let admin_email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &admin_email, password, "admin", Some(school1.id)).await;

    let student_email = generate_unique_email();
    let student = create_test_user(
        &mut tx,
        &student_email,
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
        .uri(&format!("/api/students/{}", student.id))
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[sqlx::test(migrations = "./migrations")]

async fn test_update_student(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let school = create_test_school(&mut tx, &generate_unique_school_name()).await;

    let admin_email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &admin_email, password, "admin", Some(school.id)).await;

    let student_email = generate_unique_email();
    let student = create_test_user(
        &mut tx,
        &student_email,
        "pass123",
        "student",
        Some(school.id),
    )
    .await;

    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &admin_email, password).await;

    let app = setup_test_app(pool.clone()).await;

    let request = Request::builder()
        .method("PUT")
        .uri(&format!("/api/students/{}", student.id))
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", token))
        .body(Body::from(
            serde_json::to_string(&json!({
                "first_name": "Updated",
                "last_name": "Student",
                "grade_level": "11"
            }))
            .unwrap(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(body["first_name"], "Updated");
    assert_eq!(body["last_name"], "Student");
    assert_eq!(body["grade_level"], "11");
}

#[sqlx::test(migrations = "./migrations")]

async fn test_delete_student(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let school = create_test_school(&mut tx, &generate_unique_school_name()).await;

    let admin_email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &admin_email, password, "admin", Some(school.id)).await;

    let student_email = generate_unique_email();
    let student = create_test_user(
        &mut tx,
        &student_email,
        "pass123",
        "student",
        Some(school.id),
    )
    .await;

    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &admin_email, password).await;

    let app = setup_test_app(pool.clone()).await;

    let request = Request::builder()
        .method("DELETE")
        .uri(&format!("/api/students/{}", student.id))
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[sqlx::test(migrations = "./migrations")]

async fn test_delete_student_from_different_school_forbidden(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let school1 = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let school2 = create_test_school(&mut tx, &generate_unique_school_name()).await;

    let admin_email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &admin_email, password, "admin", Some(school1.id)).await;

    let student_email = generate_unique_email();
    let student = create_test_user(
        &mut tx,
        &student_email,
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
        .method("DELETE")
        .uri(&format!("/api/students/{}", student.id))
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[sqlx::test(migrations = "./migrations")]

async fn test_create_student_invalid_email(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let school = create_test_school(&mut tx, &generate_unique_school_name()).await;

    let admin_email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &admin_email, password, "admin", Some(school.id)).await;

    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &admin_email, password).await;

    let app = setup_test_app(pool.clone()).await;

    let request = Request::builder()
        .method("POST")
        .uri("/api/students")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", token))
        .body(Body::from(
            serde_json::to_string(&json!({
                "first_name": "Student",
                "last_name": "Test",
                "email": "not-an-email",
                "password": "studentpass123",
                "date_of_birth": "2010-01-15",
                "grade_level": "10"
            }))
            .unwrap(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[sqlx::test(migrations = "./migrations")]

async fn test_get_students_with_pagination(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let school = create_test_school(&mut tx, &generate_unique_school_name()).await;

    let admin_email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &admin_email, password, "admin", Some(school.id)).await;

    for i in 0..5 {
        let student_email = generate_unique_email();
        create_test_user(
            &mut tx,
            &student_email,
            "pass123",
            "student",
            Some(school.id),
        )
        .await;
    }

    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &admin_email, password).await;

    let app = setup_test_app(pool.clone()).await;

    let request = Request::builder()
        .method("GET")
        .uri("/api/students?page=1&limit=2")
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(body["meta"]["page"], 1);
    assert_eq!(body["meta"]["limit"], 2);
    let data = body["data"].as_array().unwrap();
    assert_eq!(data.len(), 2);
}

#[sqlx::test(migrations = "./migrations")]

async fn test_unauthorized_access_to_students(pool: PgPool) {
    let app = setup_test_app(pool.clone()).await;

    let request = Request::builder()
        .method("GET")
        .uri("/api/students")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
