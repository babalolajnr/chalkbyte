//! Level (grade) seeding functionality.
//!
//! Provides functions for generating and inserting fake level data
//! into the database.

use chalkbyte_models::{LevelId, SchoolId};
use rayon::prelude::*;
use sqlx::{PgPool, Postgres, Transaction};
use std::time::Instant;

use super::models::LevelSeed;

const LEVEL_NAMES: [&str; 12] = [
    "Grade 1", "Grade 2", "Grade 3", "Grade 4", "Grade 5", "Grade 6", "Grade 7", "Grade 8",
    "Grade 9", "Grade 10", "Grade 11", "Grade 12",
];

/// Generates level data for schools
pub fn generate_levels(school_ids: &[SchoolId], levels_per_school: usize) -> Vec<LevelSeed> {
    school_ids
        .par_iter()
        .flat_map(|&school_id| {
            (0..levels_per_school)
                .map(|i| {
                    let name = if i < LEVEL_NAMES.len() {
                        LEVEL_NAMES[i].to_string()
                    } else {
                        format!("Grade {}", i + 1)
                    };

                    LevelSeed {
                        name,
                        description: None,
                        school_id,
                    }
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Seeds levels into the database for given schools
pub async fn seed_levels(
    db: &PgPool,
    school_ids: &[SchoolId],
    levels_per_school: usize,
) -> Result<Vec<LevelId>, Box<dyn std::error::Error>> {
    let start_time = Instant::now();
    let total_levels = school_ids.len() * levels_per_school;
    println!(
        "📊 Seeding {} levels ({} per school)...",
        total_levels, levels_per_school
    );

    let levels = generate_levels(school_ids, levels_per_school);
    let level_ids = insert_levels_batch(db, &levels).await?;

    println!(
        "   ✓ Inserted {} levels in {:?}",
        level_ids.len(),
        start_time.elapsed()
    );

    Ok(level_ids)
}

/// Inserts levels in batches
pub async fn insert_levels_batch(
    db: &PgPool,
    levels: &[LevelSeed],
) -> Result<Vec<LevelId>, Box<dyn std::error::Error>> {
    let mut tx = db.begin().await?;

    const BATCH_SIZE: usize = 500;
    let mut all_ids = Vec::with_capacity(levels.len());

    for chunk in levels.chunks(BATCH_SIZE) {
        let ids = insert_levels_chunk(&mut tx, chunk).await?;
        all_ids.extend(ids);
    }

    tx.commit().await?;
    Ok(all_ids)
}

async fn insert_levels_chunk(
    tx: &mut Transaction<'_, Postgres>,
    levels: &[LevelSeed],
) -> Result<Vec<LevelId>, Box<dyn std::error::Error>> {
    if levels.is_empty() {
        return Ok(Vec::new());
    }

    let mut query = String::from("INSERT INTO levels (name, description, school_id) VALUES ");

    for (i, _) in levels.iter().enumerate() {
        if i > 0 {
            query.push_str(", ");
        }
        let param_idx = i * 3;
        query.push_str(&format!(
            "(${}, ${}, ${})",
            param_idx + 1,
            param_idx + 2,
            param_idx + 3
        ));
    }

    query.push_str(" RETURNING id");

    let mut q = sqlx::query_scalar(&query);
    for level in levels {
        q = q
            .bind(&level.name)
            .bind(&level.description)
            .bind(level.school_id);
    }

    let ids: Vec<LevelId> = q.fetch_all(&mut **tx).await?;
    Ok(ids)
}

/// Clears all levels from the database
pub async fn clear_levels(db: &PgPool) -> Result<u64, Box<dyn std::error::Error>> {
    let start_time = Instant::now();
    println!("🗑️  Clearing levels...");

    let result = sqlx::query!("DELETE FROM levels")
        .execute(db)
        .await?
        .rows_affected();

    println!(
        "   ✓ Deleted {} levels in {:?}",
        result,
        start_time.elapsed()
    );

    Ok(result)
}

/// Gets all level IDs for a specific school
#[allow(dead_code)]
pub async fn get_levels_for_school(
    db: &PgPool,
    school_id: SchoolId,
) -> Result<Vec<LevelId>, Box<dyn std::error::Error>> {
    let ids: Vec<LevelId> = sqlx::query_scalar!(
        r#"SELECT id FROM levels WHERE school_id = $1 ORDER BY name"#,
        school_id as SchoolId
    )
    .fetch_all(db)
    .await?
    .into_iter()
    .map(LevelId::from)
    .collect();

    Ok(ids)
}
