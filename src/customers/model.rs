use serde::{Deserialize, Serialize};

use crate::users::Actor;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Customer {
    pub id: String,
    pub tenant_id: String,
    pub name: String,
    /// Used as a practical identifier at the register (e.g. looking up a
    /// customer by phone number). Unique per tenant — see `CustomerRepository::create`.
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

/// Partial update (all fields optional).
#[derive(Debug, Deserialize)]
pub struct UpdateCustomerRequest {
    pub name: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
}
