//! School seeding functionality.
//!
//! Provides functions for generating and inserting fake school data
//! into the database.

use fake::faker::address::en::*;
use fake::{Fake, Faker};
use rayon::prelude::*;
use sqlx::{PgPool, Postgres, Transaction};
use std::time::Instant;
use uuid::Uuid;

use super::models::SchoolSeed;

/// Generates school data in parallel using Rayon
pub fn generate_schools(count: usize) -> Vec<SchoolSeed> {
    (0..count)
        .into_par_iter()
        .map(|_| {
            let city: String = CityName().fake();
            let street: String = StreetName().fake();
            let building: String = BuildingNumber().fake();
            let state: String = StateAbbr().fake();
            let zip: String = ZipCode().fake();

            SchoolSeed {
                name: format!("{} {} School", city, Faker.fake::<String>()),
                address: format!("{} {}, {}, {} {}", building, street, city, state, zip),
            }
        })
        .collect()
}

/// Seeds schools into the database
pub async fn seed_schools(
    db: &PgPool,
    count: usize,
) -> Result<Vec<Uuid>, Box<dyn std::error::Error>> {
    let start_time = Instant::now();
    println!("📚 Seeding {} schools...", count);

    let schools = generate_schools(count);
    let school_ids = insert_schools_batch(db, &schools).await?;

    println!(
        "   ✓ Inserted {} schools in {:?}",
        school_ids.len(),
        start_time.elapsed()
    );

    Ok(school_ids)
}

/// Inserts schools in batches using multi-value INSERT statements
pub async fn insert_schools_batch(
    db: &PgPool,
    schools: &[SchoolSeed],
) -> Result<Vec<Uuid>, Box<dyn std::error::Error>> {
    let mut tx = db.begin().await?;

    const BATCH_SIZE: usize = 500;
    let mut all_ids = Vec::with_capacity(schools.len());

    for chunk in schools.chunks(BATCH_SIZE) {
        let ids = insert_schools_chunk(&mut tx, chunk).await?;
        all_ids.extend(ids);
    }

    tx.commit().await?;
    Ok(all_ids)
}

async fn insert_schools_chunk(
    tx: &mut Transaction<'_, Postgres>,
    schools: &[SchoolSeed],
) -> Result<Vec<Uuid>, Box<dyn std::error::Error>> {
    if schools.is_empty() {
        return Ok(Vec::new());
    }

    let mut query = String::from("INSERT INTO schools (name, address) VALUES ");
    let mut params = Vec::with_capacity(schools.len() * 2);

    for (i, school) in schools.iter().enumerate() {
        if i > 0 {
            query.push_str(", ");
        }
        let param_idx = i * 2;
        query.push_str(&format!("(${}, ${})", param_idx + 1, param_idx + 2));
        params.push(school.name.clone());
        params.push(school.address.clone());
    }

    query.push_str(" RETURNING id");

    let mut q = sqlx::query_scalar(&query);
    for param in &params {
        q = q.bind(param);
    }

    let ids = q.fetch_all(&mut **tx).await?;
    Ok(ids)
}

/// Clears all schools from the database
pub async fn clear_schools(db: &PgPool) -> Result<u64, Box<dyn std::error::Error>> {
    let start_time = Instant::now();
    println!("🗑️  Clearing schools...");

    let result = sqlx::query!("DELETE FROM schools")
        .execute(db)
        .await?
        .rows_affected();

    println!(
        "   ✓ Deleted {} schools in {:?}",
        result,
        start_time.elapsed()
    );

    Ok(result)
}
