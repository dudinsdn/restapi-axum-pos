use std::collections::HashMap;
use std::future::Future;

use parking_lot::RwLock;

use super::model::Tenant;

/// Kontrak storage untuk tenant. Method dideklarasikan lewat
/// `-> impl Future<...> + Send` (bukan gula `async fn`) supaya bound
/// `Send` ikut terjamin di level trait — penting karena trait ini dipakai
/// generic di handler Axum, yang mensyaratkan future-nya `Send`.
///
/// Implementasi in-memory sekarang tidak pernah benar-benar `.await`
/// apa pun (murni operasi `RwLock` yang sinkron), tapi signature-nya
/// sudah async dari awal. Jadi kalau nanti ganti ke Postgres/SQLite,
/// cukup buat struct baru yang implement trait ini — handler dan
/// service tidak perlu diubah sama sekali.
pub trait TenantRepository: Send + Sync + 'static {
    fn create(&self, tenant: Tenant) -> impl Future<Output = bool> + Send;
    fn get(&self, id: &str) -> impl Future<Output = Option<Tenant>> + Send;
    fn list(&self) -> impl Future<Output = Vec<Tenant>> + Send;
    /// Dipakai untuk rollback kalau proses register gagal setelah tenant
    /// terlanjur dibuat (mis. email sudah dipakai).
    fn delete(&self, id: &str) -> impl Future<Output = ()> + Send;
}

#[derive(Debug, Default)]
pub struct InMemoryTenantRepository {
    data: RwLock<HashMap<String, Tenant>>,
}

impl InMemoryTenantRepository {
    pub fn new() -> Self {
        Self::default()
    }
}

impl TenantRepository for InMemoryTenantRepository {
    async fn create(&self, tenant: Tenant) -> bool {
        // Satu write-lock untuk cek id + slug DAN insert sekaligus.
        // Sengaja tidak dipecah jadi read-check lalu write-insert terpisah,
        // karena itu membuka celah race condition: dua request bersamaan
        // bisa lolos cek "belum ada" sebelum salah satunya sempat insert.
        let mut data = self.data.write();

        let slug_taken =
            data.values().any(|existing| existing.slug == tenant.slug);
        if slug_taken || data.contains_key(&tenant.id) {
            return false;
        }

        data.insert(tenant.id.clone(), tenant);
        true
    }

    async fn get(&self, id: &str) -> Option<Tenant> {
        self.data.read().get(id).cloned()
    }

    async fn list(&self) -> Vec<Tenant> {
        self.data.read().values().cloned().collect()
    }

    async fn delete(&self, id: &str) {
        self.data.write().remove(id);
    }
}
