use std::collections::HashMap;
use std::time::{Duration, Instant};

use parking_lot::RwLock;

use super::model::Order;

/// How long a completed order is remembered for a given idempotency key
/// before it's evicted and the key can be reused for a genuinely new order.
const IDEMPOTENCY_TTL: Duration = Duration::from_secs(24 * 60 * 60);

/// Caches the result of `POST /orders` per `(tenant_id, Idempotency-Key)`,
/// so a client retrying the same request (e.g. after a network timeout, or
/// a cashier double-tapping "submit") gets back the SAME order instead of
/// creating a duplicate one and double-reserving stock.
///
/// Simple in-memory store (`HashMap` behind a `RwLock`, entries only
/// evicted lazily on read past their TTL) — good enough for a single
/// instance at the current scale, same tradeoff as `TokenRevocationList`.
/// If this needs to survive a restart or work across multiple instances,
/// back it with Redis (`SET NX ... EX <ttl>` is the standard pattern for
/// this exact problem).
#[derive(Default)]
pub struct IdempotencyStore {
    entries: RwLock<HashMap<(String, String), (Instant, Order)>>,
}

impl IdempotencyStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the order already created for this key, if any and not yet
    /// past its TTL.
    pub fn get(&self, tenant_id: &str, key: &str) -> Option<Order> {
        let entries = self.entries.read();
        entries
            .get(&(tenant_id.to_string(), key.to_string()))
            .filter(|(created_at, _)| created_at.elapsed() < IDEMPOTENCY_TTL)
            .map(|(_, order)| order.clone())
    }

    pub fn put(&self, tenant_id: &str, key: &str, order: Order) {
        self.entries.write().insert(
            (tenant_id.to_string(), key.to_string()),
            (Instant::now(), order),
        );
    }
}
