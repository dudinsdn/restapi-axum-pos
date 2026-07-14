use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use parking_lot::RwLock;

const MAX_LOGIN_ATTEMPTS: u32 = 5;
const LOGIN_WINDOW: Duration = Duration::from_secs(15 * 60);

/// Limit failed login attempts per email, so it can't be brute-forced.
/// Intentionally keyed per email (not per-IP) — simpler and still
/// protects a single account from repeated password guessing, regardless
/// of the attacker's IP.
#[derive(Default)]
pub struct LoginRateLimiter {
    attempts: RwLock<HashMap<String, (u32, Instant)>>,
}

impl LoginRateLimiter {
    pub fn new() -> Self {
        Self::default()
    }

    /// `true` if login attempts are still allowed.
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

/// List of tokens (via `jti`) that have been manually logged out before
/// naturally expiring. Simple (in-memory `HashSet`, no automatic cleanup
/// for entries whose token has already expired) — good enough for the
/// current scale, but if logout volume gets high, consider storing this
/// revocation list in persistent storage (e.g. Redis with TTL).
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
