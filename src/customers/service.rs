use crate::audit::FieldChange;
use crate::error::{AppError, Result};
use crate::tenants::TenantRepository;
use crate::users::Actor;

use super::model::{CreateCustomerRequest, Customer, UpdateCustomerRequest};
use super::repository::CustomerRepository;

pub async fn list_customers<CR, TR>(
    customers: &CR,
    tenants: &TR,
    tenant_id: &str,
) -> Result<Vec<Customer>>
where
    CR: CustomerRepository,
    TR: TenantRepository,
{
    ensure_tenant_exists(tenants, tenant_id).await?;
    Ok(customers.list_by_tenant(tenant_id).await)
}

pub async fn get_customer<CR: CustomerRepository>(
    customers: &CR,
    tenant_id: &str,
    customer_id: &str,
) -> Result<Customer> {
    fetch_owned_customer(customers, tenant_id, customer_id).await
}

pub async fn create_customer<CR, TR>(
    customers: &CR,
    tenants: &TR,
    tenant_id: &str,
    actor: Actor,
    payload: CreateCustomerRequest,
) -> Result<Customer>
where
    CR: CustomerRepository,
    TR: TenantRepository,
{
    ensure_tenant_exists(tenants, tenant_id).await?;
    validate_name(&payload.name)?;
    validate_phone(&payload.phone)?;

    let customer = Customer {
        id: format!("cust-{}", uuid::Uuid::new_v4().simple()),
        tenant_id: tenant_id.to_string(),
        name: payload.name,
        phone: payload.phone,
        email: payload.email,
        address: payload.address,
        created_by: actor,
    };

    if !customers.create(customer.clone()).await {
        return Err(AppError::Conflict(format!(
            "phone number '{}' already registered for this tenant",
            customer.phone
        )));
    }

    Ok(customer)
}

pub async fn update_customer<CR: CustomerRepository>(
    customers: &CR,
    tenant_id: &str,
    customer_id: &str,
    payload: UpdateCustomerRequest,
) -> Result<(Customer, Vec<FieldChange>)> {
    let mut customer =
        fetch_owned_customer(customers, tenant_id, customer_id).await?;
    let mut changes = Vec::new();

    if let Some(name) = payload.name {
        validate_name(&name)?;
        if name != customer.name {
            changes.push(FieldChange {
                field: "name".to_string(),
                old_value: customer.name.clone(),
                new_value: name.clone(),
            });
            customer.name = name;
        }
    }
    if let Some(phone) = payload.phone {
        validate_phone(&phone)?;
        if phone != customer.phone {
            if let Some(existing) =
                customers.get_by_phone(tenant_id, &phone).await
            {
                if existing.id != customer.id {
                    return Err(AppError::Conflict(format!(
                        "phone number '{phone}' already registered for this tenant"
                    )));
                }
            }
            changes.push(FieldChange {
                field: "phone".to_string(),
                old_value: customer.phone.clone(),
                new_value: phone.clone(),
            });
            customer.phone = phone;
        }
    }
    if let Some(email) = payload.email {
        let old = customer.email.clone().unwrap_or_default();
        if email != old {
            changes.push(FieldChange {
                field: "email".to_string(),
                old_value: old,
                new_value: email.clone(),
            });
            customer.email = Some(email);
        }
    }
    if let Some(address) = payload.address {
        let old = customer.address.clone().unwrap_or_default();
        if address != old {
            changes.push(FieldChange {
                field: "address".to_string(),
                old_value: old,
                new_value: address.clone(),
            });
            customer.address = Some(address);
        }
    }
    // `created_by` is intentionally never changed, same as `Product` — it
    // always records who created the data FIRST.

    if !changes.is_empty() {
        customers.update(customer.clone()).await;
    }

    Ok((customer, changes))
}

/// Returns the deleted customer (not just unit) — used by the caller to
/// write an audit log entry with the customer's name before its data is gone.
pub async fn delete_customer<CR: CustomerRepository>(
    customers: &CR,
    tenant_id: &str,
    customer_id: &str,
) -> Result<Customer> {
    let customer =
        fetch_owned_customer(customers, tenant_id, customer_id).await?;
    customers.delete(&customer.id).await;
    Ok(customer)
}

/// Fetch a customer by id AND ensure it belongs to the requesting tenant.
/// If the customer doesn't exist OR belongs to another tenant, both cases
/// return `NotFound` (not `Forbidden`) — so as not to leak to the client
/// whether that id actually exists but belongs to a different tenant.
async fn fetch_owned_customer<CR: CustomerRepository>(
    customers: &CR,
    tenant_id: &str,
    customer_id: &str,
) -> Result<Customer> {
    customers
        .get(customer_id)
        .await
        .filter(|customer| customer.tenant_id == tenant_id)
        .ok_or_else(|| AppError::NotFound("customer not found".into()))
}

async fn ensure_tenant_exists<TR: TenantRepository>(
    tenants: &TR,
    tenant_id: &str,
) -> Result<()> {
    if tenants.get(tenant_id).await.is_none() {
        return Err(AppError::NotFound("tenant not found".into()));
    }
    Ok(())
}

fn validate_name(name: &str) -> Result<()> {
    if name.trim().is_empty() {
        return Err(AppError::BadRequest("name must not be empty".into()));
    }
    Ok(())
}

fn validate_phone(phone: &str) -> Result<()> {
    if phone.trim().is_empty() {
        return Err(AppError::BadRequest("phone must not be empty".into()));
    }
    Ok(())
}
