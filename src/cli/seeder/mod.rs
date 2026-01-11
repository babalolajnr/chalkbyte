//! Database seeding module for populating test data.
//!
//! This module provides functionality to seed the database with fake schools,
//! levels (grades), branches (sections), and users (admins, teachers, students).
//!
//! # Module Structure
//!
//! - [`schools`] - School generation and insertion
//! - [`levels`] - Level/grade generation and insertion
//! - [`branches`] - Branch/section generation and insertion
//! - [`users`] - User generation (staff and students) with role assignment
//! - [`models`] - Data structures for seeding configuration
//!
//! # Usage
//!
//! ## Full seeding
//! ```ignore
//! use chalkbyte::cli::seeder::{seed_all, SeedConfig};
//!
//! let config = SeedConfig::new(10); // 10 schools with defaults
//! seed_all(&db, config).await?;
//! ```
//!
//! ## Individual seeding
//! ```ignore
//! // Seed schools first
//! let school_ids = seed_schools_only(&db, 5).await?;
//!
//! // Then levels
//! let level_ids = seed_levels_only(&db, &school_ids, 6).await?;
//!
//! // Then branches
//! let branch_ids = seed_branches_only(&db, &level_ids, 3).await?;
//! ```
//!
//! # Performance
//!
//! - Parallel data generation using Rayon
//! - Batch inserts with multi-value INSERT statements
//! - Single bcrypt hash reused for all users (cost 4 for speed)
//! - Pre-allocated vectors to avoid reallocation overhead

pub mod branches;
pub mod levels;
pub mod models;
pub mod schools;
pub mod users;

pub use models::{LevelsPerSchool, SeedConfig, UsersPerSchool};

use bcrypt::hash;
use sqlx::PgPool;
use std::time::Instant;

/// Seeds the entire database with schools, levels, branches, and users
pub async fn seed_all(db: &PgPool, config: SeedConfig) -> Result<(), Box<dyn std::error::Error>> {
    let start_time = Instant::now();

    println!("🌱 Starting full database seeding...");
    println!("   - Schools: {}", config.num_schools);
    println!(
        "   - Levels per school: {}, Branches per level: {}",
        config.levels_per_school.count, config.levels_per_school.branches_per_level
    );
    println!(
        "   - Users per school: {} admins, {} teachers, {} students",
        config.users_per_school.admins,
        config.users_per_school.teachers,
        config.total_students_per_school()
    );

    // Hash password once
    let password_hash = hash_password()?;

    // Step 1: Seed schools
    let school_ids = schools::seed_schools(db, config.num_schools).await?;

    // Step 2: Seed levels for all schools
    let level_ids = levels::seed_levels(db, &school_ids, config.levels_per_school.count).await?;

    // Step 3: Seed branches for all levels
    let branch_ids =
        branches::seed_branches(db, &level_ids, config.levels_per_school.branches_per_level)
            .await?;

    // Step 4: Build branch -> level -> school mapping for students
    let branches_with_context = build_branch_context(
        &school_ids,
        &level_ids,
        &branch_ids,
        config.levels_per_school.count,
        config.levels_per_school.branches_per_level,
    );

    // Step 5: Seed staff users (admins and teachers)
    let staff_roles = users::seed_staff_users(
        db,
        &school_ids,
        config.users_per_school.admins,
        config.users_per_school.teachers,
        &password_hash,
    )
    .await?;

    // Step 6: Seed students
    let student_roles = users::seed_students(
        db,
        &branches_with_context,
        config.levels_per_school.students_per_branch,
        &password_hash,
    )
    .await?;

    // Step 7: Assign roles
    let mut all_roles = staff_roles;
    all_roles.extend(student_roles);
    users::assign_roles_batch(db, &all_roles).await?;

    let total_users = config.num_schools * config.total_users_per_school();
    println!(
        "\n✅ Seeding complete! Created {} schools, {} levels, {} branches, {} users in {:?}",
        config.num_schools,
        level_ids.len(),
        branch_ids.len(),
        total_users,
        start_time.elapsed()
    );
    println!("\n📝 Default password for all users: password123");

    Ok(())
}

/// Seeds only schools
pub async fn seed_schools_only(
    db: &PgPool,
    count: usize,
) -> Result<Vec<uuid::Uuid>, Box<dyn std::error::Error>> {
    schools::seed_schools(db, count).await
}

