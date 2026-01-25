use chalkbyte_cli::seeder::{self, LevelsPerSchool, SeedConfig, UsersPerSchool};
use chalkbyte_models::ids::{BranchId, LevelId, SchoolId};
use clap::{Parser, Subcommand};
use dialoguer::{Input, Password};
use dotenvy::dotenv;

#[derive(Parser)]
#[command(name = "chalkbyte-cli")]
#[command(about = "Chalkbyte CLI - Administrative tools for Chalkbyte", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new system administrator account
    CreateSysadmin {
        /// First name of the system admin
        #[arg(short = 'f', long)]
        first_name: Option<String>,

        /// Last name of the system admin
        #[arg(short = 'l', long)]
        last_name: Option<String>,

        /// Email address
        #[arg(short = 'e', long)]
        email: Option<String>,

        /// Password (will be prompted securely if not provided)
        #[arg(short = 'p', long)]
        password: Option<String>,
    },
    /// Seed the database with fake schools, levels, branches, and users
    Seed {
        /// Number of schools to create
        #[arg(short = 's', long, default_value = "5")]
        schools: usize,

        /// Number of admins per school
        #[arg(long, default_value = "2")]
        admins: usize,

        /// Number of teachers per school
        #[arg(long, default_value = "5")]
        teachers: usize,

        /// Number of levels (grades) per school
        #[arg(long, default_value = "6")]
        levels: usize,

        /// Number of branches (sections) per level
        #[arg(long, default_value = "3")]
        branches: usize,

        /// Number of students per branch
        #[arg(long, default_value = "25")]
        students: usize,
    },
    /// Seed only schools
    SeedSchools {
        /// Number of schools to create
        #[arg(short = 's', long, default_value = "5")]
        schools: usize,
    },
    /// Seed levels for existing schools
    SeedLevels {
        /// Number of levels per school
        #[arg(short = 'l', long, default_value = "6")]
        levels: usize,
    },
    /// Seed branches for existing levels
    SeedBranches {
        /// Number of branches per level
        #[arg(short = 'b', long, default_value = "3")]
        branches: usize,
    },
    /// Seed staff users (admins and teachers) for existing schools
    SeedStaff {
        /// Number of admins per school
        #[arg(long, default_value = "2")]
        admins: usize,

        /// Number of teachers per school
        #[arg(long, default_value = "5")]
        teachers: usize,
    },
    /// Seed students for existing branches
    SeedStudents {
        /// Number of students per branch
        #[arg(long, default_value = "25")]
        students: usize,
    },
    /// Clear all seeded data (keeps system admins)
    ClearSeed,
    /// Clear only seeded users
    ClearUsers,
    /// Clear only schools (cascades to levels, branches)
    ClearSchools,
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to database");

    let cli = Cli::parse();

    match cli.command {
        Commands::CreateSysadmin {
            first_name,
            last_name,
            email,
            password,
        } => handle_create_sysadmin(&pool, first_name, last_name, email, password).await,
        Commands::Seed {
            schools,
            admins,
            teachers,
            levels,
            branches,
            students,
        } => handle_seed(&pool, schools, admins, teachers, levels, branches, students).await,
        Commands::SeedSchools { schools } => handle_seed_schools(&pool, schools).await,
        Commands::SeedLevels { levels } => handle_seed_levels(&pool, levels).await,
        Commands::SeedBranches { branches } => handle_seed_branches(&pool, branches).await,
        Commands::SeedStaff { admins, teachers } => {
            handle_seed_staff(&pool, admins, teachers).await
        }
        Commands::SeedStudents { students } => handle_seed_students(&pool, students).await,
        Commands::ClearSeed => handle_clear_seed(&pool).await,
        Commands::ClearUsers => handle_clear_users(&pool).await,
        Commands::ClearSchools => handle_clear_schools(&pool).await,
    }
}

