use rayon::prelude::*;
use sqlx::{PgPool, Postgres, Transaction};
use std::time::Instant;
use uuid::Uuid;

use super::models::BranchSeed;

const BRANCH_NAMES: [&str; 10] = ["A", "B", "C", "D", "E", "F", "G", "H", "I", "J"];

/// Generates branch data for levels
pub fn generate_branches(level_ids: &[Uuid], branches_per_level: usize) -> Vec<BranchSeed> {
    level_ids
        .par_iter()
        .flat_map(|&level_id| {
            (0..branches_per_level)
                .map(|i| {
                    let name = if i < BRANCH_NAMES.len() {
                        BRANCH_NAMES[i].to_string()
                    } else {
                        format!("Section {}", i + 1)
                    };

                    BranchSeed {
                        name,
                        description: None,
                        level_id,
                    }
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Seeds branches into the database for given levels
pub async fn seed_branches(
    db: &PgPool,
    level_ids: &[Uuid],
    branches_per_level: usize,
) -> Result<Vec<Uuid>, Box<dyn std::error::Error>> {
    let start_time = Instant::now();
    let total_branches = level_ids.len() * branches_per_level;
    println!(
        "🌿 Seeding {} branches ({} per level)...",
        total_branches, branches_per_level
    );

    let branches = generate_branches(level_ids, branches_per_level);
    let branch_ids = insert_branches_batch(db, &branches).await?;

    println!(
        "   ✓ Inserted {} branches in {:?}",
        branch_ids.len(),
        start_time.elapsed()
    );

    Ok(branch_ids)
}

/// Inserts branches in batches
pub async fn insert_branches_batch(
    db: &PgPool,
    branches: &[BranchSeed],
) -> Result<Vec<Uuid>, Box<dyn std::error::Error>> {
    let mut tx = db.begin().await?;

    const BATCH_SIZE: usize = 500;
    let mut all_ids = Vec::with_capacity(branches.len());

    for chunk in branches.chunks(BATCH_SIZE) {
        let ids = insert_branches_chunk(&mut tx, chunk).await?;
        all_ids.extend(ids);
    }

    tx.commit().await?;
    Ok(all_ids)
}

async fn insert_branches_chunk(
    tx: &mut Transaction<'_, Postgres>,
    branches: &[BranchSeed],
) -> Result<Vec<Uuid>, Box<dyn std::error::Error>> {
    if branches.is_empty() {
        return Ok(Vec::new());
    }

    let mut query = String::from("INSERT INTO branches (name, description, level_id) VALUES ");

    for (i, _) in branches.iter().enumerate() {
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
    for branch in branches {
        q = q
            .bind(&branch.name)
            .bind(&branch.description)
            .bind(branch.level_id);
    }

    let ids: Vec<Uuid> = q.fetch_all(&mut **tx).await?;
    Ok(ids)
}

/// Clears all branches from the database
pub async fn clear_branches(db: &PgPool) -> Result<u64, Box<dyn std::error::Error>> {
    let start_time = Instant::now();
    println!("🗑️  Clearing branches...");

    let result = sqlx::query!("DELETE FROM branches")
        .execute(db)
        .await?
        .rows_affected();

    println!(
        "   ✓ Deleted {} branches in {:?}",
        result,
        start_time.elapsed()
    );

    Ok(result)
}

/// Gets all branch IDs for a specific level
pub async fn get_branches_for_level(
    db: &PgPool,
    level_id: Uuid,
) -> Result<Vec<Uuid>, Box<dyn std::error::Error>> {
    let ids = sqlx::query_scalar!(
        r#"SELECT id FROM branches WHERE level_id = $1 ORDER BY name"#,
        level_id
    )
    .fetch_all(db)
    .await?;

    Ok(ids)
}

/// Gets all branch IDs grouped by level for a school
pub async fn get_branches_by_level_for_school(
    db: &PgPool,
    school_id: Uuid,
) -> Result<Vec<(Uuid, Uuid)>, Box<dyn std::error::Error>> {
    let rows = sqlx::query!(
        r#"
        SELECT b.id as branch_id, b.level_id
        FROM branches b
        JOIN levels l ON l.id = b.level_id
        WHERE l.school_id = $1
        ORDER BY l.name, b.name
        "#,
        school_id
    )
    .fetch_all(db)
    .await?;

    Ok(rows.iter().map(|r| (r.branch_id, r.level_id)).collect())
}
