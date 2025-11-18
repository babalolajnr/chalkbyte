use chalkbyte::cli::create_system_admin;
use chalkbyte::cli::seeder::{UsersPerSchool, clear_seeded_data, seed_database};
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
    /// Seed the database with fake schools and users
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

        /// Number of students per school
        #[arg(long, default_value = "20")]
        students: usize,
    },
    /// Clear all seeded data (keeps system admins)
    ClearSeed,
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    // Initialize database connection
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
            students,
        } => handle_seed(&pool, schools, admins, teachers, students).await,
        Commands::ClearSeed => handle_clear_seed(&pool).await,
    }
}

async fn handle_create_sysadmin(
    pool: &sqlx::postgres::PgPool,
    first_name: Option<String>,
    last_name: Option<String>,
    email: Option<String>,
    password: Option<String>,
) {
    // Use provided values or prompt interactively
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

    match create_system_admin(&pool, &first_name, &last_name, &email, &password).await {
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
    students: usize,
) {
    let users_per_school = UsersPerSchool {
        admins,
        teachers,
        students,
    };

    match seed_database(pool, schools, users_per_school).await {
        Ok(_) => {}
        Err(e) => {
            eprintln!("\n❌ Error seeding database: {}", e);
            std::process::exit(1);
        }
    }
}

async fn handle_clear_seed(pool: &sqlx::postgres::PgPool) {
    match clear_seeded_data(pool).await {
        Ok(_) => {}
        Err(e) => {
            eprintln!("\n❌ Error clearing seeded data: {}", e);
            std::process::exit(1);
        }
    }
}
