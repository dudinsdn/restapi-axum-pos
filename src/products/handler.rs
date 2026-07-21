use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::Response,
};
use serde::Deserialize;

use crate::audit::{AuditAction, AuditLogRepository, ResourceType};
use crate::categories::CategoryRepository;
use crate::customers::CustomerRepository;
use crate::error::Result;
use crate::orders::OrderRepository;
use crate::pagination::{PaginationQuery, paginated_response};
use crate::state::DynState;
use crate::tenants::TenantRepository;
use crate::users::{Actor, AuthUser, ManagerUser, Role, UserRepository};

use super::model::{
    CreateProductRequest, ProductResponse, UpdateProductRequest,
};
use super::repository::ProductRepository;
use super::service;

/// Optional `?category=` filter for `GET /products`. A separate `Query`
/// extractor from `PaginationQuery` rather than folding `category` into
/// that struct — `pagination` stays a small, reusable module shared by
/// four different list endpoints, none of which otherwise know about
/// products. Axum lets a handler take more than one `Query<T>`; each just
/// ignores whatever fields aren't its own.
#[derive(Debug, Deserialize)]
pub struct ProductFilterQuery {
    pub category: Option<String>,
}

/// `tenant_id` is NOT taken from the path/URL — always from the already
/// verified token (`AuthUser`). So there's no "wrong tenant_id" to try,
/// because the client is never asked to send it.
///
/// Paginated via `?limit=&offset=` (see `pagination` module) — the total
/// count before slicing (i.e. after the `?category=` filter, if any) is
/// returned in the `X-Total-Count` header. Optionally filtered via
/// `?category=`, matched case-insensitively so `?category=beverages` and
/// `?category=Beverages` behave the same.
pub async fn list_products<TR, PR, OR, UR, AR, CR, KR>(
    auth_user: AuthUser,
    State(state): State<DynState<TR, PR, OR, UR, AR, CR, KR>>,
    Query(pagination): Query<PaginationQuery>,
    Query(filter): Query<ProductFilterQuery>,
) -> Result<Response>
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
    UR: UserRepository,
    AR: AuditLogRepository,
    CR: CustomerRepository,
    KR: CategoryRepository,
{
    let mut products = service::list_products(
        &state.products,
        &state.tenants,
        &auth_user.tenant_id,
    )
    .await?;

    if let Some(category) = &filter.category {
        products.retain(|product| {
            product.category.eq_ignore_ascii_case(category.trim())
        });
    }

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

/// Products at or below their own `low_stock_threshold` (see
/// `Product::low_stock_threshold`), so a manager can see at a glance what
/// needs reordering without scanning the full catalog. Open to any role
/// (same as `list_products`) rather than `ManagerUser` — it exposes no
/// pricing/margin data, just stock counts, and a Cashier noticing a
/// product is about to run out is exactly the kind of thing worth
/// surfacing to them too.
pub async fn list_low_stock_products<TR, PR, OR, UR, AR, CR, KR>(
    auth_user: AuthUser,
    State(state): State<DynState<TR, PR, OR, UR, AR, CR, KR>>,
    Query(pagination): Query<PaginationQuery>,
) -> Result<Response>
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
    UR: UserRepository,
    AR: AuditLogRepository,
    CR: CustomerRepository,
    KR: CategoryRepository,
{
    let products = service::list_low_stock_products(
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
pub async fn create_product<TR, PR, OR, UR, AR, CR, KR>(
    ManagerUser(auth_user): ManagerUser,
    State(state): State<DynState<TR, PR, OR, UR, AR, CR, KR>>,
    Json(payload): Json<CreateProductRequest>,
) -> Result<(StatusCode, Json<ProductResponse>)>
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
    UR: UserRepository,
    AR: AuditLogRepository,
    CR: CustomerRepository,
    KR: CategoryRepository,
{
    let actor = Actor::from(&auth_user);
    let product = service::create_product(
        &state.products,
        &state.tenants,
        &state.categories,
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
pub async fn update_product<TR, PR, OR, UR, AR, CR, KR>(
    ManagerUser(auth_user): ManagerUser,
    Path(product_id): Path<String>,
    State(state): State<DynState<TR, PR, OR, UR, AR, CR, KR>>,
    Json(payload): Json<UpdateProductRequest>,
) -> Result<Json<ProductResponse>>
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
    UR: UserRepository,
    AR: AuditLogRepository,
    CR: CustomerRepository,
    KR: CategoryRepository,
{
    let (product, changes) = service::update_product(
        &state.products,
        &state.categories,
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
pub async fn delete_product<TR, PR, OR, UR, AR, CR, KR>(
    ManagerUser(auth_user): ManagerUser,
    Path(product_id): Path<String>,
    State(state): State<DynState<TR, PR, OR, UR, AR, CR, KR>>,
) -> Result<StatusCode>
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
    UR: UserRepository,
    AR: AuditLogRepository,
    CR: CustomerRepository,
    KR: CategoryRepository,
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
