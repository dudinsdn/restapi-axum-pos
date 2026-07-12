use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};

use crate::audit::{AuditAction, AuditLogRepository, ResourceType};
use crate::error::Result;
use crate::orders::OrderRepository;
use crate::state::AppState;
use crate::tenants::TenantRepository;
use crate::users::{Actor, AuthUser, OwnerUser, UserRepository};

use super::model::{CreateProductRequest, Product, UpdateProductRequest};
use super::repository::ProductRepository;
use super::service;

/// `tenant_id` TIDAK diambil dari path/URL — selalu dari token yang sudah
/// terverifikasi (`AuthUser`). Jadi tidak ada "tenant_id yang salah" untuk
/// dicoba, karena client tidak pernah diminta mengirimkannya.
pub async fn list_products<TR, PR, OR, UR, AR>(
    auth_user: AuthUser,
    State(state): State<Arc<AppState<TR, PR, OR, UR, AR>>>,
) -> Result<Json<Vec<Product>>>
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
    UR: UserRepository,
    AR: AuditLogRepository,
{
    let products = service::list_products(
        &state.products,
        &state.tenants,
        &auth_user.tenant_id,
    )
    .await?;
    Ok(Json(products))
}

/// Hanya owner yang boleh menambah produk ke katalog — staff/kasir cukup
/// bisa melihat & menjual, tidak mengelola stok/harga.
pub async fn create_product<TR, PR, OR, UR, AR>(
    OwnerUser(auth_user): OwnerUser,
    State(state): State<Arc<AppState<TR, PR, OR, UR, AR>>>,
    Json(payload): Json<CreateProductRequest>,
) -> Result<(StatusCode, Json<Product>)>
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
    UR: UserRepository,
    AR: AuditLogRepository,
{
    let actor = Actor::from(&auth_user);
    let product = service::create_product(
        &state.products,
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
        ResourceType::Product,
        &product.id,
        &format!("{} ({})", product.name, product.sku),
        Vec::new(),
    )
    .await;

    Ok((StatusCode::CREATED, Json(product)))
}

/// Hanya owner yang boleh mengubah data produk (harga, stok, dst).
pub async fn update_product<TR, PR, OR, UR, AR>(
    OwnerUser(auth_user): OwnerUser,
    Path(product_id): Path<String>,
    State(state): State<Arc<AppState<TR, PR, OR, UR, AR>>>,
    Json(payload): Json<UpdateProductRequest>,
) -> Result<Json<Product>>
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
    UR: UserRepository,
    AR: AuditLogRepository,
{
    let (product, changes) = service::update_product(
        &state.products,
        &auth_user.tenant_id,
        &product_id,
        payload,
    )
    .await?;

    // Tidak ada field yang benar-benar berubah nilainya (mis. client kirim
    // value yang sama persis) -> tidak perlu tulis entry audit "kosong".
    if !changes.is_empty() {
        crate::audit::service::record(
            &state.audit,
            &auth_user.tenant_id,
            &Actor::from(&auth_user),
            AuditAction::Updated,
            ResourceType::Product,
            &product.id,
            &format!("{} ({})", product.name, product.sku),
            changes,
        )
        .await;
    }

    Ok(Json(product))
}

/// Hanya owner yang boleh menghapus produk dari katalog.
pub async fn delete_product<TR, PR, OR, UR, AR>(
    OwnerUser(auth_user): OwnerUser,
    Path(product_id): Path<String>,
    State(state): State<Arc<AppState<TR, PR, OR, UR, AR>>>,
) -> Result<StatusCode>
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
    UR: UserRepository,
    AR: AuditLogRepository,
{
    let product = service::delete_product(
        &state.products,
        &auth_user.tenant_id,
        &product_id,
    )
    .await?;

    crate::audit::service::record(
        &state.audit,
        &auth_user.tenant_id,
        &Actor::from(&auth_user),
        AuditAction::Deleted,
        ResourceType::Product,
        &product.id,
        &format!("{} ({})", product.name, product.sku),
        Vec::new(),
    )
    .await;

    Ok(StatusCode::NO_CONTENT)
}
