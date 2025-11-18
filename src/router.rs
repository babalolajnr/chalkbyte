use std::time::Duration;

use crate::docs::ApiDoc;
use crate::middleware::role::{require_admin, require_system_admin};
use crate::modules::auth::router::init_auth_router;
use crate::modules::mfa::router::init_mfa_router;
use crate::modules::schools::router::init_schools_router;
use crate::modules::students::router::init_students_router;
use crate::modules::users::router::init_users_router;
use crate::state::AppState;
use axum::body::Bytes;
use axum::extract::MatchedPath;
use axum::http::{HeaderValue, Method, Request};
use axum::response::Response;
use axum::{Router, http::HeaderMap, middleware};
use tower_http::{classify::ServerErrorsFailureClass, cors::CorsLayer, trace::TraceLayer};
use tracing::{Span, error, info, info_span, warn};
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
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|request: &Request<_>| {
                    let matched_path = request
                        .extensions()
                        .get::<MatchedPath>()
                        .map(MatchedPath::as_str);

                    info_span!(
                        "http_request",
                        method = ?request.method(),
                        matched_path,
                        request_id = uuid::Uuid::new_v4().to_string(),
                        status_code = tracing::field::Empty,
                        latency_ms = tracing::field::Empty,
                    )
                })
                .on_request(|request: &Request<_>, _span: &Span| {
                    info!(
                        uri = %request.uri(),
                        headers = ?request.headers(),
                        "Received HTTP request"
                    );
                })
                .on_response(|response: &Response<_>, latency: Duration, span: &Span| {
                    let status = response.status();
                    span.record("status_code", status.as_u16());
                    span.record("latency_ms", latency.as_millis());

                    info!(
                        status = %status,
                        latency_ms = %latency.as_millis(),
                        headers = ?response.headers(),
                        "Response sent"
                    );

                    if status.is_server_error() {
                        warn!("Server error response");
                    }
                })
                .on_body_chunk(|chunk: &Bytes, latency: Duration, _span: &Span| {
                    info!(
                        chunk_size = chunk.len(),
                        latency_ms = latency.as_millis(),
                        "Sent body chunk"
                    );
                })
                .on_eos(
                    |trailers: Option<&HeaderMap>, stream_duration: Duration, _span: &Span| {
                        if let Some(trailers) = trailers {
                            info!(
                                stream_duration_ms = stream_duration.as_millis(),
                                trailers = ?trailers,
                                "Stream closed with trailers"
                            );
                        } else {
                            info!(
                                stream_duration_ms = stream_duration.as_millis(),
                                "Stream closed"
                            );
                        }
                    },
                )
                .on_failure(
                    |error: ServerErrorsFailureClass, latency: Duration, span: &Span| {
                        span.record("status_code", 500);
                        span.record("latency_ms", latency.as_millis());

                        error!(
                            error = %error,
                            latency_ms = latency.as_millis(),
                            "Request failed"
                        );
                    },
                ),
        )
}