async fn handle_create_sysadmin(
    pool: &sqlx::postgres::PgPool,
    first_name: Option<String>,
    last_name: Option<String>,
    email: Option<String>,
    password: Option<String>,
) {
    let first_name = first_name.unwrap_or_else(|| {
        Input::new()
            .with_prompt("First name")
            .interact_text()
            .expect("Failed to read first name")
    });

    let last_name = last_name.unwrap_or_else(|| {
        Input::new()
            .with_prompt("Last name")
            .interact_text()
            .expect("Failed to read last name")
    });

    let email = email.unwrap_or_else(|| {
        Input::new()
            .with_prompt("Email address")
            .interact_text()
            .expect("Failed to read email")
    });

    let password = password.unwrap_or_else(|| {
        Password::new()
            .with_prompt("Password")
            .with_confirmation("Confirm password", "Passwords don't match")
            .interact()
            .expect("Failed to read password")
    });

    match create_system_admin_internal(pool, &first_name, &last_name, &email, &password).await {
        Ok(_) => {
            println!("\n✅ System admin created successfully!");
            println!("   Email: {}", email);
            println!("   Name: {} {}", first_name, last_name);
        }
        Err(e) => {
            eprintln!("\n❌ Error creating system admin: {}", e);
            std::process::exit(1);
        }
    }
}

async fn handle_seed(
    pool: &sqlx::postgres::PgPool,
    schools: usize,
    admins: usize,
    teachers: usize,
    levels: usize,
    branches: usize,
    students: usize,
) {
    let config = SeedConfig::new(schools)
        .with_users(UsersPerSchool { admins, teachers })
        .with_levels(LevelsPerSchool {
            count: levels,
            branches_per_level: branches,
            students_per_branch: students,
        });

    match seeder::seed_all(pool, config).await {
        Ok(_) => {}
        Err(e) => {
            eprintln!("\n❌ Error seeding database: {}", e);
            std::process::exit(1);
        }
    }
}

async fn handle_seed_schools(pool: &sqlx::postgres::PgPool, schools: usize) {
    match seeder::seed_schools_only(pool, schools).await {
        Ok(ids) => {
            println!("✅ Created {} schools", ids.len());
        }
        Err(e) => {
            eprintln!("\n❌ Error seeding schools: {}", e);
            std::process::exit(1);
        }
    }
}

async fn handle_seed_levels(pool: &sqlx::postgres::PgPool, levels_per_school: usize) {
    // Get all existing schools
    let school_uuids: Vec<uuid::Uuid> =
        sqlx::query_scalar!("SELECT id FROM schools ORDER BY created_at")
            .fetch_all(pool)
            .await
            .expect("Failed to fetch schools");

    if school_uuids.is_empty() {
        eprintln!("❌ No schools found. Run `seed-schools` first.");
        std::process::exit(1);
    }

    let school_ids: Vec<SchoolId> = school_uuids.into_iter().map(SchoolId::from).collect();
    match seeder::seed_levels_only(pool, &school_ids, levels_per_school).await {
        Ok(ids) => {
            println!("✅ Created {} levels", ids.len());
        }
        Err(e) => {
            eprintln!("\n❌ Error seeding levels: {}", e);
            std::process::exit(1);
        }
    }
}

async fn handle_seed_branches(pool: &sqlx::postgres::PgPool, branches_per_level: usize) {
    // Get all existing levels
    let level_uuids: Vec<uuid::Uuid> =
        sqlx::query_scalar!("SELECT id FROM levels ORDER BY school_id, name")
            .fetch_all(pool)
            .await
            .expect("Failed to fetch levels");

    if level_uuids.is_empty() {
        eprintln!("❌ No levels found. Run `seed-levels` first.");
        std::process::exit(1);
    }

    let level_ids: Vec<LevelId> = level_uuids.into_iter().map(LevelId::from).collect();
    match seeder::seed_branches_only(pool, &level_ids, branches_per_level).await {
        Ok(ids) => {
            println!("✅ Created {} branches", ids.len());
        }
        Err(e) => {
            eprintln!("\n❌ Error seeding branches: {}", e);
            std::process::exit(1);
        }
    }
}

async fn handle_seed_staff(
    pool: &sqlx::postgres::PgPool,
    admins_per_school: usize,
    teachers_per_school: usize,
) {
    // Get all existing schools
    let school_uuids: Vec<uuid::Uuid> =
        sqlx::query_scalar!("SELECT id FROM schools ORDER BY created_at")
            .fetch_all(pool)
            .await
            .expect("Failed to fetch schools");

    if school_uuids.is_empty() {
        eprintln!("❌ No schools found. Run `seed-schools` first.");
        std::process::exit(1);
    }

    let school_ids: Vec<SchoolId> = school_uuids.into_iter().map(SchoolId::from).collect();
    match seeder::seed_staff_only(pool, &school_ids, admins_per_school, teachers_per_school).await {
        Ok(_) => {
            let total = school_ids.len() * (admins_per_school + teachers_per_school);
            println!("✅ Created {} staff users", total);
        }
        Err(e) => {
            eprintln!("\n❌ Error seeding staff: {}", e);
            std::process::exit(1);
        }
    }
}

