use std::collections::HashMap;
use std::future::Future;

use parking_lot::RwLock;

use super::model::Product;

pub trait ProductRepository: Send + Sync + 'static {
    fn create(&self, product: Product) -> impl Future<Output = bool> + Send;
    fn list_by_tenant(
        &self,
        tenant_id: &str,
    ) -> impl Future<Output = Vec<Product>> + Send;
    /// Cari produk milik satu tenant berdasarkan SKU. Dipakai order untuk
    /// mengambil nama & harga yang sebenarnya, bukan dari input client.
    fn get_by_sku(
        &self,
        tenant_id: &str,
        sku: &str,
    ) -> impl Future<Output = Option<Product>> + Send;
    /// Cari produk berdasarkan id-nya sendiri (lintas tenant) — pemanggil
    /// WAJIB cek `product.tenant_id` sendiri sebelum dipakai, karena method
    /// ini sengaja tidak scoped per tenant (dipakai lookup awal sebelum tahu
    /// siapa pemiliknya).
    fn get(&self, id: &str) -> impl Future<Output = Option<Product>> + Send;
    /// Timpa product yang sudah ada. Return `false` kalau id-nya belum ada
    /// sama sekali (harusnya tidak terjadi kalau dipanggil setelah `get`).
    fn update(&self, product: Product) -> impl Future<Output = bool> + Send;
    /// Hapus product. Return `false` kalau id-nya tidak ada.
    fn delete(&self, id: &str) -> impl Future<Output = bool> + Send;
    /// Kurangi stock atomically. Return `false` kalau produk tidak ada
    /// atau stock tidak cukup — tidak ada perubahan terjadi di kasus itu.
    fn reserve_stock(
        &self,
        product_id: &str,
        quantity: i32,
    ) -> impl Future<Output = bool> + Send;
    /// Kembalikan stock yang sudah di-reserve (dipakai untuk rollback order
    /// yang gagal, atau saat order dibatalkan).
    fn release_stock(
        &self,
        product_id: &str,
        quantity: i32,
    ) -> impl Future<Output = ()> + Send;
}

#[derive(Debug, Default)]
pub struct InMemoryProductRepository {
    data: RwLock<HashMap<String, Product>>,
}

impl InMemoryProductRepository {
    pub fn new() -> Self {
        Self::default()
    }
}

impl ProductRepository for InMemoryProductRepository {
    async fn create(&self, product: Product) -> bool {
        // Satu write-lock untuk cek id + sku (scoped per tenant) DAN insert
        // sekaligus, supaya atomic — sama seperti perbaikan slug di tenant.
        let mut data = self.data.write();

        let sku_taken = data.values().any(|existing| {
            existing.tenant_id == product.tenant_id
                && existing.sku == product.sku
        });

        if sku_taken || data.contains_key(&product.id) {
            return false;
        }

        data.insert(product.id.clone(), product);
        true
    }

    async fn list_by_tenant(&self, tenant_id: &str) -> Vec<Product> {
        self.data
            .read()
            .values()
            .filter(|product| product.tenant_id == tenant_id)
            .cloned()
            .collect()
    }

    async fn get_by_sku(&self, tenant_id: &str, sku: &str) -> Option<Product> {
        self.data
            .read()
            .values()
            .find(|product| {
                product.tenant_id == tenant_id && product.sku == sku
            })
            .cloned()
    }

    async fn get(&self, id: &str) -> Option<Product> {
        self.data.read().get(id).cloned()
    }

    async fn update(&self, product: Product) -> bool {
        let mut data = self.data.write();
        if !data.contains_key(&product.id) {
            return false;
        }
        data.insert(product.id.clone(), product);
        true
    }

    async fn delete(&self, id: &str) -> bool {
        self.data.write().remove(id).is_some()
    }

    async fn reserve_stock(&self, product_id: &str, quantity: i32) -> bool {
        let mut data = self.data.write();
        if let Some(product) = data.get_mut(product_id) {
            if product.stock >= quantity {
                product.stock -= quantity;
                return true;
            }
        }
        false
    }

    async fn release_stock(&self, product_id: &str, quantity: i32) {
        if let Some(product) = self.data.write().get_mut(product_id) {
            product.stock += quantity;
        }
    }
}
