use tower_governor::governor::{GovernorConfig, GovernorConfigBuilder};
use tower_governor::key_extractor::PeerIpKeyExtractor;

/// Rate limit configuration for the API
#[derive(Clone, Debug)]
pub struct RateLimitConfig {
    /// Requests per second for general endpoints
    pub general_per_second: u64,
    /// Burst size for general endpoints
    pub general_burst_size: u32,
    /// Requests per second for auth endpoints (stricter)
    pub auth_per_second: u64,
    /// Burst size for auth endpoints (stricter)
    pub auth_burst_size: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            general_per_second: 2,
            general_burst_size: 30,
            auth_per_second: 10,
            auth_burst_size: 5,
        }
    }
}

impl RateLimitConfig {
    pub fn from_env() -> Self {
        Self {
            general_per_second: std::env::var("RATE_LIMIT_GENERAL_PER_SECOND")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(2),
            general_burst_size: std::env::var("RATE_LIMIT_GENERAL_BURST_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(30),
            auth_per_second: std::env::var("RATE_LIMIT_AUTH_PER_SECOND")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10),
            auth_burst_size: std::env::var("RATE_LIMIT_AUTH_BURST_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(5),
        }
    }

    /// Create GovernorConfig for general API endpoints
    pub fn general_governor_config(
        &self,
    ) -> GovernorConfig<PeerIpKeyExtractor, ::governor::middleware::NoOpMiddleware> {
        GovernorConfigBuilder::default()
            .per_second(self.general_per_second)
            .burst_size(self.general_burst_size)
            .key_extractor(PeerIpKeyExtractor)
            .finish()
            .expect("Failed to build general rate limiter config")
    }

    /// Create GovernorConfig for auth endpoints (stricter limits)
    pub fn auth_governor_config(
        &self,
    ) -> GovernorConfig<PeerIpKeyExtractor, ::governor::middleware::NoOpMiddleware> {
        GovernorConfigBuilder::default()
            .per_second(self.auth_per_second)
            .burst_size(self.auth_burst_size)
            .key_extractor(PeerIpKeyExtractor)
            .finish()
            .expect("Failed to build auth rate limiter config")
    }
}
