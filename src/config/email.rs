use std::env;

#[derive(Clone, Debug)]
pub struct EmailConfig {
    pub enabled: bool,
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    pub smtp_password: String,
    pub from_email: String,
    pub from_name: String,
    pub frontend_url: String,
}

impl EmailConfig {
    pub fn from_env() -> Self {
        Self {
            enabled: env::var("SMTP_ENABLED")
                .map(|v| v.to_lowercase() == "true" || v == "1")
                .unwrap_or(false),
            smtp_host: env::var("SMTP_HOST").unwrap_or_else(|_| "localhost".to_string()),
            smtp_port: env::var("SMTP_PORT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(1025),
            smtp_username: env::var("SMTP_USERNAME").unwrap_or_else(|_| "".to_string()),
            smtp_password: env::var("SMTP_PASSWORD").unwrap_or_else(|_| "".to_string()),
            from_email: env::var("FROM_EMAIL")
                .unwrap_or_else(|_| "noreply@chalkbyte.com".to_string()),
            from_name: env::var("FROM_NAME").unwrap_or_else(|_| "Chalkbyte".to_string()),
            frontend_url: env::var("FRONTEND_URL")
                .unwrap_or_else(|_| "http://localhost:3000".to_string()),
        }
    }
}
