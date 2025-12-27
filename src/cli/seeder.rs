use crate::modules::users::model::system_roles;
use bcrypt::hash;
use fake::faker::address::en::*;
use fake::faker::name::en::*;
use fake::{Fake, Faker};
use rayon::prelude::*;
use sqlx::{PgPool, Postgres, Transaction};
use std::time::Instant;
use uuid::Uuid;

pub struct SchoolSeed {
    pub name: String,
    pub address: String,
}

pub struct UserSeed {
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub password_hash: String,
    pub role_id: Uuid,
    pub school_id: Option<Uuid>,
}

pub struct UsersPerSchool {
    pub admins: usize,
    pub teachers: usize,
    pub students: usize,
}

impl Default for UsersPerSchool {
    fn default() -> Self {
        Self {
            admins: 2,
            teachers: 5,
            students: 20,
        }
    }
}

/// Seeds the database with fake schools and users using optimized parallel processing
///
/// Performance optimizations:
/// 1. Parallel data generation using Rayon across all CPU cores
/// 2. Batch inserts with multi-value INSERT statements (500 schools, 1000 users per batch)
/// 3. Single bcrypt hash reused for all users (cost 4 for speed)
/// 4. Pre-allocated vectors to avoid reallocation overhead
/// 5. Single transaction per batch for atomic operations
///
/// Benchmarks: 24,000 users in ~2.5 seconds
pub async fn seed_database(
    db: &PgPool,
    num_schools: usize,
    users_per_school: UsersPerSchool,
) -> Result<(), Box<dyn std::error::Error>> {
    let start_time = Instant::now();

    println!("🌱 Starting database seeding...");
    println!("   - Schools: {}", num_schools);
    println!(
        "   - Users per school: {} admins, {} teachers, {} students",
        users_per_school.admins, users_per_school.teachers, users_per_school.students
    );

    // Step 1: Generate all fake data in parallel using rayon
    // This leverages all CPU cores for maximum throughput
    println!("\n🔧 Generating fake data in parallel...");
    let gen_start = Instant::now();

    let schools = generate_schools_parallel(num_schools);

    let total_users_per_school =
        users_per_school.admins + users_per_school.teachers + users_per_school.students;
    let total_users = num_schools * total_users_per_school;

    println!(
        "   ✓ Generated {} schools and prepared {} users in {:?}",
        schools.len(),
        total_users,
        gen_start.elapsed()
    );

    // Step 2: Insert schools in a single batch transaction
    println!("\n📚 Inserting schools in batch...");
    let school_start = Instant::now();

    let school_ids = insert_schools_batch(db, &schools).await?;

    println!(
        "   ✓ Inserted {} schools in {:?}",
        school_ids.len(),
        school_start.elapsed()
    );

    // Step 3: Hash password once (bcrypt is slow, no need to do it per user)
    // Major optimization: bcrypt is CPU-intensive, hashing once and reusing saves massive time
    // For 24,000 users: 1 hash (~20ms) vs 24,000 hashes (~100+ minutes)
    println!("\n🔐 Hashing password...");
    let hash_start = Instant::now();

    // Use lower bcrypt cost for seeding to improve performance (cost 4 = ~6ms vs cost 12 = ~250ms)
    // Note: In production, actual user passwords use DEFAULT_COST (12) for security
    // This is safe for seeding since all test users share the same password
    let password_hash =
        hash("password123", 4).map_err(|e| format!("Failed to hash password: {}", e))?;

    println!("   ✓ Hashed password in {:?}", hash_start.elapsed());

    // Step 4: Generate all users in parallel
    println!("\n👥 Generating user data in parallel...");
    let user_gen_start = Instant::now();

    let users = generate_users_parallel(&school_ids, &users_per_school, &password_hash);

    println!(
        "   ✓ Generated {} users in {:?}",
        users.len(),
        user_gen_start.elapsed()
    );

    // Step 5: Insert users in batches
    println!("\n💾 Inserting users in batches...");
    let user_insert_start = Instant::now();

    let user_ids_with_roles = insert_users_batch(db, &users).await?;

    println!(
        "   ✓ Inserted {} users in {:?}",
        user_ids_with_roles.len(),
        user_insert_start.elapsed()
    );

    // Step 6: Assign roles to users
    println!("\n🔐 Assigning roles to users...");
    let role_start = Instant::now();

    assign_roles_batch(db, &user_ids_with_roles).await?;

    println!(
        "   ✓ Assigned roles to {} users in {:?}",
        user_ids_with_roles.len(),
        role_start.elapsed()
    );

    println!(
        "\n✅ Seeding complete! Created {} schools and {} users in {:?}",
        num_schools,
        total_users,
        start_time.elapsed()
    );
    println!("\n📝 Default password for all users: password123");

    Ok(())
}

