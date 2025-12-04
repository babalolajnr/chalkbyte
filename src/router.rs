use crate::docs::ApiDoc;
use crate::logging::logging_middleware;
use crate::middleware::role::{require_admin, require_system_admin};
use crate::modules::auth::router::init_auth_router;
use crate::modules::branches::router::{init_branches_router, init_level_branches_router};
use crate::modules::levels::router::init_levels_router;
use crate::modules::mfa::router::init_mfa_router;
use crate::modules::schools::router::init_schools_router;
use crate::modules::students::router::init_students_router;
use crate::modules::users::router::init_users_router;
use crate::state::AppState;
use axum::http::{HeaderValue, Method};
use axum::{Router, middleware};
use tower_http::cors::CorsLayer;
use utoipa::OpenApi;
use utoipa_scalar::{Scalar, Servable as _};
use utoipa_swagger_ui::SwaggerUi;

pub fn init_router(state: AppState) -> Router {
    Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .merge(Scalar::with_url("/scalar", ApiDoc::openapi()))
        .nest(
            "/api",
            Router::new()
                .nest(
                    "/users",
                    init_users_router()
                        .route_layer(middleware::from_fn_with_state(state.clone(), require_admin)),
                )
                .nest("/auth", init_auth_router())
                .nest("/mfa", init_mfa_router())
                .nest(
                    "/schools",
                    init_schools_router().route_layer(middleware::from_fn_with_state(
                        state.clone(),
                        require_system_admin,
                    )),
                )
                .nest(
                    "/students",
                    init_students_router()
                        .route_layer(middleware::from_fn_with_state(state.clone(), require_admin)),
                )
                .nest(
                    "/levels",
                    init_levels_router()
                        .nest("/{level_id}/branches", init_level_branches_router())
                        .route_layer(middleware::from_fn_with_state(state.clone(), require_admin)),
                )
                .nest(
                    "/branches",
                    init_branches_router()
                        .route_layer(middleware::from_fn_with_state(state.clone(), require_admin)),
                ),
        )
        .with_state(state.clone())
        .layer({
            let allowed_origins: Vec<HeaderValue> = state
                .cors_config
                .allowed_origins
                .iter()
                .filter_map(|origin| origin.parse().ok())
                .collect();

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
                ])
                .allow_credentials(true)
        })
        .layer(middleware::from_fn(logging_middleware))
}
