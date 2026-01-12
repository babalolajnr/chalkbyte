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
    create_test_role, create_test_school, create_test_user, generate_unique_email,
    generate_unique_role_name, generate_unique_school_name,
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
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap_or_else(|_| {
        panic!(
            "Failed to parse login response. Status: {}, Body: {:?}",
            status,
            String::from_utf8_lossy(&body)
        )
    });
    body["access_token"]
        .as_str()
        .unwrap_or_else(|| {
            panic!(
                "No access_token in response. Status: {}, Body: {}",
                status, body
            )
        })
        .to_string()
}

async fn get_permission_id(pool: &PgPool, name: &str) -> Uuid {
    sqlx::query_scalar!("SELECT id FROM permissions WHERE name = $1", name)
        .fetch_one(pool)
        .await
        .unwrap()
}

// ============ Permission Tests ============

#[sqlx::test(migrations = "./migrations")]
async fn test_get_all_permissions(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &email, password, "system_admin", None).await;

    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &email, password).await;

    let app = setup_test_app(pool.clone()).await;

    let request = Request::builder()
        .method("GET")
        .uri("/api/roles/permissions")
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(!body["data"].as_array().unwrap().is_empty());
    assert!(body["meta"]["total"].as_i64().unwrap() > 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_get_permissions_by_category(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &email, password, "system_admin", None).await;

    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &email, password).await;

    let app = setup_test_app(pool.clone()).await;

    let request = Request::builder()
        .method("GET")
        .uri("/api/roles/permissions?category=users")
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    let permissions = body["data"].as_array().unwrap();
    for perm in permissions {
        assert_eq!(perm["category"], "users");
    }
}

// ============ Role Creation Tests ============

#[sqlx::test(migrations = "./migrations")]
async fn test_create_system_role_as_system_admin(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &email, password, "system_admin", None).await;

    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &email, password).await;

    let app = setup_test_app(pool.clone()).await;

    let role_name = format!("System Role {}", Uuid::new_v4());
    let request = Request::builder()
        .method("POST")
        .uri("/api/roles")
        .header("authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_string(&json!({
                "name": role_name,
                "description": "A test system role"
            }))
            .unwrap(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    // RoleWithPermissions uses #[serde(flatten)] so role fields are at top level
    assert_eq!(body["name"], role_name);
    assert_eq!(body["is_system_role"], true);
    assert!(body["school_id"].is_null());
}

#[sqlx::test(migrations = "./migrations")]
async fn test_create_school_role_as_system_admin(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let school = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &email, password, "system_admin", None).await;

    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &email, password).await;

    let app = setup_test_app(pool.clone()).await;

    let role_name = format!("School Role {}", Uuid::new_v4());
    let request = Request::builder()
        .method("POST")
        .uri("/api/roles")
        .header("authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_string(&json!({
                "name": role_name,
                "description": "A test school role",
                "school_id": school.id
            }))
            .unwrap(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(body["name"], role_name);
    assert_eq!(body["is_system_role"], false);
    assert_eq!(body["school_id"], school.id.to_string());
}

#[sqlx::test(migrations = "./migrations")]
async fn test_create_school_role_as_school_admin(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let school = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &email, password, "admin", Some(school.id)).await;

    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &email, password).await;

    let app = setup_test_app(pool.clone()).await;

    let role_name = format!("School Admin Role {}", Uuid::new_v4());
    let request = Request::builder()
        .method("POST")
        .uri("/api/roles")
        .header("authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_string(&json!({
                "name": role_name,
                "description": "A role created by school admin"
            }))
            .unwrap(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(body["name"], role_name);
    assert_eq!(body["is_system_role"], false);
    assert_eq!(body["school_id"], school.id.to_string());
}

