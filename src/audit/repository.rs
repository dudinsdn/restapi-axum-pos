use std::future::Future;

use super::model::AuditLogEntry;

pub trait AuditLogRepository: Send + Sync + 'static {
    fn record(&self, entry: AuditLogEntry) -> impl Future<Output = ()> + Send;
    /// Newest first.
    fn list_by_tenant(
        &self,
        tenant_id: &str,
    ) -> impl Future<Output = Vec<AuditLogEntry>> + Send;
}
