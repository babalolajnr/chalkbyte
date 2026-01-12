//! User seeding functionality.
//!
//! Provides functions for generating and inserting fake user data
//! (staff and students) into the database.

use chalkbyte_models::users::system_roles;
use chalkbyte_models::{BranchId, LevelId, RoleId, SchoolId, UserId};
use fake::Fake;
use fake::faker::name::en::*;
use rayon::prelude::*;
use sqlx::{PgPool, Postgres, Transaction};
use std::time::Instant;

use super::models::UserSeed;

/// Generates admin and teacher users for schools
pub fn generate_staff_users(
    school_ids: &[SchoolId],
    admins_per_school: usize,
    teachers_per_school: usize,
    password_hash: &str,
) -> Vec<UserSeed> {
    school_ids
        .par_iter()
        .enumerate()
        .flat_map(|(school_idx, &school_id)| {
            let mut users = Vec::with_capacity(admins_per_school + teachers_per_school);

            // Generate admins
            for user_idx in 0..admins_per_school {
                users.push(generate_user(
                    system_roles::ADMIN,
                    Some(school_id),
                    None,
                    None,
                    school_idx,
                    user_idx,
                    "admin",
                    password_hash,
                ));
            }

            // Generate teachers
            for user_idx in 0..teachers_per_school {
                users.push(generate_user(
                    system_roles::TEACHER,
                    Some(school_id),
                    None,
                    None,
                    school_idx,
                    user_idx,
                    "teacher",
                    password_hash,
                ));
            }

            users
        })
        .collect()
}