#[sqlx::test(migrations = "./migrations")]
async fn test_create_role_with_permissions(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let school = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &email, password, "admin", Some(school.id)).await;

    tx.commit().await.unwrap();

    let perm1_id = get_permission_id(&pool, "users:read").await;
    let perm2_id = get_permission_id(&pool, "users:create").await;

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &email, password).await;

    let app = setup_test_app(pool.clone()).await;

    let role_name = format!("Role With Perms {}", Uuid::new_v4());
    let request = Request::builder()
        .method("POST")
        .uri("/api/roles")
        .header("authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_string(&json!({
                "name": role_name,
                "permission_ids": [perm1_id, perm2_id]
            }))
            .unwrap(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(body["permissions"].as_array().unwrap().len(), 2);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_create_role_forbidden_for_teacher(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let school = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &email, password, "teacher", Some(school.id)).await;

    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &email, password).await;

    let app = setup_test_app(pool.clone()).await;

    let request = Request::builder()
        .method("POST")
        .uri("/api/roles")
        .header("authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_string(&json!({
                "name": "Unauthorized Role",
                "description": "Should not be created"
            }))
            .unwrap(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_create_duplicate_role_fails(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let school = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &email, password, "admin", Some(school.id)).await;

    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &email, password).await;

    let role_name = format!("Duplicate Role {}", Uuid::new_v4());

    // Create first role
    let app = setup_test_app(pool.clone()).await;
    let request = Request::builder()
        .method("POST")
        .uri("/api/roles")
        .header("authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_string(&json!({
                "name": role_name
            }))
            .unwrap(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Try to create duplicate
    let app = setup_test_app(pool.clone()).await;
    let request = Request::builder()
        .method("POST")
        .uri("/api/roles")
        .header("authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_string(&json!({
                "name": role_name
            }))
            .unwrap(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

// ============ Role Retrieval Tests ============

#[sqlx::test(migrations = "./migrations")]
async fn test_get_roles_as_system_admin_sees_all(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let school = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &email, password, "system_admin", None).await;

    // Create a school role directly in DB
    let role_name = generate_unique_role_name();
    create_test_role(&mut tx, &role_name, Some(school.id), false).await;

    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &email, password).await;

    let app = setup_test_app(pool.clone()).await;

    let request = Request::builder()
        .method("GET")
        .uri("/api/roles")
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(!body["data"].as_array().unwrap().is_empty());
}

#[sqlx::test(migrations = "./migrations")]
async fn test_get_roles_as_school_admin_scoped(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let school1 = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let school2 = create_test_school(&mut tx, &generate_unique_school_name()).await;

    let email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &email, password, "admin", Some(school1.id)).await;

    // Create roles for both schools
    let role1_name = format!("School1 Role {}", Uuid::new_v4());
    let role2_name = format!("School2 Role {}", Uuid::new_v4());

    create_test_role(&mut tx, &role1_name, Some(school1.id), false).await;
    create_test_role(&mut tx, &role2_name, Some(school2.id), false).await;

    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &email, password).await;

    let app = setup_test_app(pool.clone()).await;

    let request = Request::builder()
        .method("GET")
        .uri("/api/roles")
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    // Should only see school1's role (flattened structure)
    let roles = body["data"].as_array().unwrap();
    for role in roles {
        assert_eq!(role["school_id"], school1.id.to_string());
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn test_get_role_by_id(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let school = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &email, password, "admin", Some(school.id)).await;

    // Create a role directly in DB
    let role_name = format!("GetById Role {}", Uuid::new_v4());
    let role = create_test_role(&mut tx, &role_name, Some(school.id), false).await;
    let role_id = role.id;

    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &email, password).await;

    let app = setup_test_app(pool.clone()).await;

    let request = Request::builder()
        .method("GET")
        .uri(format!("/api/roles/{}", role_id))
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    // Flattened structure
    assert_eq!(body["id"], role_id.to_string());
    assert_eq!(body["name"], role_name);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_school_admin_cannot_access_other_school_role(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let school1 = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let school2 = create_test_school(&mut tx, &generate_unique_school_name()).await;

    let email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &email, password, "admin", Some(school1.id)).await;

    // Create a role in school2
    let role_name = format!("Other School Role {}", Uuid::new_v4());
    let role = create_test_role(&mut tx, &role_name, Some(school2.id), false).await;
    let role_id = role.id;

    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &email, password).await;

    let app = setup_test_app(pool.clone()).await;

    let request = Request::builder()
        .method("GET")
        .uri(format!("/api/roles/{}", role_id))
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_school_admin_cannot_access_system_role(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let school = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &email, password, "admin", Some(school.id)).await;

    // Create a system role
    let role_name = format!("System Role {}", Uuid::new_v4());
    let role = create_test_role(&mut tx, &role_name, None, true).await;
    let role_id = role.id;

    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &email, password).await;

    let app = setup_test_app(pool.clone()).await;

    let request = Request::builder()
        .method("GET")
        .uri(format!("/api/roles/{}", role_id))
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

