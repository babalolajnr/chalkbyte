use chalkbyte::utils::password::hash_password;
#[allow(unused_imports)]
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

#[allow(dead_code)]
pub struct TestUser {
    pub id: Uuid,
    pub email: String,
    pub password: String,
    pub role: String,
    pub school_id: Option<Uuid>,
}

#[allow(dead_code)]
pub struct TestSchool {
    pub id: Uuid,
    pub name: String,
}

pub async fn create_test_user(
    tx: &mut Transaction<'_, Postgres>,
    email: &str,
    password: &str,
    role: &str,
    school_id: Option<Uuid>,
) -> TestUser {
    let hashed = hash_password(password).unwrap();
    let user = sqlx::query!(
        r#"
        INSERT INTO users (first_name, last_name, email, password, role, school_id)
        VALUES ($1, $2, $3, $4, $5::text::user_role, $6)
        RETURNING id, email, role as "role: String", school_id
        "#,
        "Test",
        "User",
        email,
        hashed,
        role,
        school_id
    )
    .fetch_one(&mut **tx)
    .await
    .unwrap();

    TestUser {
        id: user.id,
        email: user.email,
        password: password.to_string(),
        role: user.role,
        school_id: user.school_id,
    }
}

#[allow(dead_code)]
pub async fn create_test_school(tx: &mut Transaction<'_, Postgres>, name: &str) -> TestSchool {
    let school = sqlx::query!(
        r#"
        INSERT INTO schools (name, address)
        VALUES ($1, $2)
        RETURNING id, name
        "#,
        name,
        Some("Test Address")
    )
    .fetch_one(&mut **tx)
    .await
    .unwrap();

    TestSchool {
        id: school.id,
        name: school.name,
    }
}

pub fn generate_unique_email() -> String {
    format!("test-{}@test.com", Uuid::new_v4())
}

#[allow(dead_code)]
pub fn generate_unique_school_name() -> String {
    format!("Test School {}", Uuid::new_v4())
}

#[allow(dead_code)]
pub struct TestLevel {
    pub id: Uuid,
    pub name: String,
    pub school_id: Uuid,
}

#[allow(dead_code)]
pub struct TestBranch {
    pub id: Uuid,
    pub name: String,
    pub level_id: Uuid,
}

#[allow(dead_code)]
pub async fn create_test_level(
    tx: &mut Transaction<'_, Postgres>,
    name: &str,
    school_id: Uuid,
) -> TestLevel {
    let level = sqlx::query!(
        r#"
        INSERT INTO levels (name, description, school_id)
        VALUES ($1, $2, $3)
        RETURNING id, name, school_id
        "#,
        name,
        Some("Test level description"),
        school_id
    )
    .fetch_one(&mut **tx)
    .await
    .unwrap();

    TestLevel {
        id: level.id,
        name: level.name,
        school_id: level.school_id,
    }
}

#[allow(dead_code)]
pub async fn create_test_branch(
    tx: &mut Transaction<'_, Postgres>,
    name: &str,
    level_id: Uuid,
) -> TestBranch {
    let branch = sqlx::query!(
        r#"
        INSERT INTO branches (name, description, level_id)
        VALUES ($1, $2, $3)
        RETURNING id, name, level_id
        "#,
        name,
        Some("Test branch description"),
        level_id
    )
    .fetch_one(&mut **tx)
    .await
    .unwrap();

    TestBranch {
        id: branch.id,
        name: branch.name,
        level_id: branch.level_id,
    }
}

#[allow(dead_code)]
pub fn generate_unique_level_name() -> String {
    format!("Level {}", Uuid::new_v4())
}

#[allow(dead_code)]
pub fn generate_unique_branch_name() -> String {
    format!("Branch {}", Uuid::new_v4())
}