/// Generates student users assigned to branches and levels
pub fn generate_students(
    branches_with_levels: &[(BranchId, LevelId, SchoolId)], // (branch_id, level_id, school_id)
    students_per_branch: usize,
    password_hash: &str,
) -> Vec<UserSeed> {
    branches_with_levels
        .par_iter()
        .enumerate()
        .flat_map(|(branch_idx, &(branch_id, level_id, school_id))| {
            (0..students_per_branch)
                .map(|student_idx| {
                    generate_user(
                        system_roles::STUDENT,
                        Some(school_id),
                        Some(level_id),
                        Some(branch_id),
                        branch_idx,
                        student_idx,
                        "student",
                        password_hash,
                    )
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn generate_user(
    role_id: RoleId,
    school_id: Option<SchoolId>,
    level_id: Option<LevelId>,
    branch_id: Option<BranchId>,
    group_idx: usize,
    user_idx: usize,
    role_prefix: &str,
    password_hash: &str,
) -> UserSeed {
    let first_name: String = FirstName().fake();
    let last_name: String = LastName().fake();

    let email = format!(
        "{}.{}+{}{}@example.com",
        first_name.to_lowercase(),
        last_name.to_lowercase(),
        role_prefix,
        group_idx * 1000 + user_idx
    );

    UserSeed {
        first_name,
        last_name,
        email,
        password_hash: password_hash.to_string(),
        role_id,
        school_id,
        level_id,
        branch_id,
    }
}

/// Seeds staff users (admins and teachers) into the database
pub async fn seed_staff_users(
    db: &PgPool,
    school_ids: &[SchoolId],
    admins_per_school: usize,
    teachers_per_school: usize,
    password_hash: &str,
) -> Result<Vec<(UserId, RoleId)>, Box<dyn std::error::Error>> {
    let start_time = Instant::now();
    let total_staff = school_ids.len() * (admins_per_school + teachers_per_school);
    println!(
        "👥 Seeding {} staff users ({} admins, {} teachers per school)...",
        total_staff, admins_per_school, teachers_per_school
    );

    let users = generate_staff_users(
        school_ids,
        admins_per_school,
        teachers_per_school,
        password_hash,
    );
    let user_roles = insert_users_batch(db, &users).await?;

    println!(
        "   ✓ Inserted {} staff users in {:?}",
        user_roles.len(),
        start_time.elapsed()
    );

    Ok(user_roles)
}

/// Seeds student users into the database
pub async fn seed_students(
    db: &PgPool,
    branches_with_levels: &[(BranchId, LevelId, SchoolId)], // (branch_id, level_id, school_id)
    students_per_branch: usize,
    password_hash: &str,
) -> Result<Vec<(UserId, RoleId)>, Box<dyn std::error::Error>> {
    let start_time = Instant::now();
    let total_students = branches_with_levels.len() * students_per_branch;
    println!(
        "🎓 Seeding {} students ({} per branch)...",
        total_students, students_per_branch
    );

    let users = generate_students(branches_with_levels, students_per_branch, password_hash);
    let user_roles = insert_users_batch(db, &users).await?;

    println!(
        "   ✓ Inserted {} students in {:?}",
        user_roles.len(),
        start_time.elapsed()
    );

    Ok(user_roles)
}

/// Inserts users in batches, returns (user_id, role_id) tuples for role assignment
pub async fn insert_users_batch(
    db: &PgPool,
    users: &[UserSeed],
) -> Result<Vec<(UserId, RoleId)>, Box<dyn std::error::Error>> {
    let mut tx = db.begin().await?;

    // 7 params per user
    const BATCH_SIZE: usize = 800;

    let mut all_user_roles = Vec::with_capacity(users.len());

    for chunk in users.chunks(BATCH_SIZE) {
        let user_ids = insert_users_chunk(&mut tx, chunk).await?;
        for (user_id, user_seed) in user_ids.iter().zip(chunk.iter()) {
            all_user_roles.push((*user_id, user_seed.role_id));
        }
    }

    tx.commit().await?;
    Ok(all_user_roles)
}

async fn insert_users_chunk(
    tx: &mut Transaction<'_, Postgres>,
    users: &[UserSeed],
) -> Result<Vec<UserId>, Box<dyn std::error::Error>> {
    if users.is_empty() {
        return Ok(Vec::new());
    }

    let mut query = String::from(
        "INSERT INTO users (first_name, last_name, email, password, school_id, level_id, branch_id) VALUES ",
    );

    for (i, _) in users.iter().enumerate() {
        if i > 0 {
            query.push_str(", ");
        }
        let param_idx = i * 7;
        query.push_str(&format!(
            "(${}, ${}, ${}, ${}, ${}, ${}, ${})",
            param_idx + 1,
            param_idx + 2,
            param_idx + 3,
            param_idx + 4,
            param_idx + 5,
            param_idx + 6,
            param_idx + 7
        ));
    }

    query.push_str(" RETURNING id");

    let mut q = sqlx::query_scalar(&query);
    for user in users {
        q = q
            .bind(&user.first_name)
            .bind(&user.last_name)
            .bind(&user.email)
            .bind(&user.password_hash)
            .bind(user.school_id)
            .bind(user.level_id)
            .bind(user.branch_id);
    }

    let ids: Vec<UserId> = q.fetch_all(&mut **tx).await?;
    Ok(ids)
}

/// Assigns roles to users in batches
pub async fn assign_roles_batch(
    db: &PgPool,
    user_roles: &[(UserId, RoleId)],
) -> Result<(), Box<dyn std::error::Error>> {
    let start_time = Instant::now();
    println!("🔐 Assigning roles to {} users...", user_roles.len());

    let mut tx = db.begin().await?;

    const BATCH_SIZE: usize = 2000;

    for chunk in user_roles.chunks(BATCH_SIZE) {
        assign_roles_chunk(&mut tx, chunk).await?;
    }

    tx.commit().await?;

    println!("   ✓ Assigned roles in {:?}", start_time.elapsed());

    Ok(())
}

async fn assign_roles_chunk(
    tx: &mut Transaction<'_, Postgres>,
    user_roles: &[(UserId, RoleId)],
) -> Result<(), Box<dyn std::error::Error>> {
    if user_roles.is_empty() {
        return Ok(());
    }

    let mut query = String::from("INSERT INTO user_roles (user_id, role_id) VALUES ");

    for (i, _) in user_roles.iter().enumerate() {
        if i > 0 {
            query.push_str(", ");
        }
        let param_idx = i * 2;
        query.push_str(&format!("(${}, ${})", param_idx + 1, param_idx + 2));
    }

    query.push_str(" ON CONFLICT (user_id, role_id) DO NOTHING");

    let mut q = sqlx::query(&query);
    for (user_id, role_id) in user_roles {
        q = q.bind(user_id).bind(role_id);
    }

    q.execute(&mut **tx).await?;
    Ok(())
}

/// Clears all seeded users (preserves system admins)
pub async fn clear_users(db: &PgPool) -> Result<u64, Box<dyn std::error::Error>> {
    let start_time = Instant::now();
    println!("🗑️  Clearing seeded users...");

    let result = sqlx::query!(
        r#"DELETE FROM users u
        WHERE u.email LIKE '%@example.com'
        AND NOT EXISTS (
            SELECT 1 FROM user_roles ur
            WHERE ur.user_id = u.id
            AND ur.role_id = $1
        )"#,
        system_roles::SYSTEM_ADMIN as RoleId
    )
    .execute(db)
    .await?
    .rows_affected();

    println!(
        "   ✓ Deleted {} users in {:?}",
        result,
        start_time.elapsed()
    );

    Ok(result)
}