// ============ Role Update Tests ============

#[sqlx::test(migrations = "./migrations")]
async fn test_update_role(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let school = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &email, password, "admin", Some(school.id)).await;

    // Create a role directly in DB
    let role_name = format!("Update Role {}", Uuid::new_v4());
    let role = create_test_role(&mut tx, &role_name, Some(school.id), false).await;
    let role_id = role.id;

    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &email, password).await;

    let app = setup_test_app(pool.clone()).await;

    let new_name = format!("Updated Role {}", Uuid::new_v4());
    let request = Request::builder()
        .method("PUT")
        .uri(format!("/api/roles/{}", role_id))
        .header("authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_string(&json!({
                "name": new_name,
                "description": "Updated description"
            }))
            .unwrap(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    // Flattened structure
    assert_eq!(body["name"], new_name);
    assert_eq!(body["description"], "Updated description");
}

// ============ Role Delete Tests ============

#[sqlx::test(migrations = "./migrations")]
async fn test_delete_role(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let school = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &email, password, "admin", Some(school.id)).await;

    // Create a role directly in DB
    let role_name = format!("Delete Role {}", Uuid::new_v4());
    let role = create_test_role(&mut tx, &role_name, Some(school.id), false).await;
    let role_id = role.id;

    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &email, password).await;

    let app = setup_test_app(pool.clone()).await;

    let request = Request::builder()
        .method("DELETE")
        .uri(format!("/api/roles/{}", role_id))
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    // delete_role returns () which becomes 200 OK with empty body
    assert_eq!(response.status(), StatusCode::OK);

    // Verify role is deleted
    let deleted = sqlx::query!("SELECT id FROM roles WHERE id = $1", role_id)
        .fetch_optional(&pool)
        .await
        .unwrap();

    assert!(deleted.is_none());
}

// ============ Permission Assignment Tests ============

#[sqlx::test(migrations = "./migrations")]
async fn test_assign_permissions_to_role(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let school = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &email, password, "admin", Some(school.id)).await;

    // Create a role with permission
    let role_name = format!("Perm Query Role {}", Uuid::new_v4());
    let role = create_test_role(&mut tx, &role_name, Some(school.id), false).await;
    let role_id = role.id;

    tx.commit().await.unwrap();

    let perm_id = get_permission_id(&pool, "users:read").await;

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &email, password).await;

    let app = setup_test_app(pool.clone()).await;

    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/roles/{}/permissions", role_id))
        .header("authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_string(&json!({
                "permission_ids": [perm_id]
            }))
            .unwrap(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    let permissions = body["permissions"].as_array().unwrap();
    assert!(permissions.iter().any(|p| p["name"] == "users:read"));
}

#[sqlx::test(migrations = "./migrations")]
async fn test_remove_permission_from_role(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let school = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let email = generate_unique_email();
    let password = "testpass123";
    create_test_user(&mut tx, &email, password, "admin", Some(school.id)).await;

    // Create a role
    let role_name = format!("Perm Role {}", Uuid::new_v4());
    let role = create_test_role(&mut tx, &role_name, Some(school.id), false).await;
    let role_id = role.id;

    let perm_id = get_permission_id(&pool, "users:read").await;

    sqlx::query!(
        "INSERT INTO role_permissions (role_id, permission_id) VALUES ($1, $2)",
        role_id,
        perm_id
    )
    .execute(&mut *tx)
    .await
    .unwrap();

    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &email, password).await;

    let app = setup_test_app(pool.clone()).await;

    let request = Request::builder()
        .method("DELETE")
        .uri(format!("/api/roles/{}/permissions/{}", role_id, perm_id))
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    let permissions = body["permissions"].as_array().unwrap();
    assert!(!permissions.iter().any(|p| p["name"] == "users:read"));
}

// ============ User Role Assignment Tests ============

#[sqlx::test(migrations = "./migrations")]
async fn test_assign_role_to_user(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let school = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let admin_email = generate_unique_email();
    let admin_password = "testpass123";
    create_test_user(
        &mut tx,
        &admin_email,
        admin_password,
        "admin",
        Some(school.id),
    )
    .await;

    let user_email = generate_unique_email();
    let target_user =
        create_test_user(&mut tx, &user_email, "userpass", "teacher", Some(school.id)).await;

    // Create a role
    let role_name = format!("Perm Remove Role {}", Uuid::new_v4());
    let role = create_test_role(&mut tx, &role_name, Some(school.id), false).await;
    let role_id = role.id;

    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &admin_email, admin_password).await;

    let app = setup_test_app(pool.clone()).await;

    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/users/{}/roles", target_user.id))
        .header("authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_string(&json!({
                "role_id": role_id
            }))
            .unwrap(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(body["user_id"], target_user.id.to_string());
    assert_eq!(body["role_id"], role_id.to_string());
}

