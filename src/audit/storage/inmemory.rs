use parking_lot::RwLock;

use super::super::model::AuditLogEntry;
use super::super::repository::AuditLogRepository;

#[derive(Debug, Default)]
pub struct InMemoryAuditLogRepository {
    // Append-only log, so a plain Vec (instead of a HashMap) is enough and
    // automatically preserves write order.
    entries: RwLock<Vec<AuditLogEntry>>,
}

impl InMemoryAuditLogRepository {
    pub fn new() -> Self {
        Self::default()
    }
}

impl AuditLogRepository for InMemoryAuditLogRepository {
    async fn record(&self, entry: AuditLogEntry) {
        self.entries.write().push(entry);
    }

    async fn list_by_tenant(&self, tenant_id: &str) -> Vec<AuditLogEntry> {
        // Reverse the insert order (instead of sorting by `at`) — `at` only
        // has second precision, so several fast sequential actions (create ->
        // update -> delete) can end up with the same timestamp and become
        // ambiguous if sorted by time. This Vec is append-only, so the insert
        // order itself is already chronological.
        self.entries
            .read()
            .iter()
            .rev()
            .filter(|entry| entry.tenant_id == tenant_id)
            .cloned()
            .collect()
    }
}
