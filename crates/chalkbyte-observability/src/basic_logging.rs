use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

/// Initialize basic console logging when observability feature is disabled.
///
/// This provides a minimal but functional logging setup that enables console output
/// for all tracing macros (info!, warn!, error!, debug!, etc.) throughout the application.
///
/// # Configuration
///
/// - **Log Level**: Controlled by `LOG_LEVEL` environment variable (default: "info")
/// - **Filtering**: Noisy dependencies filtered to warn level for cleaner output
/// - **Format**: Compact format with ISO 8601 timestamps and ANSI colors (auto-detected)
/// - **Target**: Shows module paths (e.g., "chalkbyte::modules::schools")
pub fn init_basic_console_logging() {
    // Determine log level from environment variable
    let log_level = std::env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string());

    // Create environment filter with default log level and suppressed noisy deps
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new(format!(
            "{}={},tower_http=warn,hyper=warn,tonic=warn,h2=warn,sqlx=warn",
            env!("CARGO_PKG_NAME"),
            log_level
        ))
    });

    // Create console layer with compact formatting and colors
    let console_layer = fmt::layer()
        .compact()
        .with_target(true)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_file(true)
        .with_line_number(true)
        .with_ansi(true)
        .with_filter(env_filter);

    // Initialize the tracing subscriber with console output only
    tracing_subscriber::registry().with(console_layer).init();

    // Print initialization message to stderr (bypasses logging system)
    eprintln!(
        "ℹ️  Observability disabled - console logging only (OBSERVABILITY_ENABLED=false or feature not compiled)"
    );
}
