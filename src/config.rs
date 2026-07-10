/// Konfigurasi aplikasi, dibaca dari environment variable.
///
/// Dipisah dari `state.rs` supaya port/log-level bisa diubah tanpa
/// menyentuh apa pun yang berhubungan dengan storage.
#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub log_filter: String,
}

impl Config {
    pub fn from_env() -> Self {
        let port = std::env::var("PORT")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(3000);

        let log_filter = std::env::var("RUST_LOG")
            .unwrap_or_else(|_| "info,axum=debug".into());

        Self { port, log_filter }
    }
}