async fn handle_seed_students(pool: &sqlx::postgres::PgPool, students_per_branch: usize) {
    // Get all branches with their level and school context
    let rows = sqlx::query!(
        r#"
        SELECT b.id as branch_id, b.level_id, l.school_id
        FROM branches b
        JOIN levels l ON l.id = b.level_id
        ORDER BY l.school_id, l.name, b.name
        "#
    )
    .fetch_all(pool)
    .await
    .expect("Failed to fetch branches");

    if rows.is_empty() {
        eprintln!("❌ No branches found. Run `seed-branches` first.");
        std::process::exit(1);
    }

    let branches_with_context: Vec<(BranchId, LevelId, SchoolId)> = rows
        .iter()
        .map(|r| {
            (
                BranchId::from(r.branch_id),
                LevelId::from(r.level_id),
                SchoolId::from(r.school_id),
            )
        })
        .collect();

    match seeder::seed_students_only(pool, &branches_with_context, students_per_branch).await {
        Ok(_) => {
            let total = branches_with_context.len() * students_per_branch;
            println!("✅ Created {} students", total);
        }
        Err(e) => {
            eprintln!("\n❌ Error seeding students: {}", e);
            std::process::exit(1);
        }
    }
}

async fn handle_clear_seed(pool: &sqlx::postgres::PgPool) {
    match seeder::clear_all(pool).await {
        Ok(_) => {}
        Err(e) => {
            eprintln!("\n❌ Error clearing seeded data: {}", e);
            std::process::exit(1);
        }
    }
}

async fn handle_clear_users(pool: &sqlx::postgres::PgPool) {
    match seeder::clear_users_only(pool).await {
        Ok(_) => {
            println!("✅ Cleared seeded users");
        }
        Err(e) => {
            eprintln!("\n❌ Error clearing users: {}", e);
            std::process::exit(1);
        }
    }
}

async fn handle_clear_schools(pool: &sqlx::postgres::PgPool) {
    match seeder::clear_schools_only(pool).await {
        Ok(_) => {
            println!("✅ Cleared all schools (and associated levels, branches)");
        }
        Err(e) => {
            eprintln!("\n❌ Error clearing schools: {}", e);
            std::process::exit(1);
        }
    }
}

/// Creates a system administrator account (internal CLI function).
///
/// This is the internal implementation used by the CLI. The library version
/// is now removed to keep lib.rs minimal (exports only seeder module).
async fn create_system_admin_internal(
    db: &sqlx::postgres::PgPool,
    first_name: &str,
    last_name: &str,
    email: &str,
    password: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use chalkbyte_core::hash_password;
    use chalkbyte_models::users::system_roles;

    let hashed_password =
        hash_password(password).map_err(|e| format!("Failed to hash password: {}", e.error))?;

    // Start a transaction
    let mut tx = db.begin().await?;

    // Insert the user
    let user_id = sqlx::query_scalar::<_, uuid::Uuid>(
        "INSERT INTO users (first_name, last_name, email, password, school_id)
         VALUES ($1, $2, $3, $4, NULL)
         ON CONFLICT (email) DO NOTHING
         RETURNING id",
    )
    .bind(first_name)
    .bind(last_name)
    .bind(email)
    .bind(&hashed_password)
    .fetch_optional(&mut *tx)
    .await?;

    let user_id = match user_id {
        Some(id) => id,
        None => {
            tx.rollback().await?;
            return Err("User with this email already exists".into());
        }
    };

    // Assign the system admin role
    sqlx::query(
        "INSERT INTO user_roles (user_id, role_id)
         VALUES ($1, $2)
         ON CONFLICT (user_id, role_id) DO NOTHING",
    )
    .bind(user_id)
    .bind(system_roles::SYSTEM_ADMIN)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(())
}
