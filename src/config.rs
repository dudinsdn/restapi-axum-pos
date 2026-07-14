/// Application configuration, read from environment variables.
#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub jwt_secret: String,
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

        Self { port, jwt_secret }
    }
}
