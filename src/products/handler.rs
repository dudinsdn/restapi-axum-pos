use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::Response,
};

use crate::audit::{AuditAction, AuditLogRepository, ResourceType};
use crate::customers::CustomerRepository;
use crate::error::Result;
use crate::orders::OrderRepository;
use crate::pagination::{PaginationQuery, paginated_response};
use crate::state::AppState;
use crate::tenants::TenantRepository;
use crate::users::{Actor, AuthUser, ManagerUser, Role, UserRepository};

use super::model::{
    CreateProductRequest, ProductResponse, UpdateProductRequest,
};
use super::repository::ProductRepository;
use super::service;

/// `tenant_id` is NOT taken from the path/URL — always from the already
/// verified token (`AuthUser`). So there's no "wrong tenant_id" to try,
/// because the client is never asked to send it.
///
/// Paginated via `?limit=&offset=` (see `pagination` module) — the total
/// count before slicing is returned in the `X-Total-Count` header.
pub async fn list_products<TR, PR, OR, UR, AR, CR>(
    auth_user: AuthUser,
    State(state): State<Arc<AppState<TR, PR, OR, UR, AR, CR>>>,
    Query(pagination): Query<PaginationQuery>,
) -> Result<Response>
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
    UR: UserRepository,
    AR: AuditLogRepository,
    CR: CustomerRepository,
{
    let products = service::list_products(
        &state.products,
        &state.tenants,
        &auth_user.tenant_id,
    )
    .await?;

    let can_see_cost_price =
        matches!(auth_user.role, Role::Owner | Role::Admin);
    let response: Vec<ProductResponse> = products
        .into_iter()
        .map(|product| {
            ProductResponse::from_product(product, can_see_cost_price)
        })
        .collect();

    Ok(paginated_response(response, &pagination))
}

/// Owner and Admin can add products to the catalog — Cashier can only
/// view & sell, not manage stock/price.
pub async fn create_product<TR, PR, OR, UR, AR, CR>(
    ManagerUser(auth_user): ManagerUser,
    State(state): State<Arc<AppState<TR, PR, OR, UR, AR, CR>>>,
    Json(payload): Json<CreateProductRequest>,
) -> Result<(StatusCode, Json<ProductResponse>)>
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
    UR: UserRepository,
    AR: AuditLogRepository,
    CR: CustomerRepository,
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

    // `ManagerUser` guarantees Owner or Admin, so cost_price is always
    // included here.
    Ok((
        StatusCode::CREATED,
        Json(ProductResponse::from_product(product, true)),
    ))
}

/// Owner and Admin can update product data (price, stock, etc).
pub async fn update_product<TR, PR, OR, UR, AR, CR>(
    ManagerUser(auth_user): ManagerUser,
    Path(product_id): Path<String>,
    State(state): State<Arc<AppState<TR, PR, OR, UR, AR, CR>>>,
    Json(payload): Json<UpdateProductRequest>,
) -> Result<Json<ProductResponse>>
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
    UR: UserRepository,
    AR: AuditLogRepository,
    CR: CustomerRepository,
{
    let (product, changes) = service::update_product(
        &state.products,
        &auth_user.tenant_id,
        &product_id,
        payload,
    )
    .await?;

    // No field actually changed value (e.g. client sent the exact same
    // value) -> no need to write an "empty" audit entry.
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

    // `ManagerUser` guarantees Owner or Admin, so cost_price is always
    // included here.
    Ok(Json(ProductResponse::from_product(product, true)))
}

/// Owner and Admin can delete a product from the catalog.
pub async fn delete_product<TR, PR, OR, UR, AR, CR>(
    ManagerUser(auth_user): ManagerUser,
    Path(product_id): Path<String>,
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
