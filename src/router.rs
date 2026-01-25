#[cfg(feature = "observability")]
use chalkbyte_observability::{logging_middleware, metrics_middleware, is_observability_enabled};
#[cfg(not(feature = "observability"))]
use crate::middleware::observability_stubs::{logging_middleware, metrics_middleware, is_observability_enabled};
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
use std::path::PathBuf;

use tower_http::LatencyUnit;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing::Level;
#[cfg(feature = "scalar")]
use crate::docs::ApiDoc;
#[cfg(feature = "scalar")]
use utoipa::OpenApi;
#[cfg(feature = "scalar")]
use utoipa_scalar::{Scalar, Servable as _};

async fn health_handler() -> impl IntoResponse {
    axum::Json(serde_json::json!({
        "status": "healthy",
        "service": "chalkbyte-api",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

/// Builds the API router with all routes and middleware (shared between prod and test)
#[allow(unused_variables)]
fn build_api_router(state: AppState, apply_rate_limiting: bool) -> Router {
    use chalkbyte_cache::{CacheControlConfig, cache_control, etag_middleware};
    #[cfg(not(test))]
    use tower_governor::GovernorLayer;

    // Cache-Control configurations
    let no_cache = cache_control(CacheControlConfig::no_store());
    let private_short = cache_control(CacheControlConfig::private(60).with_must_revalidate());
    let private_medium = cache_control(CacheControlConfig::private(300).with_must_revalidate());
    // For frequently changing data - always revalidate but still use ETags
    let revalidate_always = cache_control(CacheControlConfig::no_cache());

    let api_routes = Router::new()
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
            {
                let auth_router = init_auth_router().layer(no_cache.clone());
                #[cfg(not(test))]
                {
                    if apply_rate_limiting {
                        let auth_governor_config = state.rate_limit_config.auth_governor_config();
                        auth_router.layer(GovernorLayer::new(auth_governor_config))
                    } else {
                        auth_router
                    }
                }
                #[cfg(test)]
                {
                    auth_router
                }
            },
        )
        // MFA endpoints - no caching (sensitive)
        .nest(
            "/mfa",
            {
                let mfa_router = init_mfa_router().layer(no_cache.clone());
                #[cfg(not(test))]
                {
                    if apply_rate_limiting {
                        let mfa_governor_config = state.rate_limit_config.auth_governor_config();
                        mfa_router.layer(GovernorLayer::new(mfa_governor_config))
                    } else {
                        mfa_router
                    }
                }
                #[cfg(test)]
                {
                    mfa_router
                }
            },
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
        );

    // Apply general rate limiting to all API routes (production only)
    #[cfg(not(test))]
    let api_routes = if apply_rate_limiting {
        let general_governor_config = state.rate_limit_config.general_governor_config();
        api_routes.layer(GovernorLayer::new(general_governor_config))
    } else {
        api_routes
    };

    #[cfg(feature = "scalar")]
    let router = Router::new()
        .merge(Scalar::with_url("/scalar", ApiDoc::openapi()))
        .route("/health", axum::routing::get(health_handler))
        .nest("/api", api_routes)
        .nest_service("/files", ServeDir::new(PathBuf::from("./uploads")))
        .with_state(state.clone());

    #[cfg(not(feature = "scalar"))]
    let router = Router::new()
        .route("/health", axum::routing::get(health_handler))
        .nest("/api", api_routes)
        .nest_service("/files", ServeDir::new(PathBuf::from("./uploads")))
        .with_state(state.clone());

    let router = router
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

/// Initialize router with rate limiting (for production use)
#[cfg(not(test))]
pub fn init_router(state: AppState) -> Router {
    build_api_router(state, true)
}

/// Initialize router without rate limiting (for tests)
#[cfg(test)]
pub fn init_router(state: AppState) -> Router {
    build_api_router(state, false)
}
