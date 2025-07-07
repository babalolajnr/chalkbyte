use crate::db::init_app_state;
use crate::router::init_router;
use dotenvy::dotenv;

pub(crate) mod config;
pub(crate) mod db;
pub(crate) mod modules;
pub(crate) mod router;
pub(crate) mod utils;

#[tokio::main]
async fn main() {
    dotenv().ok();
    tracing_subscriber::fmt::init();

    let state = init_app_state().await;
    let app = init_router(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