#[sqlx::test(migrations = "./migrations")]
async fn test_assign_duplicate_role_is_idempotent(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let school = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let admin_email = generate_unique_email();
    let admin_password = "testpass123";
    create_test_user(
        &mut tx,
        &admin_email,
        admin_password,
        "admin",
        Some(school.id),
    )
    .await;

    let user_email = generate_unique_email();
    let target_user =
        create_test_user(&mut tx, &user_email, "userpass", "teacher", Some(school.id)).await;

    // Create a role
    let role_name = format!("Assignable Role {}", Uuid::new_v4());
    let role = create_test_role(&mut tx, &role_name, Some(school.id), false).await;
    let role_id = role.id;

    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &admin_email, admin_password).await;

    // First assignment
    let app = setup_test_app(pool.clone()).await;
    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/users/{}/roles", target_user.id))
        .header("authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_string(&json!({
                "role_id": role_id
            }))
            .unwrap(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Second assignment should succeed (idempotent operation)
    let app = setup_test_app(pool.clone()).await;
    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/users/{}/roles", target_user.id))
        .header("authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_string(&json!({
                "role_id": role_id
            }))
            .unwrap(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    // Role assignment is idempotent - re-assigning same role returns success
    assert_eq!(response.status(), StatusCode::OK);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_school_admin_cannot_assign_role_to_other_school_user(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let school1 = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let school2 = create_test_school(&mut tx, &generate_unique_school_name()).await;

    let admin_email = generate_unique_email();
    let admin_password = "testpass123";
    create_test_user(
        &mut tx,
        &admin_email,
        admin_password,
        "admin",
        Some(school1.id),
    )
    .await;

    let user_email = generate_unique_email();
    let other_school_user = create_test_user(
        &mut tx,
        &user_email,
        "userpass",
        "teacher",
        Some(school2.id),
    )
    .await;

    // Create a role in school1
    let role_name = format!("School1 Role {}", Uuid::new_v4());
    let role = create_test_role(&mut tx, &role_name, Some(school1.id), false).await;
    let role_id = role.id;

    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &admin_email, admin_password).await;

    let app = setup_test_app(pool.clone()).await;

    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/users/{}/roles", other_school_user.id))
        .header("authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_string(&json!({
                "role_id": role_id
            }))
            .unwrap(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_school_admin_cannot_assign_system_role(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let school = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let admin_email = generate_unique_email();
    let admin_password = "testpass123";
    create_test_user(
        &mut tx,
        &admin_email,
        admin_password,
        "admin",
        Some(school.id),
    )
    .await;

    let user_email = generate_unique_email();
    let target_user =
        create_test_user(&mut tx, &user_email, "userpass", "teacher", Some(school.id)).await;

    // Create a system role
    let role_name = format!("System Role {}", Uuid::new_v4());
    let role = create_test_role(&mut tx, &role_name, None, true).await;
    let role_id = role.id;

    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &admin_email, admin_password).await;

    let app = setup_test_app(pool.clone()).await;

    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/users/{}/roles", target_user.id))
        .header("authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_string(&json!({
                "role_id": role_id
            }))
            .unwrap(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_remove_role_from_user(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let school = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let admin_email = generate_unique_email();
    let admin_password = "testpass123";
    let admin = create_test_user(
        &mut tx,
        &admin_email,
        admin_password,
        "admin",
        Some(school.id),
    )
    .await;

    let user_email = generate_unique_email();
    let target_user =
        create_test_user(&mut tx, &user_email, "userpass", "teacher", Some(school.id)).await;

    // Create a role
    let role_name = format!("Dup Assign Role {}", Uuid::new_v4());
    let role = create_test_role(&mut tx, &role_name, Some(school.id), false).await;
    let role_id = role.id;

    // Assign role to user directly
    sqlx::query!(
        "INSERT INTO user_roles (user_id, role_id, assigned_by) VALUES ($1, $2, $3)",
        target_user.id,
        role_id,
        admin.id
    )
    .execute(&mut *tx)
    .await
    .unwrap();

    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &admin_email, admin_password).await;

    let app = setup_test_app(pool.clone()).await;

    let request = Request::builder()
        .method("DELETE")
        .uri(format!("/api/users/{}/roles/{}", target_user.id, role_id))
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    // remove_role_from_user returns () which becomes 200 OK
    assert_eq!(response.status(), StatusCode::OK);

    // Verify role is removed
    let assignment = sqlx::query!(
        "SELECT id FROM user_roles WHERE user_id = $1 AND role_id = $2",
        target_user.id,
        role_id
    )
    .fetch_optional(&pool)
    .await
    .unwrap();

    assert!(assignment.is_none());
}

