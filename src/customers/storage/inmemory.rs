use parking_lot::RwLock;
use std::collections::HashMap;

use super::super::model::Customer;
use super::super::repository::CustomerRepository;

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
        // A single write-lock to check id + phone number (scoped per tenant)
        // AND insert at once, so it's atomic — same as the sku check in
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
