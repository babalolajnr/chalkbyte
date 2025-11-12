use crate::db::init_app_state;
use crate::router::init_router;
use dotenvy::dotenv;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

pub(crate) mod cli;
pub(crate) mod config;
pub(crate) mod db;
pub(crate) mod docs;
pub(crate) mod middleware;
pub(crate) mod modules;
pub(crate) mod router;
pub(crate) mod utils;
pub mod validator;

#[tokio::main]
async fn main() {
    dotenv().ok();

    let args: Vec<String> = std::env::args().collect();

    // Check if this is a CLI command
    if args.len() > 1 && args[1] == "create-sysadmin" {
        handle_create_sysadmin(args).await;
        return;
    }

    // Normal server startup
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                // axum logs rejections from built-in extractors with the `axum::rejection`
                // target, at `TRACE` level. `axum::rejection=trace` enables showing those events
                format!(
                    "{}=debug,tower_http=debug,axum::rejection=trace",
                    env!("CARGO_CRATE_NAME")
                )
                .into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let state = init_app_state().await;
    let app = init_router(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("🚀 Server running on http://localhost:3000");
    println!("📚 Swagger UI available at http://localhost:3000/swagger-ui");
    println!("📖 Scalar UI available at http://localhost:3000/scalar");
    axum::serve(listener, app).await.unwrap();
}

async fn handle_create_sysadmin(args: Vec<String>) {
    if args.len() != 6 {
        eprintln!(
            "Usage: {} create-sysadmin <first_name> <last_name> <email> <password>",
            args[0]
        );
        std::process::exit(1);
    }

    let first_name = &args[2];
    let last_name = &args[3];
    let email = &args[4];
    let password = &args[5];

    // Initialize database connection
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to database");

    match cli::create_system_admin(&pool, first_name, last_name, email, password).await {
        Ok(_) => {
            println!("✅ System admin created successfully!");
            println!("   Email: {}", email);
            println!("   Name: {} {}", first_name, last_name);
        }
        Err(e) => {
            eprintln!("❌ Error creating system admin: {}", e);
            std::process::exit(1);
        }
    }
}
