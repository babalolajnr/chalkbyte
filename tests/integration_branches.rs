mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use chalkbyte::config::cors::CorsConfig;
use chalkbyte::config::email::EmailConfig;
use chalkbyte::config::jwt::JwtConfig;
use chalkbyte::router::init_router;
use chalkbyte::state::AppState;
use common::{
    create_test_branch, create_test_level, create_test_school, create_test_user,
    generate_unique_branch_name, generate_unique_email, generate_unique_level_name,
    generate_unique_school_name,
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

async fn create_branch(
    app: axum::Router,
    token: &str,
    level_id: Uuid,
    name: &str,
    description: Option<&str>,
) -> (StatusCode, serde_json::Value) {
    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/levels/{}/branches", level_id))
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
async fn test_create_branch_as_admin(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();
    let school = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let level = create_test_level(&mut tx, &generate_unique_level_name(), school.id).await;
    let admin_email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &admin_email, password, "admin", Some(school.id)).await;
    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &admin_email, password).await;

    let app = setup_test_app(pool.clone()).await;
    let (status, body) =
        create_branch(app, &token, level.id, "Branch A", Some("First branch")).await;

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["name"], "Branch A");
    assert_eq!(body["description"], "First branch");
    assert_eq!(body["level_id"], level.id.to_string());
    assert!(body["id"].is_string());
}

#[sqlx::test(migrations = "./migrations")]
async fn test_create_branch_as_student_forbidden(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();
    let school = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let level = create_test_level(&mut tx, &generate_unique_level_name(), school.id).await;
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
    let (status, _body) = create_branch(app, &token, level.id, "Branch A", None).await;

    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_create_duplicate_branch_same_level(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();
    let school = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let level = create_test_level(&mut tx, &generate_unique_level_name(), school.id).await;
    let branch_name = generate_unique_branch_name();
    create_test_branch(&mut tx, &branch_name, level.id).await;
    let admin_email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &admin_email, password, "admin", Some(school.id)).await;
    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &admin_email, password).await;

    let app = setup_test_app(pool.clone()).await;
    let (status, _body) = create_branch(app, &token, level.id, &branch_name, None).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_create_same_branch_name_different_levels(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();
    let school = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let level1 = create_test_level(&mut tx, &generate_unique_level_name(), school.id).await;
    let level2 = create_test_level(&mut tx, &generate_unique_level_name(), school.id).await;
    let branch_name = generate_unique_branch_name();
    create_test_branch(&mut tx, &branch_name, level1.id).await;
    let admin_email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &admin_email, password, "admin", Some(school.id)).await;
    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &admin_email, password).await;

    let app = setup_test_app(pool.clone()).await;
    let (status, body) = create_branch(app, &token, level2.id, &branch_name, None).await;

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["name"], branch_name);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_get_branches_by_level(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();
    let school = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let level = create_test_level(&mut tx, &generate_unique_level_name(), school.id).await;
    create_test_branch(&mut tx, "Branch A", level.id).await;
    create_test_branch(&mut tx, "Branch B", level.id).await;
    let admin_email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &admin_email, password, "admin", Some(school.id)).await;
    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &admin_email, password).await;

    let app = setup_test_app(pool.clone()).await;
    let request = Request::builder()
        .method("GET")
        .uri(format!("/api/levels/{}/branches", level.id))
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(status, StatusCode::OK);
    assert!(body["data"].is_array());
    assert_eq!(body["data"].as_array().unwrap().len(), 2);
    assert!(body["meta"]["total"].as_i64().unwrap() >= 2);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_get_branches_scoped_by_school(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();
    let school1 = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let school2 = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let level1 = create_test_level(&mut tx, &generate_unique_level_name(), school1.id).await;
    let level2 = create_test_level(&mut tx, &generate_unique_level_name(), school2.id).await;
    create_test_branch(&mut tx, "Branch A", level1.id).await;
    create_test_branch(&mut tx, "Branch B", level2.id).await;
    let admin_email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &admin_email, password, "admin", Some(school1.id)).await;
    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &admin_email, password).await;

    let app = setup_test_app(pool.clone()).await;
    let request = Request::builder()
        .method("GET")
        .uri(format!("/api/levels/{}/branches", level2.id))
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    let status = response.status();

    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_get_branch_by_id(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();
    let school = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let level = create_test_level(&mut tx, &generate_unique_level_name(), school.id).await;
    let branch = create_test_branch(&mut tx, "Branch A", level.id).await;
    let admin_email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &admin_email, password, "admin", Some(school.id)).await;
    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &admin_email, password).await;

    let app = setup_test_app(pool.clone()).await;
    let request = Request::builder()
        .method("GET")
        .uri(format!("/api/branches/{}", branch.id))
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["id"], branch.id.to_string());
    assert_eq!(body["name"], "Branch A");
    assert_eq!(body["level_id"], level.id.to_string());
    assert!(body["student_count"].is_number());
}

