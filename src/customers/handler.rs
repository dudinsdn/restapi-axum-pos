use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};

use crate::audit::{AuditAction, AuditLogRepository, ResourceType};
use crate::error::Result;
use crate::orders::OrderRepository;
use crate::products::ProductRepository;
use crate::state::AppState;
use crate::tenants::TenantRepository;
use crate::users::{Actor, AuthUser, ManagerUser, UserRepository};

use super::model::{CreateCustomerRequest, Customer, UpdateCustomerRequest};
use super::repository::CustomerRepository;
use super::service;

/// Semua role (Owner/Admin/Cashier) boleh lihat daftar pelanggan — kasir
/// perlu cari pelanggan lama saat transaksi.
pub async fn list_customers<TR, PR, OR, UR, AR, CR>(
    auth_user: AuthUser,
    State(state): State<Arc<AppState<TR, PR, OR, UR, AR, CR>>>,
) -> Result<Json<Vec<Customer>>>
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
    UR: UserRepository,
    AR: AuditLogRepository,
    CR: CustomerRepository,
{
    let customers = service::list_customers(
        &state.customers,
        &state.tenants,
        &auth_user.tenant_id,
    )
    .await?;
    Ok(Json(customers))
}

/// Semua role boleh lihat detail satu pelanggan.
pub async fn get_customer<TR, PR, OR, UR, AR, CR>(
    auth_user: AuthUser,
    Path(customer_id): Path<String>,
    State(state): State<Arc<AppState<TR, PR, OR, UR, AR, CR>>>,
) -> Result<Json<Customer>>
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
    UR: UserRepository,
    AR: AuditLogRepository,
    CR: CustomerRepository,
{
    let customer = service::get_customer(
        &state.customers,
        &auth_user.tenant_id,
        &customer_id,
    )
    .await?;
    Ok(Json(customer))
}

/// Semua role boleh daftarkan pelanggan baru — kasir sering mendaftarkan
/// pelanggan langsung saat transaksi pertama mereka.
pub async fn create_customer<TR, PR, OR, UR, AR, CR>(
    auth_user: AuthUser,
    State(state): State<Arc<AppState<TR, PR, OR, UR, AR, CR>>>,
    Json(payload): Json<CreateCustomerRequest>,
) -> Result<(StatusCode, Json<Customer>)>
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
    UR: UserRepository,
    AR: AuditLogRepository,
    CR: CustomerRepository,
{
    let actor = Actor::from(&auth_user);
    let customer = service::create_customer(
        &state.customers,
        &state.tenants,
        &auth_user.tenant_id,
        actor.clone(),
        payload,
    )
    .await?;

    crate::audit::service::record(
        &state.audit,
        &auth_user.tenant_id,
        &actor,
        AuditAction::Created,
        ResourceType::Customer,
        &customer.id,
        &format!("{} ({})", customer.name, customer.phone),
        Vec::new(),
    )
    .await;

    Ok((StatusCode::CREATED, Json(customer)))
}

/// Semua role boleh perbarui data kontak pelanggan (mis. kasir memperbaiki
/// nomor HP/alamat yang salah saat melayani pelanggan).
pub async fn update_customer<TR, PR, OR, UR, AR, CR>(
    auth_user: AuthUser,
    Path(customer_id): Path<String>,
    State(state): State<Arc<AppState<TR, PR, OR, UR, AR, CR>>>,
    Json(payload): Json<UpdateCustomerRequest>,
) -> Result<Json<Customer>>
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
    UR: UserRepository,
    AR: AuditLogRepository,
    CR: CustomerRepository,
{
    let (customer, changes) = service::update_customer(
        &state.customers,
        &auth_user.tenant_id,
        &customer_id,
        payload,
    )
    .await?;

    if !changes.is_empty() {
        crate::audit::service::record(
            &state.audit,
            &auth_user.tenant_id,
            &Actor::from(&auth_user),
            AuditAction::Updated,
            ResourceType::Customer,
            &customer.id,
            &format!("{} ({})", customer.name, customer.phone),
            changes,
        )
        .await;
    }

    Ok(Json(customer))
}

/// Hanya Owner dan Admin yang boleh menghapus data pelanggan — ini aksi
/// destruktif/permanen, jadi tidak dibiarkan untuk Cashier walau mereka
/// boleh membuat & mengedit data pelanggan sehari-hari.
pub async fn delete_customer<TR, PR, OR, UR, AR, CR>(
    ManagerUser(auth_user): ManagerUser,
    Path(customer_id): Path<String>,
    State(state): State<Arc<AppState<TR, PR, OR, UR, AR, CR>>>,
) -> Result<StatusCode>
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
    UR: UserRepository,
    AR: AuditLogRepository,
    CR: CustomerRepository,
{
    let customer = service::delete_customer(
        &state.customers,
        &auth_user.tenant_id,
        &customer_id,
    )
    .await?;

    crate::audit::service::record(
        &state.audit,
        &auth_user.tenant_id,
        &Actor::from(&auth_user),
        AuditAction::Deleted,
        ResourceType::Customer,
        &customer.id,
        &format!("{} ({})", customer.name, customer.phone),
        Vec::new(),
    )
    .await;

    Ok(StatusCode::NO_CONTENT)
}
