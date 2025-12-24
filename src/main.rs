use crate::router::init_router;
use crate::state::init_app_state;
use dotenvy::dotenv;

mod config;
mod docs;
mod logging;
mod middleware;
mod modules;
mod router;
mod state;
mod utils;
mod validator;

#[tokio::main]
async fn main() {
    dotenv().ok();

    logging::init_tracing();

    let state = init_app_state().await;
    let app = init_router(state);

    // Get the port from the environment variable, default to 3000 if not set
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(3000);

    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    println!("🚀 Server running on http://localhost:{}", port);
    println!(
        "📚 Swagger UI available at http://localhost:{}/swagger-ui",
        port
    );
    println!("📖 Scalar UI available at http://localhost:{}/scalar", port);
    println!("📊 Metrics available at http://localhost:{}/metrics", port);

    // Set up graceful shutdown
    let shutdown_signal = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install CTRL+C signal handler");
        println!("🛑 Shutting down gracefully...");
        logging::shutdown_tracer().await;
    };

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal)
        .await
        .unwrap();
}
