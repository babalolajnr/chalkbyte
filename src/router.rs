use crate::docs::ApiDoc;
use crate::logging::is_observability_enabled;
use crate::logging::logging_middleware;
use crate::metrics::metrics_middleware;
use crate::middleware::role::require_admin;
use crate::modules::academic_sessions::router::init_academic_sessions_router;
use crate::modules::auth::router::init_auth_router;
use crate::modules::branches::router::{init_branches_router, init_level_branches_router};
use crate::modules::levels::router::init_levels_router;
use crate::modules::mfa::router::init_mfa_router;
use crate::modules::roles::router::{
    init_roles_router, init_user_permissions_router, init_user_roles_router,
};
use crate::modules::schools::router::init_schools_router;
use crate::modules::students::router::init_students_router;
use crate::modules::terms::router::{init_session_terms_router, init_terms_router};
use crate::modules::users::router::init_users_router;
use crate::state::AppState;

use axum::http::{HeaderValue, Method};
use axum::response::IntoResponse;
use axum::{Router, middleware};

use tower_http::LatencyUnit;
use tower_http::cors::CorsLayer;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing::Level;
use utoipa::OpenApi;
use utoipa_scalar::{Scalar, Servable as _};
use utoipa_swagger_ui::SwaggerUi;

async fn health_handler() -> impl IntoResponse {
    axum::Json(serde_json::json!({
        "status": "healthy",
        "service": "chalkbyte-api",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

/// Initialize router with rate limiting (for production use)
#[cfg(not(feature = "test-utils"))]
pub fn init_router(state: AppState) -> Router {
    use chalkbyte_cache::{CacheControlConfig, cache_control, etag_middleware};
    use tower_governor::GovernorLayer;

    // Create rate limiter configs
    let general_governor_config = state.rate_limit_config.general_governor_config();
    let auth_governor_config = state.rate_limit_config.auth_governor_config();
    let mfa_governor_config = state.rate_limit_config.auth_governor_config();

    // Cache-Control configurations
    let no_cache = cache_control(CacheControlConfig::no_store());
    let private_short = cache_control(CacheControlConfig::private(60).with_must_revalidate());
    let private_medium = cache_control(CacheControlConfig::private(300).with_must_revalidate());
    // For frequently changing data - always revalidate but still use ETags
    let revalidate_always = cache_control(CacheControlConfig::no_cache());

    let router = Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .merge(Scalar::with_url("/scalar", ApiDoc::openapi()))
        .route("/health", axum::routing::get(health_handler))
        .nest(
            "/api",
            Router::new()
                .nest(
                    "/users",
                    init_users_router()
                        .nest("/{user_id}/roles", init_user_roles_router())
                        .nest("/{user_id}/permissions", init_user_permissions_router())
                        .route_layer(middleware::from_fn_with_state(state.clone(), require_admin))
                        // Users list: private cache, short TTL with ETag
                        .layer(private_short.clone())
                        .layer(middleware::from_fn(etag_middleware)),
                )
                // Auth endpoints - no caching (sensitive)
                .nest(
                    "/auth",
                    init_auth_router()
                        .layer(GovernorLayer::new(auth_governor_config))
                        .layer(no_cache.clone()),
                )
                // MFA endpoints - no caching (sensitive)
                .nest(
                    "/mfa",
                    init_mfa_router()
                        .layer(GovernorLayer::new(mfa_governor_config))
                        .layer(no_cache.clone()),
                )
                .nest(
                    "/schools",
                    init_schools_router()
                        .route_layer(middleware::from_fn_with_state(state.clone(), require_admin))
                        // Schools: private cache, medium TTL with ETag
                        .layer(private_medium.clone())
                        .layer(middleware::from_fn(etag_middleware)),
                )
                .nest(
                    "/students",
                    init_students_router()
                        .route_layer(middleware::from_fn_with_state(state.clone(), require_admin))
                        // Students: private cache, short TTL
                        .layer(private_short.clone())
                        .layer(middleware::from_fn(etag_middleware)),
                )
                .nest(
                    "/levels",
                    init_levels_router()
                        .nest("/{level_id}/branches", init_level_branches_router())
                        .route_layer(middleware::from_fn_with_state(state.clone(), require_admin))
                        // Levels/branches: private cache, medium TTL
                        .layer(private_medium.clone())
                        .layer(middleware::from_fn(etag_middleware)),
                )
                .nest(
                    "/branches",
                    init_branches_router()
                        .route_layer(middleware::from_fn_with_state(state.clone(), require_admin))
                        // Branches: private cache, medium TTL
                        .layer(private_medium.clone())
                        .layer(middleware::from_fn(etag_middleware)),
                )
                // Roles and permissions endpoints
                .nest(
                    "/roles",
                    init_roles_router()
                        .route_layer(middleware::from_fn_with_state(state.clone(), require_admin))
                        // Roles: private cache, medium TTL (rarely changes)
                        .layer(private_medium.clone())
                        .layer(middleware::from_fn(etag_middleware)),
                )
                // Academic sessions and terms endpoints - use no-cache to always revalidate
                // since these are frequently modified and stale data causes UX issues
                .nest(
                    "/academic-sessions",
                    init_academic_sessions_router()
                        .nest("/{session_id}/terms", init_session_terms_router())
                        .route_layer(middleware::from_fn_with_state(state.clone(), require_admin))
                        .layer(revalidate_always.clone())
                        .layer(middleware::from_fn(etag_middleware)),
                )
                .nest(
                    "/terms",
                    init_terms_router()
                        .route_layer(middleware::from_fn_with_state(state.clone(), require_admin))
                        .layer(revalidate_always.clone())
                        .layer(middleware::from_fn(etag_middleware)),
                )
                // Apply general rate limiting to all API routes
                .layer(GovernorLayer::new(general_governor_config)),
        )
        .with_state(state.clone())
        .layer({
            let allowed_origins = state
                .cors_config
                .allowed_origins
                .iter()
                .filter_map(|origin| origin.parse().ok())
                .collect::<Vec<HeaderValue>>();

            CorsLayer::new()
                .allow_origin(allowed_origins)
                .allow_methods([
                    Method::GET,
                    Method::POST,
                    Method::PUT,
                    Method::PATCH,
                    Method::DELETE,
                    Method::OPTIONS,
                ])
                .allow_headers([
                    axum::http::header::AUTHORIZATION,
                    axum::http::header::CONTENT_TYPE,
                    axum::http::header::ACCEPT,
                    axum::http::header::IF_NONE_MATCH,
                ])
                .expose_headers([axum::http::header::ETAG, axum::http::header::CACHE_CONTROL])
                .allow_credentials(true)
        })
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(
                    DefaultMakeSpan::new()
                        .level(Level::INFO)
                        .include_headers(true),
                )
                .on_response(
                    DefaultOnResponse::new()
                        .level(Level::INFO)
                        .latency_unit(LatencyUnit::Millis)
                        .include_headers(true),
                ),
        );

    // Conditionally apply observability middleware
    if is_observability_enabled() {
        router
            .layer(middleware::from_fn(metrics_middleware))
            .layer(middleware::from_fn(logging_middleware))
    } else {
        router
    }
}

/// Initialize router without rate limiting (for tests)
#[cfg(feature = "test-utils")]
pub fn init_router(state: AppState) -> Router {
    use chalkbyte_cache::{CacheControlConfig, cache_control, etag_middleware};

    // Cache-Control configurations for tests
    let no_cache = cache_control(CacheControlConfig::no_store());
    let private_short = cache_control(CacheControlConfig::private(60).with_must_revalidate());
    let private_medium = cache_control(CacheControlConfig::private(300).with_must_revalidate());
    // For frequently changing data - always revalidate but still use ETags
    let revalidate_always = cache_control(CacheControlConfig::no_cache());

    let router = Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .merge(Scalar::with_url("/scalar", ApiDoc::openapi()))
        .route("/health", axum::routing::get(health_handler))
        .nest(
            "/api",
            Router::new()
                .nest(
                    "/users",
                    init_users_router()
                        .nest("/{user_id}/roles", init_user_roles_router())
                        .nest("/{user_id}/permissions", init_user_permissions_router())
                        .route_layer(middleware::from_fn_with_state(state.clone(), require_admin))
                        .layer(private_short.clone())
                        .layer(middleware::from_fn(etag_middleware)),
                )
                .nest("/auth", init_auth_router().layer(no_cache.clone()))
                .nest("/mfa", init_mfa_router().layer(no_cache.clone()))
                .nest(
                    "/schools",
                    init_schools_router()
                        .route_layer(middleware::from_fn_with_state(state.clone(), require_admin))
                        .layer(private_medium.clone())
                        .layer(middleware::from_fn(etag_middleware)),
                )
                .nest(
                    "/students",
                    init_students_router()
                        .route_layer(middleware::from_fn_with_state(state.clone(), require_admin))
                        .layer(private_short.clone())
                        .layer(middleware::from_fn(etag_middleware)),
                )
                .nest(
                    "/levels",
                    init_levels_router()
                        .nest("/{level_id}/branches", init_level_branches_router())
                        .route_layer(middleware::from_fn_with_state(state.clone(), require_admin))
                        .layer(private_medium.clone())
                        .layer(middleware::from_fn(etag_middleware)),
                )
                .nest(
                    "/branches",
                    init_branches_router()
                        .route_layer(middleware::from_fn_with_state(state.clone(), require_admin))
                        .layer(private_medium.clone())
                        .layer(middleware::from_fn(etag_middleware)),
                )
                .nest(
                    "/roles",
                    init_roles_router()
                        .route_layer(middleware::from_fn_with_state(state.clone(), require_admin))
                        .layer(private_medium.clone())
                        .layer(middleware::from_fn(etag_middleware)),
                )
                // Academic sessions and terms endpoints - use no-cache to always revalidate
                // since these are frequently modified and stale data causes UX issues
                .nest(
                    "/academic-sessions",
                    init_academic_sessions_router()
                        .nest("/{session_id}/terms", init_session_terms_router())
                        .route_layer(middleware::from_fn_with_state(state.clone(), require_admin))
                        .layer(revalidate_always.clone())
                        .layer(middleware::from_fn(etag_middleware)),
                )
                .nest(
                    "/terms",
                    init_terms_router()
                        .route_layer(middleware::from_fn_with_state(state.clone(), require_admin))
                        .layer(revalidate_always.clone())
                        .layer(middleware::from_fn(etag_middleware)),
                ),
        )
        .with_state(state.clone())
        .layer({
            let allowed_origins = state
                .cors_config
                .allowed_origins
                .iter()
                .filter_map(|origin| origin.parse().ok())
                .collect::<Vec<HeaderValue>>();

            CorsLayer::new()
                .allow_origin(allowed_origins)
                .allow_methods([
                    Method::GET,
                    Method::POST,
                    Method::PUT,
                    Method::PATCH,
                    Method::DELETE,
                    Method::OPTIONS,
                ])
                .allow_headers([
                    axum::http::header::AUTHORIZATION,
                    axum::http::header::CONTENT_TYPE,
                    axum::http::header::ACCEPT,
                    axum::http::header::IF_NONE_MATCH,
                ])
                .expose_headers([axum::http::header::ETAG, axum::http::header::CACHE_CONTROL])
                .allow_credentials(true)
        })
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(
                    DefaultMakeSpan::new()
                        .level(Level::INFO)
                        .include_headers(true),
                )
                .on_response(
                    DefaultOnResponse::new()
                        .level(Level::INFO)
                        .latency_unit(LatencyUnit::Millis)
                        .include_headers(true),
                ),
        );

    // Conditionally apply observability middleware
    if is_observability_enabled() {
        router
            .layer(middleware::from_fn(metrics_middleware))
            .layer(middleware::from_fn(logging_middleware))
    } else {
        router
    }
}
