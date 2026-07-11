use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use parking_lot::RwLock;

const MAX_LOGIN_ATTEMPTS: u32 = 5;
const LOGIN_WINDOW: Duration = Duration::from_secs(15 * 60);

/// Batasi percobaan login yang gagal per email, supaya tidak bisa
/// di-brute-force. Sengaja di-key per email (bukan per-IP) — lebih
/// sederhana dan tetap melindungi satu akun dari tebakan password
/// bertubi-tubi, terlepas dari IP penyerangnya.
#[derive(Default)]
pub struct LoginRateLimiter {
    attempts: RwLock<HashMap<String, (u32, Instant)>>,
}

impl LoginRateLimiter {
    pub fn new() -> Self {
        Self::default()
    }

    /// `true` kalau masih boleh coba login.
    pub fn check(&self, key: &str) -> bool {
        match self.attempts.read().get(key) {
            Some((count, started)) if started.elapsed() < LOGIN_WINDOW => {
                *count < MAX_LOGIN_ATTEMPTS
            }
            _ => true,
        }
    }

    pub fn record_failure(&self, key: &str) {
        let mut attempts = self.attempts.write();
        let entry = attempts
            .entry(key.to_string())
            .or_insert((0, Instant::now()));

        if entry.1.elapsed() >= LOGIN_WINDOW {
            *entry = (0, Instant::now());
        }
        entry.0 += 1;
    }

    pub fn reset(&self, key: &str) {
        self.attempts.write().remove(key);
    }
}

/// Daftar token (via `jti`) yang sudah di-logout secara manual sebelum
/// expired secara alami. Sederhana (in-memory `HashSet`, tidak ada
/// pembersihan otomatis untuk entry yang sudah lewat masa berlaku token-nya)
/// — cukup untuk skala sekarang, tapi kalau volume logout tinggi, pertimbangkan
/// simpan revocation list ini di storage yang persist (mis. Redis dengan TTL).
#[derive(Default)]
pub struct TokenRevocationList {
    revoked: RwLock<HashSet<String>>,
}

impl TokenRevocationList {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn revoke(&self, jti: &str) {
        self.revoked.write().insert(jti.to_string());
    }

    pub fn is_revoked(&self, jti: &str) -> bool {
        self.revoked.read().contains(jti)
    }
}
