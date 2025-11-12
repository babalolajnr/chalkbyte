use chalkbyte::cli::create_system_admin;
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
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    let cli = Cli::parse();

    match cli.command {
        Commands::CreateSysadmin {
            first_name,
            last_name,
            email,
            password,
        } => handle_create_sysadmin(first_name, last_name, email, password).await,
    }
}

async fn handle_create_sysadmin(
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

    // Initialize database connection
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to database");

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
