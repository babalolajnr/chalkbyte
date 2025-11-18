use std::env;

#[derive(Clone, Debug)]
pub struct CorsConfig {
    pub allowed_origins: Vec<String>,
}

impl CorsConfig {
    pub fn from_env() -> Self {
        let allowed_origins = env::var("ALLOWED_ORIGINS")
            .unwrap_or_else(|_| "http://localhost:3000,http://localhost:5173".to_string())
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        Self { allowed_origins }
    }
}
