use serde::Serialize;

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
}

/// Satu baris riwayat: siapa, ngapain, terhadap apa, kapan. Ditulis sekali,
/// tidak pernah diubah/dihapus — jadi tetap valid walau resource aslinya
/// (product/order) sudah lama hilang dari database.
#[derive(Debug, Clone, Serialize)]
pub struct AuditLogEntry {
    pub id: String,
    pub tenant_id: String,
    pub actor: Actor,
    pub action: AuditAction,
    pub resource_type: ResourceType,
    pub resource_id: String,
    /// Label ringkas biar enak dibaca (mis. nama produk atau nama pelanggan
    /// order) tanpa perlu join balik ke resource yang mungkin sudah dihapus.
    pub label: String,
    /// Unix timestamp (detik).
    pub at: u64,
}