#[sqlx::test(migrations = "./migrations")]
async fn test_get_branch_from_different_school_not_found(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();
    let school1 = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let school2 = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let level = create_test_level(&mut tx, &generate_unique_level_name(), school2.id).await;
    let branch = create_test_branch(&mut tx, "Branch A", level.id).await;
    let admin_email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &admin_email, password, "admin", Some(school1.id)).await;
    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &admin_email, password).await;

    let app = setup_test_app(pool.clone()).await;
    let request = Request::builder()
        .method("GET")
        .uri(format!("/api/branches/{}", branch.id))
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    let status = response.status();

    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_update_branch(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();
    let school = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let level = create_test_level(&mut tx, &generate_unique_level_name(), school.id).await;
    let branch = create_test_branch(&mut tx, "Branch A", level.id).await;
    let admin_email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &admin_email, password, "admin", Some(school.id)).await;
    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &admin_email, password).await;

    let app = setup_test_app(pool.clone()).await;
    let request = Request::builder()
        .method("PUT")
        .uri(format!("/api/branches/{}", branch.id))
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", token))
        .body(Body::from(
            serde_json::to_string(&json!({
                "name": "Branch A Updated",
                "description": "Updated description"
            }))
            .unwrap(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["id"], branch.id.to_string());
    assert_eq!(body["name"], "Branch A Updated");
    assert_eq!(body["description"], "Updated description");
}

#[sqlx::test(migrations = "./migrations")]
async fn test_delete_branch(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();
    let school = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let level = create_test_level(&mut tx, &generate_unique_level_name(), school.id).await;
    let branch = create_test_branch(&mut tx, "Branch A", level.id).await;
    let admin_email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &admin_email, password, "admin", Some(school.id)).await;
    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &admin_email, password).await;

    let app = setup_test_app(pool.clone()).await;
    let request = Request::builder()
        .method("DELETE")
        .uri(format!("/api/branches/{}", branch.id))
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    let status = response.status();

    assert_eq!(status, StatusCode::NO_CONTENT);

    let app = setup_test_app(pool.clone()).await;
    let request = Request::builder()
        .method("GET")
        .uri(format!("/api/branches/{}", branch.id))
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    let status = response.status();

    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_assign_students_to_branch(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();
    let school = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let level = create_test_level(&mut tx, &generate_unique_level_name(), school.id).await;
    let branch = create_test_branch(&mut tx, "Branch A", level.id).await;
    let student1_email = generate_unique_email();
    let student2_email = generate_unique_email();
    let password = "testpass123";
    let student1 = create_test_user(
        &mut tx,
        &student1_email,
        password,
        "student",
        Some(school.id),
    )
    .await;
    let student2 = create_test_user(
        &mut tx,
        &student2_email,
        password,
        "student",
        Some(school.id),
    )
    .await;
    let admin_email = generate_unique_email();
    create_test_user(&mut tx, &admin_email, password, "admin", Some(school.id)).await;
    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &admin_email, password).await;

    let app = setup_test_app(pool.clone()).await;
    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/branches/{}/students", branch.id))
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
    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["assigned_count"], 2);
    assert_eq!(body["failed_ids"].as_array().unwrap().len(), 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_assign_students_with_invalid_ids(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();
    let school = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let level = create_test_level(&mut tx, &generate_unique_level_name(), school.id).await;
    let branch = create_test_branch(&mut tx, "Branch A", level.id).await;
    let student_email = generate_unique_email();
    let password = "testpass123";
    let student = create_test_user(
        &mut tx,
        &student_email,
        password,
        "student",
        Some(school.id),
    )
    .await;
    let admin_email = generate_unique_email();
    create_test_user(&mut tx, &admin_email, password, "admin", Some(school.id)).await;
    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &admin_email, password).await;

    let invalid_id = Uuid::new_v4();
    let app = setup_test_app(pool.clone()).await;
    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/branches/{}/students", branch.id))
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", token))
        .body(Body::from(
            serde_json::to_string(&json!({
                "student_ids": [student.id, invalid_id]
            }))
            .unwrap(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["assigned_count"], 1);
    assert_eq!(body["failed_ids"].as_array().unwrap().len(), 1);
    assert_eq!(body["failed_ids"][0], invalid_id.to_string());
}

#[sqlx::test(migrations = "./migrations")]
async fn test_move_student_to_branch(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();
    let school = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let level = create_test_level(&mut tx, &generate_unique_level_name(), school.id).await;
    let branch1 = create_test_branch(&mut tx, "Branch A", level.id).await;
    let branch2 = create_test_branch(&mut tx, "Branch B", level.id).await;
    let student_email = generate_unique_email();
    let password = "testpass123";
    let student = create_test_user(
        &mut tx,
        &student_email,
        password,
        "student",
        Some(school.id),
    )
    .await;
    sqlx::query!(
        "UPDATE users SET branch_id = $1 WHERE id = $2",
        branch1.id,
        student.id
    )
    .execute(&mut *tx)
    .await
    .unwrap();
    let admin_email = generate_unique_email();
    create_test_user(&mut tx, &admin_email, password, "admin", Some(school.id)).await;
    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &admin_email, password).await;

    let app = setup_test_app(pool.clone()).await;
    let request = Request::builder()
        .method("PATCH")
        .uri(format!("/api/branches/students/move/{}", student.id))
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", token))
        .body(Body::from(
            serde_json::to_string(&json!({
                "branch_id": branch2.id
            }))
            .unwrap(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    let status = response.status();

    assert_eq!(status, StatusCode::NO_CONTENT);

    let app = setup_test_app(pool.clone()).await;
    let request = Request::builder()
        .method("GET")
        .uri(format!("/api/branches/{}/students", branch2.id))
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(
        body.as_array()
            .unwrap()
            .iter()
            .any(|s| s["id"] == student.id.to_string())
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn test_remove_student_from_branch(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();
    let school = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let level = create_test_level(&mut tx, &generate_unique_level_name(), school.id).await;
    let branch = create_test_branch(&mut tx, "Branch A", level.id).await;
    let student_email = generate_unique_email();
    let password = "testpass123";
    let student = create_test_user(
        &mut tx,
        &student_email,
        password,
        "student",
        Some(school.id),
    )
    .await;
    sqlx::query!(
        "UPDATE users SET branch_id = $1 WHERE id = $2",
        branch.id,
        student.id
    )
    .execute(&mut *tx)
    .await
    .unwrap();
    let admin_email = generate_unique_email();
    create_test_user(&mut tx, &admin_email, password, "admin", Some(school.id)).await;
    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &admin_email, password).await;

    let app = setup_test_app(pool.clone()).await;
    let request = Request::builder()
        .method("DELETE")
        .uri(format!("/api/branches/students/{}", student.id))
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    let status = response.status();

    assert_eq!(status, StatusCode::NO_CONTENT);

    let result = sqlx::query!("SELECT branch_id FROM users WHERE id = $1", student.id)
        .fetch_one(&pool)
        .await
        .unwrap();

    assert!(result.branch_id.is_none());
}

#[sqlx::test(migrations = "./migrations")]
async fn test_get_students_in_branch(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();
    let school = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let level = create_test_level(&mut tx, &generate_unique_level_name(), school.id).await;
    let branch = create_test_branch(&mut tx, "Branch A", level.id).await;
    let student1_email = generate_unique_email();
    let student2_email = generate_unique_email();
    let password = "testpass123";
    let student1 = create_test_user(
        &mut tx,
        &student1_email,
        password,
        "student",
        Some(school.id),
    )
    .await;
    let student2 = create_test_user(
        &mut tx,
        &student2_email,
        password,
        "student",
        Some(school.id),
    )
    .await;
    sqlx::query!(
        "UPDATE users SET branch_id = $1 WHERE id = $2",
        branch.id,
        student1.id
    )
    .execute(&mut *tx)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE users SET branch_id = $1 WHERE id = $2",
        branch.id,
        student2.id
    )
    .execute(&mut *tx)
    .await
    .unwrap();
    let admin_email = generate_unique_email();
    create_test_user(&mut tx, &admin_email, password, "admin", Some(school.id)).await;
    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &admin_email, password).await;

    let app = setup_test_app(pool.clone()).await;
    let request = Request::builder()
        .method("GET")
        .uri(format!("/api/branches/{}/students", branch.id))
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_array());
    assert_eq!(body.as_array().unwrap().len(), 2);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_branch_with_student_count(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();
    let school = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let level = create_test_level(&mut tx, &generate_unique_level_name(), school.id).await;
    let branch = create_test_branch(&mut tx, "Branch A", level.id).await;
    let student1_email = generate_unique_email();
    let student2_email = generate_unique_email();
    let student3_email = generate_unique_email();
    let password = "testpass123";
    let student1 = create_test_user(
        &mut tx,
        &student1_email,
        password,
        "student",
        Some(school.id),
    )
    .await;
    let student2 = create_test_user(
        &mut tx,
        &student2_email,
        password,
        "student",
        Some(school.id),
    )
    .await;
    let student3 = create_test_user(
        &mut tx,
        &student3_email,
        password,
        "student",
        Some(school.id),
    )
    .await;
    sqlx::query!(
        "UPDATE users SET branch_id = $1 WHERE id = $2",
        branch.id,
        student1.id
    )
    .execute(&mut *tx)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE users SET branch_id = $1 WHERE id = $2",
        branch.id,
        student2.id
    )
    .execute(&mut *tx)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE users SET branch_id = $1 WHERE id = $2",
        branch.id,
        student3.id
    )
    .execute(&mut *tx)
    .await
    .unwrap();
    let admin_email = generate_unique_email();
    create_test_user(&mut tx, &admin_email, password, "admin", Some(school.id)).await;
    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &admin_email, password).await;

    let app = setup_test_app(pool.clone()).await;
    let request = Request::builder()
        .method("GET")
        .uri(format!("/api/branches/{}", branch.id))
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["student_count"], 3);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_cannot_assign_teacher_to_branch(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();
    let school = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let level = create_test_level(&mut tx, &generate_unique_level_name(), school.id).await;
    let branch = create_test_branch(&mut tx, "Branch A", level.id).await;
    let teacher_email = generate_unique_email();
    let password = "testpass123";
    let teacher = create_test_user(
        &mut tx,
        &teacher_email,
        password,
        "teacher",
        Some(school.id),
    )
    .await;
    let admin_email = generate_unique_email();
    create_test_user(&mut tx, &admin_email, password, "admin", Some(school.id)).await;
    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &admin_email, password).await;

    let app = setup_test_app(pool.clone()).await;
    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/branches/{}/students", branch.id))
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
    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["assigned_count"], 0);
    assert_eq!(body["failed_ids"].as_array().unwrap().len(), 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_unauthorized_access_to_branches(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();
    let school = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let level = create_test_level(&mut tx, &generate_unique_level_name(), school.id).await;
    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let request = Request::builder()
        .method("GET")
        .uri(format!("/api/levels/{}/branches", level.id))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    let status = response.status();

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_filter_branches_by_name(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();
    let school = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let level = create_test_level(&mut tx, &generate_unique_level_name(), school.id).await;
    create_test_branch(&mut tx, "Science Branch", level.id).await;
    create_test_branch(&mut tx, "Arts Branch", level.id).await;
    create_test_branch(&mut tx, "Commerce Branch", level.id).await;
    let admin_email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &admin_email, password, "admin", Some(school.id)).await;
    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &admin_email, password).await;

    let app = setup_test_app(pool.clone()).await;
    let request = Request::builder()
        .method("GET")
        .uri(format!("/api/levels/{}/branches?name=Science", level.id))
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(status, StatusCode::OK);
    assert!(body["data"].is_array());
    let branches = body["data"].as_array().unwrap();
    assert!(branches.len() > 0);
    assert!(
        branches
            .iter()
            .all(|b| b["name"].as_str().unwrap().contains("Science"))
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn test_pagination_for_branches(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();
    let school = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let level = create_test_level(&mut tx, &generate_unique_level_name(), school.id).await;
    for i in 1..=15 {
        create_test_branch(&mut tx, &format!("Branch {}", i), level.id).await;
    }
    let admin_email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &admin_email, password, "admin", Some(school.id)).await;
    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &admin_email, password).await;

    let app = setup_test_app(pool.clone()).await;
    let request = Request::builder()
        .method("GET")
        .uri(format!("/api/levels/{}/branches?page=1&limit=10", level.id))
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"].as_array().unwrap().len(), 10);
    assert_eq!(body["meta"]["page"], 1);
    assert_eq!(body["meta"]["limit"], 10);
    assert_eq!(body["meta"]["has_more"], true);
    assert!(body["meta"]["total"].as_i64().unwrap() >= 15);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_move_student_to_null_branch(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();
    let school = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let level = create_test_level(&mut tx, &generate_unique_level_name(), school.id).await;
    let branch = create_test_branch(&mut tx, "Branch A", level.id).await;
    let student_email = generate_unique_email();
    let password = "testpass123";
    let student = create_test_user(
        &mut tx,
        &student_email,
        password,
        "student",
        Some(school.id),
    )
    .await;
    sqlx::query!(
        "UPDATE users SET branch_id = $1 WHERE id = $2",
        branch.id,
        student.id
    )
    .execute(&mut *tx)
    .await
    .unwrap();
    let admin_email = generate_unique_email();
    create_test_user(&mut tx, &admin_email, password, "admin", Some(school.id)).await;
    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &admin_email, password).await;

    let app = setup_test_app(pool.clone()).await;
    let request = Request::builder()
        .method("PATCH")
        .uri(format!("/api/branches/students/move/{}", student.id))
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", token))
        .body(Body::from(
            serde_json::to_string(&json!({
                "branch_id": null
            }))
            .unwrap(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    let status = response.status();

    assert_eq!(status, StatusCode::NO_CONTENT);

    let result = sqlx::query!("SELECT branch_id FROM users WHERE id = $1", student.id)
        .fetch_one(&pool)
        .await
        .unwrap();

    assert!(result.branch_id.is_none());
}

#[sqlx::test(migrations = "./migrations")]
async fn test_cross_school_branch_assignment_forbidden(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();
    let school1 = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let school2 = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let level1 = create_test_level(&mut tx, &generate_unique_level_name(), school1.id).await;
    let _level2 = create_test_level(&mut tx, &generate_unique_level_name(), school2.id).await;
    let branch1 = create_test_branch(&mut tx, "Branch A", level1.id).await;
    let student_email = generate_unique_email();
    let password = "testpass123";
    let student = create_test_user(
        &mut tx,
        &student_email,
        password,
        "student",
        Some(school2.id),
    )
    .await;
    let admin_email = generate_unique_email();
    create_test_user(&mut tx, &admin_email, password, "admin", Some(school1.id)).await;
    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &admin_email, password).await;

    let app = setup_test_app(pool.clone()).await;
    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/branches/{}/students", branch1.id))
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", token))
        .body(Body::from(
            serde_json::to_string(&json!({
                "student_ids": [student.id]
            }))
            .unwrap(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["assigned_count"], 0);
    assert_eq!(body["failed_ids"].as_array().unwrap().len(), 1);
}
