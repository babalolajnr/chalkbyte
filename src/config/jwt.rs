use std::env;

#[derive(Clone, Debug)]
pub struct JwtConfig {
    pub secret: String,
    pub access_token_expiry: i64,
    pub refresh_token_expiry: i64,
}

impl JwtConfig {
    pub fn from_env() -> Self {
        Self {
            secret: env::var("JWT_SECRET")
                .unwrap_or_else(|_| "your-secret-key-change-in-production".to_string()),
            access_token_expiry: env::var("JWT_ACCESS_EXPIRY")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(3600), // 1 hour
            refresh_token_expiry: env::var("JWT_REFRESH_EXPIRY")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(604800), // 7 days
        }
    }
}
