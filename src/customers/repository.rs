use std::collections::HashMap;
use std::future::Future;

use parking_lot::RwLock;

use super::model::Customer;

pub trait CustomerRepository: Send + Sync + 'static {
    fn create(&self, customer: Customer) -> impl Future<Output = bool> + Send;
    fn list_by_tenant(
        &self,
        tenant_id: &str,
    ) -> impl Future<Output = Vec<Customer>> + Send;
    /// Cari customer berdasarkan id-nya sendiri (lintas tenant) —
    /// pemanggil WAJIB cek `customer.tenant_id` sendiri sebelum dipakai,
    /// sama seperti pola yang sama di `ProductRepository::get`.
    fn get(&self, id: &str) -> impl Future<Output = Option<Customer>> + Send;
    /// Cari customer milik satu tenant berdasarkan nomor HP. Dipakai untuk
    /// cek keunikan sebelum create/update, mirip `ProductRepository::get_by_sku`.
    fn get_by_phone(
        &self,
        tenant_id: &str,
        phone: &str,
    ) -> impl Future<Output = Option<Customer>> + Send;
    /// Timpa customer yang sudah ada. Return `false` kalau id-nya belum
    /// ada sama sekali (harusnya tidak terjadi kalau dipanggil setelah
    /// `get`).
    fn update(&self, customer: Customer) -> impl Future<Output = bool> + Send;
    /// Hapus customer. Return `false` kalau id-nya tidak ada.
    fn delete(&self, id: &str) -> impl Future<Output = bool> + Send;
}

#[derive(Debug, Default)]
pub struct InMemoryCustomerRepository {
    data: RwLock<HashMap<String, Customer>>,
}

impl InMemoryCustomerRepository {
    pub fn new() -> Self {
        Self::default()
    }
}

impl CustomerRepository for InMemoryCustomerRepository {
    async fn create(&self, customer: Customer) -> bool {
        // Satu write-lock untuk cek id + nomor HP (scoped per tenant) DAN
        // insert sekaligus, supaya atomic — sama seperti pengecekan sku di
        // ProductRepository.
        let mut data = self.data.write();

        let phone_taken = data.values().any(|existing| {
            existing.tenant_id == customer.tenant_id
                && existing.phone == customer.phone
        });

        if phone_taken || data.contains_key(&customer.id) {
            return false;
        }

        data.insert(customer.id.clone(), customer);
        true
    }

    async fn list_by_tenant(&self, tenant_id: &str) -> Vec<Customer> {
        self.data
            .read()
            .values()
            .filter(|customer| customer.tenant_id == tenant_id)
            .cloned()
            .collect()
    }

    async fn get(&self, id: &str) -> Option<Customer> {
        self.data.read().get(id).cloned()
    }

    async fn get_by_phone(
        &self,
        tenant_id: &str,
        phone: &str,
    ) -> Option<Customer> {
        self.data
            .read()
            .values()
            .find(|customer| {
                customer.tenant_id == tenant_id && customer.phone == phone
            })
            .cloned()
    }

    async fn update(&self, customer: Customer) -> bool {
        let mut data = self.data.write();
        if !data.contains_key(&customer.id) {
            return false;
        }
        data.insert(customer.id.clone(), customer);
        true
    }

    async fn delete(&self, id: &str) -> bool {
        self.data.write().remove(id).is_some()
    }
}
