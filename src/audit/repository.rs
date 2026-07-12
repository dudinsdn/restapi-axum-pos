use std::future::Future;

use parking_lot::RwLock;

use super::model::AuditLogEntry;

pub trait AuditLogRepository: Send + Sync + 'static {
    fn record(&self, entry: AuditLogEntry) -> impl Future<Output = ()> + Send;
    /// Terbaru duluan.
    fn list_by_tenant(
        &self,
        tenant_id: &str,
    ) -> impl Future<Output = Vec<AuditLogEntry>> + Send;
}

#[derive(Debug, Default)]
pub struct InMemoryAuditLogRepository {
    // Append-only log, jadi Vec biasa (bukan HashMap) sudah cukup dan
    // otomatis menjaga urutan waktu penulisan.
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
        // Reverse urutan insert (bukan sort by `at`) — `at` cuma presisi
        // detik, jadi beberapa aksi berurutan cepat (create -> update ->
        // delete) bisa dapat timestamp yang sama dan jadi ambigu kalau
        // di-sort by waktu. Vec ini append-only, jadi urutan insert-nya
        // sendiri sudah otomatis kronologis.
        self.entries
            .read()
            .iter()
            .rev()
            .filter(|entry| entry.tenant_id == tenant_id)
            .cloned()
            .collect()
    }
}
