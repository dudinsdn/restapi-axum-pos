/// Application configuration, read from environment variables.
#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub jwt_secret: String,
    /// If set, the app connects to this Postgres instance and runs
    /// migrations on startup instead of using the in-memory repositories.
    /// See `main.rs` for how the choice is made — everything downstream
    /// (handlers, services) is unaffected either way, since both options
    /// implement the same repository traits.
    pub database_url: Option<String>,
}

impl Config {
    pub fn from_env() -> Self {
        let port = std::env::var("PORT")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(3000);

        let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| {
            tracing::warn!(
                "JWT_SECRET tidak di-set — pakai secret default yang HANYA aman untuk \
                 development. WAJIB di-set lewat environment variable sebelum deploy ke \
                 production, kalau tidak semua token bisa dipalsukan."
            );
            "dev-only-insecure-secret-change-me".to_string()
        });

        let database_url = std::env::var("DATABASE_URL").ok().filter(|url| {
            if url.trim().is_empty() {
                tracing::warn!(
                    "DATABASE_URL is set but empty — falling back to the \
                     in-memory repositories."
                );
                false
            } else {
                true
            }
        });

        if database_url.is_none() {
            tracing::info!(
                "DATABASE_URL not set — using in-memory repositories. Data \
                 will NOT survive a restart. Set DATABASE_URL to a Postgres \
                 connection string to persist data instead."
            );
        }

        Self {
            port,
            jwt_secret,
            database_url,
        }
    }
}