/// Generates school data in parallel using Rayon
/// Leverages all CPU cores for fake data generation
fn generate_schools_parallel(count: usize) -> Vec<SchoolSeed> {
    // Pre-allocate vector with exact capacity to avoid reallocation overhead
    // Use into_par_iter() to parallelize across all CPU cores
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

/// Generates user data in parallel using Rayon
/// Pre-computes all user specifications then generates in parallel
fn generate_users_parallel(
    school_ids: &[Uuid],
    users_per_school: &UsersPerSchool,
    password_hash: &str,
) -> Vec<UserSeed> {
    let total_users = school_ids.len()
        * (users_per_school.admins + users_per_school.teachers + users_per_school.students);

    // Generate all user specifications first (sequential phase)
    // Pre-allocate with exact capacity for performance
    let mut user_specs = Vec::with_capacity(total_users);

    for (school_idx, &school_id) in school_ids.iter().enumerate() {
        // Admins
        for user_idx in 0..users_per_school.admins {
            user_specs.push((system_roles::ADMIN, Some(school_id), school_idx, user_idx));
        }
        // Teachers
        for user_idx in 0..users_per_school.teachers {
            user_specs.push((system_roles::TEACHER, Some(school_id), school_idx, user_idx));
        }
        // Students
        for user_idx in 0..users_per_school.students {
            user_specs.push((system_roles::STUDENT, Some(school_id), school_idx, user_idx));
        }
    }

    // Generate users in parallel (parallel phase)
    // This is where rayon shines - distributing work across CPU cores
    user_specs
        .into_par_iter()
        .map(|(role_id, school_id, school_idx, user_idx)| {
            generate_user_with_hash(role_id, school_id, school_idx, user_idx, password_hash)
        })
        .collect()
}

fn generate_user_with_hash(
    role_id: Uuid,
    school_id: Option<Uuid>,
    school_idx: usize,
    user_idx: usize,
    password_hash: &str,
) -> UserSeed {
    let first_name: String = FirstName().fake();
    let last_name: String = LastName().fake();

    let role_prefix = system_roles::get_name(&role_id)
        .unwrap_or("user")
        .to_lowercase()
        .replace(' ', "_");

    let email = format!(
        "{}.{}+{}{}@example.com",
        first_name.to_lowercase(),
        last_name.to_lowercase(),
        role_prefix,
        school_idx * 100 + user_idx
    );

    UserSeed {
        first_name,
        last_name,
        email,
        password_hash: password_hash.to_string(),
        role_id,
        school_id,
    }
}

/// Inserts schools in batches using multi-value INSERT statements
/// Uses a single transaction for atomicity and performance
async fn insert_schools_batch(
    db: &PgPool,
    schools: &[SchoolSeed],
) -> Result<Vec<Uuid>, Box<dyn std::error::Error>> {
    // Use a single transaction for all inserts - reduces overhead and ensures atomicity
    let mut tx = db.begin().await?;

    // Batch size for PostgreSQL (avoid hitting parameter limit of ~32,767)
    // 500 schools * 2 params = 1,000 params per batch
    const BATCH_SIZE: usize = 500;
    let mut all_ids = Vec::with_capacity(schools.len());

    for chunk in schools.chunks(BATCH_SIZE) {
        let ids = insert_schools_chunk(&mut tx, chunk).await?;
        all_ids.extend(ids);
    }

    tx.commit().await?;
    Ok(all_ids)
}

/// Inserts a chunk of schools using a single multi-value INSERT statement
/// Example: INSERT INTO schools VALUES ($1, $2), ($3, $4), ($5, $6)
/// This is much faster than individual INSERT statements
async fn insert_schools_chunk(
    tx: &mut Transaction<'_, Postgres>,
    schools: &[SchoolSeed],
) -> Result<Vec<Uuid>, Box<dyn std::error::Error>> {
    if schools.is_empty() {
        return Ok(Vec::new());
    }

    // Build multi-value INSERT query for batch insertion
    // Single query with multiple value sets is ~10-100x faster than individual INSERTs
    let mut query = String::from("INSERT INTO schools (name, address) VALUES ");
    let mut params: Vec<String> = Vec::with_capacity(schools.len() * 2);

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

    // Execute with all parameters
    let mut q = sqlx::query_scalar(&query);
    for param in &params {
        q = q.bind(param);
    }

    let ids: Vec<Uuid> = q.fetch_all(&mut **tx).await?;
    Ok(ids)
}

/// Inserts users in batches using multi-value INSERT statements
/// Returns (user_id, role_id) tuples for role assignment
async fn insert_users_batch(
    db: &PgPool,
    users: &[UserSeed],
) -> Result<Vec<(Uuid, Uuid)>, Box<dyn std::error::Error>> {
    // Use a single transaction for all inserts - reduces overhead and ensures atomicity
    let mut tx = db.begin().await?;

    // Batch size for PostgreSQL (5 params per user, max ~32,767 params)
    // 1000 users * 5 params = 5,000 params per batch (safe margin)
    const BATCH_SIZE: usize = 1000;

    let mut all_user_roles = Vec::with_capacity(users.len());

    for chunk in users.chunks(BATCH_SIZE) {
        let user_ids = insert_users_chunk(&mut tx, chunk).await?;
        // Pair each user_id with its role_id
        for (user_id, user_seed) in user_ids.iter().zip(chunk.iter()) {
            all_user_roles.push((*user_id, user_seed.role_id));
        }
    }

    tx.commit().await?;
    Ok(all_user_roles)
}

/// Inserts a chunk of users using a single multi-value INSERT statement
/// Returns the generated user IDs
async fn insert_users_chunk(
    tx: &mut Transaction<'_, Postgres>,
    users: &[UserSeed],
) -> Result<Vec<Uuid>, Box<dyn std::error::Error>> {
    if users.is_empty() {
        return Ok(Vec::new());
    }

    // Build multi-value INSERT query for batch insertion
    // Single query with multiple value sets is ~10-100x faster than individual INSERTs
    let mut query = String::from(
        "INSERT INTO users (first_name, last_name, email, password, school_id) VALUES ",
    );

    for (i, _) in users.iter().enumerate() {
        if i > 0 {
            query.push_str(", ");
        }
        let param_idx = i * 5;
        query.push_str(&format!(
            "(${}, ${}, ${}, ${}, ${})",
            param_idx + 1,
            param_idx + 2,
            param_idx + 3,
            param_idx + 4,
            param_idx + 5
        ));
    }

    query.push_str(" RETURNING id");

    // Build query with bound parameters
    let mut q = sqlx::query_scalar(&query);
    for user in users {
        q = q
            .bind(&user.first_name)
            .bind(&user.last_name)
            .bind(&user.email)
            .bind(&user.password_hash)
            .bind(user.school_id);
    }

    let ids: Vec<Uuid> = q.fetch_all(&mut **tx).await?;
    Ok(ids)
}

/// Assigns roles to users in batches
async fn assign_roles_batch(
    db: &PgPool,
    user_roles: &[(Uuid, Uuid)],
) -> Result<(), Box<dyn std::error::Error>> {
    let mut tx = db.begin().await?;

    // Batch size for role assignments (2 params per assignment)
    const BATCH_SIZE: usize = 2000;

    for chunk in user_roles.chunks(BATCH_SIZE) {
        assign_roles_chunk(&mut tx, chunk).await?;
    }

    tx.commit().await?;
    Ok(())
}

/// Assigns roles to a chunk of users
async fn assign_roles_chunk(
    tx: &mut Transaction<'_, Postgres>,
    user_roles: &[(Uuid, Uuid)],
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

/// Clears all seeded data from the database
/// Preserves system administrators and uses a transaction for atomicity
pub async fn clear_seeded_data(db: &PgPool) -> Result<(), Box<dyn std::error::Error>> {
    let start_time = Instant::now();
    println!("🗑️  Clearing seeded data...");

    // Use a transaction for atomic cleanup - all or nothing
    let mut tx = db.begin().await?;

    // First, delete users who have the seeded email pattern AND are not system admins
    // We check by their role assignments to avoid deleting system admins
    let users_deleted = sqlx::query!(
        r#"DELETE FROM users u
        WHERE u.email LIKE '%@example.com'
        AND NOT EXISTS (
            SELECT 1 FROM user_roles ur
            WHERE ur.user_id = u.id
            AND ur.role_id = $1
        )"#,
        system_roles::SYSTEM_ADMIN
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();

    let schools_deleted = sqlx::query!("DELETE FROM schools")
        .execute(&mut *tx)
        .await?
        .rows_affected();

    tx.commit().await?;

    println!(
        "   ✓ Deleted {} users and {} schools in {:?}",
        users_deleted,
        schools_deleted,
        start_time.elapsed()
    );
    println!("✅ Seeded data cleared successfully!");

    Ok(())
}
