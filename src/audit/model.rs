use serde::{Deserialize, Serialize};

use crate::users::Actor;

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditAction {
    Created,
    Updated,
    Deleted,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceType {
    Product,
    Order,
    Customer,
    Category,
}

/// A single field that changed during an update: value before & after.
/// Stored in structured form (not just concatenated into text) so the client
/// can display it in any format without needing to parse a string.
///
/// Derives `Deserialize` (unlike `AuditAction`/`ResourceType`, which are
/// stored as plain `TEXT` with a manual mapping in `audit::postgres`)
/// because `Vec<FieldChange>` round-trips through a JSONB column as-is —
/// see `PgAuditLogRepository`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldChange {
    pub field: String,
    pub old_value: String,
    pub new_value: String,
}

/// A single history entry: who, did what, to what, when. Written once,
/// never modified/deleted — so it stays valid even after the original
/// resource (product/order) is long gone from the database.
#[derive(Debug, Clone, Serialize)]
pub struct AuditLogEntry {
    pub id: String,
    pub tenant_id: String,
    pub actor: Actor,
    pub action: AuditAction,
    pub resource_type: ResourceType,
    pub resource_id: String,
    /// Short label for readability (e.g. product name or the order's
    /// customer name) without needing to join back to a resource that may
    /// have already been deleted.
    pub label: String,
    /// Empty for `Created`/`Deleted` actions. For `Updated`, contains
    /// whichever fields actually changed value (fields sent with the same
    /// value are not considered a change).
    pub changes: Vec<FieldChange>,
    /// Unix timestamp (seconds).
    pub at: u64,
}
