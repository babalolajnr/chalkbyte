use crate::router::init_router;
use crate::state::init_app_state;
use dotenvy::dotenv;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

mod config;
mod docs;
mod middleware;
mod modules;
mod router;
mod state;
mod utils;
mod validator;

#[tokio::main]
async fn main() {
    dotenv().ok();

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
