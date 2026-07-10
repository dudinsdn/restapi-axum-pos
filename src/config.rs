/// Configuration loaded from environment variables
#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub rust_log: String,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            port: std::env::var("PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(3000),
            rust_log: std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "info,axum=debug".into()),
        }
    }
}
