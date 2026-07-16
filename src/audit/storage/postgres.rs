use sqlx::PgPool;
use sqlx::types::Json;

use crate::users::Actor;

use super::super::model::{
    AuditAction, AuditLogEntry, FieldChange, ResourceType,
};
use super::super::repository::AuditLogRepository;

#[derive(Debug, Clone, sqlx::FromRow)]
struct AuditLogRow {
    id: String,
    tenant_id: String,
    actor: Json<Actor>,
    action: String,
    resource_type: String,
    resource_id: String,
    label: String,
    changes: Json<Vec<FieldChange>>,
    at: i64,
}

impl From<AuditLogRow> for AuditLogEntry {
    fn from(row: AuditLogRow) -> Self {
        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            actor: row.actor.0,
            action: action_from_str(&row.action),
            resource_type: resource_type_from_str(&row.resource_type),
            resource_id: row.resource_id,
            label: row.label,
            changes: row.changes.0,
            at: row.at as u64,
        }
    }
}

/// Same reasoning as `role_to_str`/`role_from_str` in
/// `users::postgres` — kept in sync with `AuditAction`'s
/// `#[serde(rename_all = "snake_case")]` representation.
fn action_to_str(action: AuditAction) -> &'static str {
    match action {
        AuditAction::Created => "created",
        AuditAction::Updated => "updated",
        AuditAction::Deleted => "deleted",
    }
}

fn action_from_str(value: &str) -> AuditAction {
    match value {
        "created" => AuditAction::Created,
        "updated" => AuditAction::Updated,
        _ => AuditAction::Deleted,
    }
}

fn resource_type_to_str(resource_type: ResourceType) -> &'static str {
    match resource_type {
        ResourceType::Product => "product",
        ResourceType::Order => "order",
        ResourceType::Customer => "customer",
        ResourceType::Category => "category",
    }
}

fn resource_type_from_str(value: &str) -> ResourceType {
    match value {
        "product" => ResourceType::Product,
        "order" => ResourceType::Order,
        "customer" => ResourceType::Customer,
        _ => ResourceType::Category,
    }
}

/// Postgres-backed `AuditLogRepository`. Append-only, same as
/// `InMemoryAuditLogRepository` — there's no `update`/`delete` here at
/// all, matching the trait.
#[derive(Debug, Clone)]
pub struct PgAuditLogRepository {
    pool: PgPool,
}

impl PgAuditLogRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl AuditLogRepository for PgAuditLogRepository {
    async fn record(&self, entry: AuditLogEntry) {
        let _ = sqlx::query(
            "INSERT INTO audit_log \
             (id, tenant_id, actor, action, resource_type, resource_id, label, changes, at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(&entry.id)
        .bind(&entry.tenant_id)
        .bind(Json(entry.actor.clone()))
        .bind(action_to_str(entry.action))
        .bind(resource_type_to_str(entry.resource_type))
        .bind(&entry.resource_id)
        .bind(&entry.label)
        .bind(Json(entry.changes.clone()))
        .bind(entry.at as i64)
        .execute(&self.pool)
        .await;
    }

    async fn list_by_tenant(&self, tenant_id: &str) -> Vec<AuditLogEntry> {
        // `ORDER BY at DESC` is the database-level equivalent of the
        // in-memory version's `.rev()` — newest first. Ties (same-second
        // timestamps from rapid sequential actions) fall back to
        // insertion order isn't guaranteed by `at` alone in SQL, so this
        // is a best-effort match rather than a byte-for-byte guarantee;
        // acceptable since `at` is second-precision either way.
        sqlx::query_as::<_, AuditLogRow>(
            "SELECT id, tenant_id, actor, action, resource_type, resource_id, \
                    label, changes, at \
             FROM audit_log WHERE tenant_id = $1 ORDER BY at DESC",
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(Into::into)
        .collect()
    }
}
