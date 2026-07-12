use std::time::{SystemTime, UNIX_EPOCH};

use crate::users::Actor;

use super::model::{AuditAction, AuditLogEntry, ResourceType};
use super::repository::AuditLogRepository;

#[allow(clippy::too_many_arguments)]
pub async fn record<AR: AuditLogRepository>(
    audit: &AR,
    tenant_id: &str,
    actor: &Actor,
    action: AuditAction,
    resource_type: ResourceType,
    resource_id: &str,
    label: &str,
) {
    let entry = AuditLogEntry {
        id: format!("audit-{}", uuid::Uuid::new_v4().simple()),
        tenant_id: tenant_id.to_string(),
        actor: actor.clone(),
        action,
        resource_type,
        resource_id: resource_id.to_string(),
        label: label.to_string(),
        at: now_unix(),
    };

    audit.record(entry).await;
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}