// ============ User Roles/Permissions Query Tests ============

#[sqlx::test(migrations = "./migrations")]
async fn test_get_user_roles(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let school = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let admin_email = generate_unique_email();
    let admin_password = "testpass123";
    let admin = create_test_user(
        &mut tx,
        &admin_email,
        admin_password,
        "admin",
        Some(school.id),
    )
    .await;

    let user_email = generate_unique_email();
    let target_user =
        create_test_user(&mut tx, &user_email, "userpass", "teacher", Some(school.id)).await;

    // Create a role
    let role_name = format!("Removable Role {}", Uuid::new_v4());
    let role = create_test_role(&mut tx, &role_name, Some(school.id), false).await;
    let role_id = role.id;

    sqlx::query!(
        "INSERT INTO user_roles (user_id, role_id, assigned_by) VALUES ($1, $2, $3)",
        target_user.id,
        role_id,
        admin.id
    )
    .execute(&mut *tx)
    .await
    .unwrap();

    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &admin_email, admin_password).await;

    let app = setup_test_app(pool.clone()).await;

    let request = Request::builder()
        .method("GET")
        .uri(format!("/api/users/{}/roles", target_user.id))
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    // Response is Vec<RoleWithPermissions> with flattened role fields
    let roles = body.as_array().unwrap();
    assert!(roles.iter().any(|r| r["name"] == role_name));
}

#[sqlx::test(migrations = "./migrations")]
async fn test_get_user_permissions(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();

    let school = create_test_school(&mut tx, &generate_unique_school_name()).await;
    let admin_email = generate_unique_email();
    let admin_password = "testpass123";
    let admin = create_test_user(
        &mut tx,
        &admin_email,
        admin_password,
        "admin",
        Some(school.id),
    )
    .await;

    let user_email = generate_unique_email();
    let target_user =
        create_test_user(&mut tx, &user_email, "userpass", "teacher", Some(school.id)).await;

    // Create a role
    let role_name = format!("Query Role {}", Uuid::new_v4());
    let role = create_test_role(&mut tx, &role_name, Some(school.id), false).await;
    let role_id = role.id;

    let perm_id = get_permission_id(&pool, "users:read").await;

    sqlx::query!(
        "INSERT INTO role_permissions (role_id, permission_id) VALUES ($1, $2)",
        role_id,
        perm_id
    )
    .execute(&mut *tx)
    .await
    .unwrap();

    sqlx::query!(
        "INSERT INTO user_roles (user_id, role_id, assigned_by) VALUES ($1, $2, $3)",
        target_user.id,
        role_id,
        admin.id
    )
    .execute(&mut *tx)
    .await
    .unwrap();

    tx.commit().await.unwrap();

    let app = setup_test_app(pool.clone()).await;
    let token = get_auth_token(app, &admin_email, admin_password).await;

    let app = setup_test_app(pool.clone()).await;

    let request = Request::builder()
        .method("GET")
        .uri(format!("/api/users/{}/permissions", target_user.id))
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    let permissions = body.as_array().unwrap();
    assert!(permissions.iter().any(|p| p["name"] == "users:read"));
}

// ============ Unauthorized Access Tests ============

#[sqlx::test(migrations = "./migrations")]
async fn test_unauthorized_access_to_roles(pool: PgPool) {
    let app = setup_test_app(pool.clone()).await;

    let request = Request::builder()
        .method("GET")
        .uri("/api/roles")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_unauthorized_access_to_permissions(pool: PgPool) {
    let app = setup_test_app(pool.clone()).await;

    let request = Request::builder()
        .method("GET")
        .uri("/api/roles/permissions")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
