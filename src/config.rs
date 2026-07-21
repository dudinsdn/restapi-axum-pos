/// Application configuration, read from environment variables.
#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub jwt_secret: String,
    /// Postgres connection string — required. There's no in-memory
    /// fallback: a single, always-tested backend is worth more than the
    /// convenience of running without a database, since an in-memory
    /// implementation that's never actually exercised by the same tests
    /// that guard the code shipped to production isn't confidence, it's
    /// just a second codebase to keep in sync by hand.
    pub database_url: String,
}

impl Config {
    pub fn from_env() -> Self {
        let port = std::env::var("PORT")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(3000);

        let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| {
            tracing::warn!(
                "JWT_SECRET is not set — use the default secret, which is \
                ONLY secure for development. It MUST be set via an \
                environment variable before deploying to production, \
                otherwise all tokens can be forged."
            );
            "dev-only-insecure-secret-change-me".to_string()
        });

        let database_url = std::env::var("DATABASE_URL")
            .ok()
            .filter(|url| !url.trim().is_empty())
            .unwrap_or_else(|| {
                panic!(
                    "DATABASE_URL is required (e.g. \
                     postgres://user:pass@localhost:5432/dbname) — see \
                     env.example. There is no in-memory fallback."
                )
            });

        Self {
            port,
            jwt_secret,
            database_url,
        }
    }
}
