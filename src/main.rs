use std::net::SocketAddr;

#[cfg(not(feature = "observability"))]
use tracing::info;

use crate::router::init_router;
use crate::state::init_app_state;
use dotenvy::dotenv;

mod config;
mod docs;
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

#[cfg(feature = "observability")]
async fn start_metrics_server(
    metrics_handle: chalkbyte_observability::PrometheusHandle,
    metrics_port: u16,
) {
    use chalkbyte_observability::metrics::metrics_app;

    let app = metrics_app(metrics_handle);

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

    #[cfg(feature = "observability")]
    {
        use chalkbyte_observability::{init_tracing, shutdown_tracer, is_observability_enabled, init_metrics};

        // Check if observability is enabled (default: true)
        let observability_enabled = is_observability_enabled();

        if observability_enabled {
            init_tracing();
        }

        // Initialize metrics only if observability is enabled
        let metrics_handle = if observability_enabled {
            init_metrics()
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
            shutdown_tracer().await;
        } else {
            println!("📴 Observability disabled (OBSERVABILITY_ENABLED=false)");
            start_main_server(state, port).await;
        }
    }

    #[cfg(not(feature = "observability"))]
    {
        eprintln!("⚠️  OBSERVABILITY IS DISABLED");
        eprintln!("   Observability (metrics, tracing) is not available.");
        eprintln!("   To enable, rebuild with: cargo build --features observability");
        eprintln!();
        
        let state = init_app_state().await;

        // Get the port from the environment variable, default to 3000 if not set
        let port = std::env::var("PORT")
            .ok()
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(3000);

        println!("🚀 Server running on http://localhost:{}", port);
        println!(
            "📚 Swagger UI available at http://localhost:{}/swagger-ui",
            port
        );
        println!("📖 Scalar UI available at http://localhost:{}/scalar", port);
        
        info!("Observability is disabled (compiled without observability feature). To enable, rebuild with: cargo build --features observability");

        start_main_server(state, port).await;
    }
}
