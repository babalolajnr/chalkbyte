//! HTTP caching middleware for ETag and Cache-Control headers.
//!
//! This module provides middleware for HTTP-level caching using:
//! - `Cache-Control` headers for controlling client/proxy caching
//! - `ETag` headers for conditional requests (If-None-Match)
//!
//! # Example
//!
//! ```ignore
//! use chalkbyte_cache::middleware::{cache_control, etag_middleware};
//! use axum::Router;
//!
//! let app = Router::new()
//!     .route("/api/public", get(handler))
//!     .layer(cache_control(CacheControlConfig::public(300)))
//!     .layer(axum::middleware::from_fn(etag_middleware));
//! ```

use axum::{
    body::Body,
    extract::Request,
    http::{
        HeaderValue, StatusCode,
        header::{CACHE_CONTROL, ETAG, IF_NONE_MATCH},
    },
    middleware::Next,
    response::{IntoResponse, Response},
};
use http_body_util::BodyExt;
use sha2::{Digest, Sha256};
use std::time::Duration;
use tower_http::set_header::SetResponseHeaderLayer;

/// Configuration for Cache-Control header.
#[derive(Debug, Clone)]
pub struct CacheControlConfig {
    /// Whether the response can be cached by any cache (public) or only by browser (private).
    pub public: bool,
    /// Maximum age in seconds the response is considered fresh.
    pub max_age: u64,
    /// Whether the cache must revalidate with the origin server before using a stale response.
    pub must_revalidate: bool,
    /// Whether the response should not be cached at all.
    pub no_cache: bool,
    /// Whether the response should not be stored at all.
    pub no_store: bool,
    /// Shared cache max age (for CDNs/proxies).
    pub s_maxage: Option<u64>,
    /// Stale-while-revalidate directive.
    pub stale_while_revalidate: Option<u64>,
}

impl CacheControlConfig {
    /// Create a public cache configuration.
    ///
    /// # Arguments
    ///
    /// * `max_age` - Maximum age in seconds
    pub fn public(max_age: u64) -> Self {
        Self {
            public: true,
            max_age,
            must_revalidate: false,
            no_cache: false,
            no_store: false,
            s_maxage: None,
            stale_while_revalidate: None,
        }
    }

    /// Create a private cache configuration (browser-only).
    ///
    /// # Arguments
    ///
    /// * `max_age` - Maximum age in seconds
    pub fn private(max_age: u64) -> Self {
        Self {
            public: false,
            max_age,
            must_revalidate: false,
            no_cache: false,
            no_store: false,
            s_maxage: None,
            stale_while_revalidate: None,
        }
    }

    /// Create a no-cache configuration (always revalidate).
    pub fn no_cache() -> Self {
        Self {
            public: false,
            max_age: 0,
            must_revalidate: true,
            no_cache: true,
            no_store: false,
            s_maxage: None,
            stale_while_revalidate: None,
        }
    }

    /// Create a no-store configuration (never cache).
    pub fn no_store() -> Self {
        Self {
            public: false,
            max_age: 0,
            must_revalidate: false,
            no_cache: true,
            no_store: true,
            s_maxage: None,
            stale_while_revalidate: None,
        }
    }

    /// Set must-revalidate directive.
    pub fn with_must_revalidate(mut self) -> Self {
        self.must_revalidate = true;
        self
    }

    /// Set s-maxage for shared caches (CDNs/proxies).
    pub fn with_s_maxage(mut self, seconds: u64) -> Self {
        self.s_maxage = Some(seconds);
        self
    }

    /// Set stale-while-revalidate directive.
    pub fn with_stale_while_revalidate(mut self, seconds: u64) -> Self {
        self.stale_while_revalidate = Some(seconds);
        self
    }

    /// Build the Cache-Control header value.
    pub fn to_header_value(&self) -> HeaderValue {
        let mut directives = Vec::new();

        if self.no_store {
            directives.push("no-store".to_string());
        }

        if self.no_cache {
            directives.push("no-cache".to_string());
        }

        if !self.no_store && !self.no_cache {
            if self.public {
                directives.push("public".to_string());
            } else {
                directives.push("private".to_string());
            }

            directives.push(format!("max-age={}", self.max_age));

            if let Some(s_maxage) = self.s_maxage {
                directives.push(format!("s-maxage={}", s_maxage));
            }

            if let Some(swr) = self.stale_while_revalidate {
                directives.push(format!("stale-while-revalidate={}", swr));
            }
        }

        if self.must_revalidate {
            directives.push("must-revalidate".to_string());
        }

        HeaderValue::from_str(&directives.join(", "))
            .unwrap_or_else(|_| HeaderValue::from_static("no-cache"))
    }
}

impl Default for CacheControlConfig {
    fn default() -> Self {
        Self::no_cache()
    }
}

/// Helper struct to generate Cache-Control header values.
#[derive(Clone)]
pub struct CacheControlMakeHeader(HeaderValue);

impl<B> tower_http::set_header::MakeHeaderValue<Response<B>> for CacheControlMakeHeader {
    fn make_header_value(&mut self, _message: &Response<B>) -> Option<HeaderValue> {
        Some(self.0.clone())
    }
}

/// Create a Cache-Control layer with the given configuration.
///
/// # Example
///
/// ```ignore
/// use chalkbyte_cache::middleware::{cache_control, CacheControlConfig};
///
/// let layer = cache_control(CacheControlConfig::public(300));
/// ```
pub fn cache_control(config: CacheControlConfig) -> SetResponseHeaderLayer<CacheControlMakeHeader> {
    let header_value = config.to_header_value();
    SetResponseHeaderLayer::if_not_present(CACHE_CONTROL, CacheControlMakeHeader(header_value))
}