/// Seeds levels for existing schools
pub async fn seed_levels_only(
    db: &PgPool,
    school_ids: &[uuid::Uuid],
    levels_per_school: usize,
) -> Result<Vec<uuid::Uuid>, Box<dyn std::error::Error>> {
    levels::seed_levels(db, school_ids, levels_per_school).await
}

/// Seeds branches for existing levels
pub async fn seed_branches_only(
    db: &PgPool,
    level_ids: &[uuid::Uuid],
    branches_per_level: usize,
) -> Result<Vec<uuid::Uuid>, Box<dyn std::error::Error>> {
    branches::seed_branches(db, level_ids, branches_per_level).await
}

/// Seeds staff users for existing schools
pub async fn seed_staff_only(
    db: &PgPool,
    school_ids: &[uuid::Uuid],
    admins_per_school: usize,
    teachers_per_school: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let password_hash = hash_password()?;
    let user_roles = users::seed_staff_users(
        db,
        school_ids,
        admins_per_school,
        teachers_per_school,
        &password_hash,
    )
    .await?;
    users::assign_roles_batch(db, &user_roles).await?;
    Ok(())
}

/// Seeds students for existing branches
pub async fn seed_students_only(
    db: &PgPool,
    branches_with_context: &[(uuid::Uuid, uuid::Uuid, uuid::Uuid)],
    students_per_branch: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let password_hash = hash_password()?;
    let user_roles = users::seed_students(
        db,
        branches_with_context,
        students_per_branch,
        &password_hash,
    )
    .await?;
    users::assign_roles_batch(db, &user_roles).await?;
    Ok(())
}

/// Clears all seeded data from the database
pub async fn clear_all(db: &PgPool) -> Result<(), Box<dyn std::error::Error>> {
    let start_time = Instant::now();
    println!("🗑️  Clearing all seeded data...");

    // Order matters due to foreign keys: users -> branches -> levels -> schools
    users::clear_users(db).await?;
    branches::clear_branches(db).await?;
    levels::clear_levels(db).await?;
    schools::clear_schools(db).await?;

    println!("✅ All seeded data cleared in {:?}", start_time.elapsed());
    Ok(())
}

/// Clears only seeded users
pub async fn clear_users_only(db: &PgPool) -> Result<(), Box<dyn std::error::Error>> {
    users::clear_users(db).await?;
    Ok(())
}

/// Clears branches (and associated student assignments)
pub async fn clear_branches_only(db: &PgPool) -> Result<(), Box<dyn std::error::Error>> {
    branches::clear_branches(db).await?;
    Ok(())
}

/// Clears levels (cascades to branches)
pub async fn clear_levels_only(db: &PgPool) -> Result<(), Box<dyn std::error::Error>> {
    levels::clear_levels(db).await?;
    Ok(())
}

/// Clears schools (cascades to levels, branches)
pub async fn clear_schools_only(db: &PgPool) -> Result<(), Box<dyn std::error::Error>> {
    schools::clear_schools(db).await?;
    Ok(())
}

// Helper functions

fn hash_password() -> Result<String, Box<dyn std::error::Error>> {
    println!("🔐 Hashing password...");
    let start = Instant::now();
    // Use lower bcrypt cost for seeding (cost 4 = ~6ms vs cost 12 = ~250ms)
    let hash = hash("Password@123", 4).map_err(|e| format!("Failed to hash password: {}", e))?;
    println!("   ✓ Hashed password in {:?}", start.elapsed());
    Ok(hash)
}

/// Builds (branch_id, level_id, school_id) tuples for student assignment
fn build_branch_context(
    school_ids: &[uuid::Uuid],
    level_ids: &[uuid::Uuid],
    branch_ids: &[uuid::Uuid],
    levels_per_school: usize,
    branches_per_level: usize,
) -> Vec<(uuid::Uuid, uuid::Uuid, uuid::Uuid)> {
    let mut result = Vec::with_capacity(branch_ids.len());

    for (school_idx, &school_id) in school_ids.iter().enumerate() {
        let level_start = school_idx * levels_per_school;
        let level_end = level_start + levels_per_school;

        for (level_offset, &level_id) in level_ids[level_start..level_end].iter().enumerate() {
            let branch_start = (level_start + level_offset) * branches_per_level;
            let branch_end = branch_start + branches_per_level;

            for &branch_id in &branch_ids[branch_start..branch_end] {
                result.push((branch_id, level_id, school_id));
            }
        }
    }

    result
}
