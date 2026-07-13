use serde::{Deserialize, Serialize};

use crate::users::Actor;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Customer {
    pub id: String,
    pub tenant_id: String,
    pub name: String,
    /// Dipakai sebagai identifier praktis di kasir (mis. cari pelanggan
    /// lewat nomor HP). Unik per tenant — lihat `CustomerRepository::create`.
    pub phone: String,
    pub email: Option<String>,
    pub address: Option<String>,
    pub created_by: Actor,
}

#[derive(Debug, Deserialize)]
pub struct CreateCustomerRequest {
    pub name: String,
    pub phone: String,
    pub email: Option<String>,
    pub address: Option<String>,
}

/// Update sebagian (semua field opsional).
#[derive(Debug, Deserialize)]
pub struct UpdateCustomerRequest {
    pub name: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
}
