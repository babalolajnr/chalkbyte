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

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("🚀 Server running on http://localhost:3000");
    println!("📚 Swagger UI available at http://localhost:3000/swagger-ui");
    println!("📖 Scalar UI available at http://localhost:3000/scalar");
    axum::serve(listener, app).await.unwrap();
}
