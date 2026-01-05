use std::net::SocketAddr;

use crate::router::init_router;
use crate::state::init_app_state;
use dotenvy::dotenv;

mod config;
mod docs;
mod logging;
mod metrics;
mod middleware;
mod modules;
mod router;
mod state;
mod utils;
mod validator;

async fn start_main_server(state: state::AppState, port: u16) {
    let app = init_router(state);

    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();

    println!("🚀 Server running on http://localhost:{}", port);
    println!(
        "📚 Swagger UI available at http://localhost:{}/swagger-ui",
        port
    );
    println!("📖 Scalar UI available at http://localhost:{}/scalar", port);

    let shutdown_signal = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install CTRL+C signal handler");
        println!("🛑 Shutting down main server gracefully...");
    };

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal)
    .await
    .unwrap();
}

async fn start_metrics_server(
    metrics_handle: metrics_exporter_prometheus::PrometheusHandle,
    metrics_port: u16,
) {
    let app = metrics::metrics_app(metrics_handle);

    let addr = format!("0.0.0.0:{}", metrics_port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();

    println!(
        "📊 Metrics server running on http://localhost:{}/metrics",
        metrics_port
    );

    let shutdown_signal = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install CTRL+C signal handler");
        println!("🛑 Shutting down metrics server gracefully...");
    };

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal)
        .await
        .unwrap();
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    // Check if observability is enabled (default: true)
    let observability_enabled = std::env::var("OBSERVABILITY_ENABLED")
        .map(|v| v.to_lowercase() != "false" && v != "0")
        .unwrap_or(true);

    if observability_enabled {
        logging::init_tracing();
    }

    // Initialize metrics only if observability is enabled
    let metrics_handle = if observability_enabled {
        metrics::init_metrics()
    } else {
        None
    };

    let state = init_app_state().await;

    // Get the port from the environment variable, default to 3000 if not set
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(3000);

    // Get metrics port from environment variable, default to 3001
    let metrics_port = std::env::var("METRICS_PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(3001);

    // Start servers based on observability configuration
    if let Some(handle) = metrics_handle {
        // Start both servers concurrently
        // The metrics endpoint runs on a separate port and should not be publicly exposed
        let (_main, _metrics) = tokio::join!(
            start_main_server(state, port),
            start_metrics_server(handle, metrics_port)
        );

        // Shutdown tracing
        logging::shutdown_tracer().await;
    } else {
        println!("📴 Observability disabled (OBSERVABILITY_ENABLED=false)");
        start_main_server(state, port).await;
    }
}