/// Create a Cache-Control layer from duration.
///
/// # Example
///
/// ```ignore
/// use chalkbyte_cache::middleware::cache_control_duration;
/// use std::time::Duration;
///
/// let layer = cache_control_duration(Duration::from_secs(300), true);
/// ```
pub fn cache_control_duration(
    duration: Duration,
    public: bool,
) -> SetResponseHeaderLayer<CacheControlMakeHeader> {
    let config = if public {
        CacheControlConfig::public(duration.as_secs())
    } else {
        CacheControlConfig::private(duration.as_secs())
    };
    cache_control(config)
}

/// Generate an ETag from response body bytes.
fn generate_etag(body: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(body);
    let hash = hasher.finalize();
    format!("\"{}\"", hex::encode(&hash[..16]))
}

/// Weak ETag comparison (ignores the W/ prefix).
fn etags_match(client_etag: &str, server_etag: &str) -> bool {
    let client = client_etag.trim().trim_start_matches("W/");
    let server = server_etag.trim().trim_start_matches("W/");
    client == server
}

/// ETag middleware for conditional GET requests.
///
/// This middleware:
/// 1. Generates an ETag based on response body hash
/// 2. Compares with `If-None-Match` header from client
/// 3. Returns 304 Not Modified if ETags match
///
/// # Note
///
/// This middleware buffers the entire response body, so it's best suited
/// for smaller responses. For large responses, consider using pre-computed ETags.
///
/// # Example
///
/// ```ignore
/// use chalkbyte_cache::middleware::etag_middleware;
/// use axum::{Router, middleware};
///
/// let app = Router::new()
///     .route("/api/data", get(handler))
///     .layer(middleware::from_fn(etag_middleware));
/// ```
pub async fn etag_middleware(request: Request, next: Next) -> Response {
    let if_none_match = request
        .headers()
        .get(IF_NONE_MATCH)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let response = next.run(request).await;

    // Only process successful GET responses
    if !response.status().is_success() {
        return response;
    }

    // Skip if response already has an ETag
    if response.headers().contains_key(ETAG) {
        if let Some(client_etag) = if_none_match {
            if let Some(server_etag) = response.headers().get(ETAG).and_then(|v| v.to_str().ok()) {
                if etags_match(&client_etag, server_etag) {
                    return StatusCode::NOT_MODIFIED.into_response();
                }
            }
        }
        return response;
    }

    // Collect the response body
    let (parts, body) = response.into_parts();

    let bytes = match body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(_) => return (parts, Body::empty()).into_response(),
    };

    // Generate ETag
    let etag = generate_etag(&bytes);

    // Check If-None-Match
    if let Some(client_etag) = if_none_match {
        // Handle multiple ETags in If-None-Match (comma-separated)
        let matches = client_etag
            .split(',')
            .any(|tag| etags_match(tag.trim(), &etag));

        if matches {
            return StatusCode::NOT_MODIFIED.into_response();
        }
    }

    // Build response with ETag header
    let mut response = Response::from_parts(parts, Body::from(bytes));
    if let Ok(etag_value) = HeaderValue::from_str(&etag) {
        response.headers_mut().insert(ETAG, etag_value);
    }

    response
}

/// Configuration for cacheable routes.
#[derive(Debug, Clone)]
pub struct CacheableRoute {
    /// Route path pattern.
    pub path: String,
    /// Cache control configuration.
    pub config: CacheControlConfig,
    /// Whether to enable ETag generation.
    pub etag: bool,
}

impl CacheableRoute {
    /// Create a new cacheable route configuration.
    pub fn new(path: impl Into<String>, config: CacheControlConfig) -> Self {
        Self {
            path: path.into(),
            config,
            etag: true,
        }
    }

    /// Disable ETag generation for this route.
    pub fn without_etag(mut self) -> Self {
        self.etag = false;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_control_public() {
        let config = CacheControlConfig::public(300);
        let header = config.to_header_value();
        assert_eq!(header.to_str().unwrap(), "public, max-age=300");
    }

    #[test]
    fn test_cache_control_private() {
        let config = CacheControlConfig::private(60);
        let header = config.to_header_value();
        assert_eq!(header.to_str().unwrap(), "private, max-age=60");
    }

    #[test]
    fn test_cache_control_no_store() {
        let config = CacheControlConfig::no_store();
        let header = config.to_header_value();
        assert!(header.to_str().unwrap().contains("no-store"));
    }

    #[test]
    fn test_cache_control_with_s_maxage() {
        let config = CacheControlConfig::public(300).with_s_maxage(600);
        let header = config.to_header_value();
        let value = header.to_str().unwrap();
        assert!(value.contains("max-age=300"));
        assert!(value.contains("s-maxage=600"));
    }

    #[test]
    fn test_etag_generation() {
        let body = b"test response body";
        let etag = generate_etag(body);
        assert!(etag.starts_with('"'));
        assert!(etag.ends_with('"'));
    }

    #[test]
    fn test_etags_match() {
        assert!(etags_match("\"abc123\"", "\"abc123\""));
        assert!(etags_match("W/\"abc123\"", "\"abc123\""));
        assert!(etags_match("\"abc123\"", "W/\"abc123\""));
        assert!(!etags_match("\"abc123\"", "\"xyz789\""));
    }
}
