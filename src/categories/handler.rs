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
use crate::products::{ProductRepository, ProductResponse};
use crate::state::AppState;
use crate::tenants::TenantRepository;
use crate::users::{Actor, AuthUser, ManagerUser, Role, UserRepository};

use super::model::{Category, CreateCategoryRequest, UpdateCategoryRequest};
use super::repository::CategoryRepository;
use super::service;

/// All roles (Owner/Admin/Cashier) can view the category list — a cashier
/// browsing by category while ringing up a sale needs this just as much as
/// a manager curating the list needs to see what already exists.
///
/// Paginated via `?limit=&offset=` (see `pagination` module) — the total
/// count before slicing is returned in the `X-Total-Count` header.
pub async fn list_categories<TR, PR, OR, UR, AR, CR, KR>(
    auth_user: AuthUser,
    State(state): State<Arc<AppState<TR, PR, OR, UR, AR, CR, KR>>>,
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
    let categories = service::list_categories(
        &state.categories,
        &state.tenants,
        &auth_user.tenant_id,
    )
    .await?;
    Ok(paginated_response(categories, &pagination))
}

/// All roles can view a single category's detail.
pub async fn get_category<TR, PR, OR, UR, AR, CR, KR>(
    auth_user: AuthUser,
    Path(category_id): Path<String>,
    State(state): State<Arc<AppState<TR, PR, OR, UR, AR, CR, KR>>>,
) -> Result<Json<Category>>
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
    UR: UserRepository,
    AR: AuditLogRepository,
    CR: CustomerRepository,
    KR: CategoryRepository,
{
    let category = service::get_category(
        &state.categories,
        &auth_user.tenant_id,
        &category_id,
    )
    .await?;
    Ok(Json(category))
}

/// Products currently tagged with this category's name — the "look up by
/// product" half of category CRUD. Open to any role, same as
/// `list_categories`/`list_products`: it exposes no pricing data by
/// itself, and `ProductResponse` still applies the usual Owner/Admin-only
/// `cost_price` gating on top.
pub async fn list_products_in_category<TR, PR, OR, UR, AR, CR, KR>(
    auth_user: AuthUser,
    Path(category_id): Path<String>,
    State(state): State<Arc<AppState<TR, PR, OR, UR, AR, CR, KR>>>,
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
    let products = service::list_products_in_category(
        &state.categories,
        &state.products,
        &state.tenants,
        &auth_user.tenant_id,
        &category_id,
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

/// Owner and Admin can create categories — Cashier can only browse/see
/// them (per `list_categories`/`get_category`), not curate the list.
pub async fn create_category<TR, PR, OR, UR, AR, CR, KR>(
    ManagerUser(auth_user): ManagerUser,
    State(state): State<Arc<AppState<TR, PR, OR, UR, AR, CR, KR>>>,
    Json(payload): Json<CreateCategoryRequest>,
) -> Result<(StatusCode, Json<Category>)>
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
    let category = service::create_category(
        &state.categories,
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
        ResourceType::Category,
        &category.id,
        &category.name,
        Vec::new(),
    )
    .await;

    Ok((StatusCode::CREATED, Json(category)))
}

/// Owner and Admin can rename a category.
pub async fn update_category<TR, PR, OR, UR, AR, CR, KR>(
    ManagerUser(auth_user): ManagerUser,
    Path(category_id): Path<String>,
    State(state): State<Arc<AppState<TR, PR, OR, UR, AR, CR, KR>>>,
    Json(payload): Json<UpdateCategoryRequest>,
) -> Result<Json<Category>>
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
    UR: UserRepository,
    AR: AuditLogRepository,
    CR: CustomerRepository,
    KR: CategoryRepository,
{
    let (category, changes) = service::update_category(
        &state.categories,
        &auth_user.tenant_id,
        &category_id,
        payload,
    )
    .await?;

    if !changes.is_empty() {
        crate::audit::service::record(
            &state.audit,
            &auth_user.tenant_id,
            &Actor::from(&auth_user),
            AuditAction::Updated,
            ResourceType::Category,
            &category.id,
            &category.name,
            changes,
        )
        .await;
    }

    Ok(Json(category))
}

/// Only Owner and Admin can delete a category — same reasoning as
/// `customers::handler::delete_customer`: destructive/permanent, so it's
/// not left open to Cashier even though Cashier can view the list day to
/// day.
pub async fn delete_category<TR, PR, OR, UR, AR, CR, KR>(
    ManagerUser(auth_user): ManagerUser,
    Path(category_id): Path<String>,
    State(state): State<Arc<AppState<TR, PR, OR, UR, AR, CR, KR>>>,
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
    let category = service::delete_category(
        &state.categories,
        &state.products,
        &auth_user.tenant_id,
        &category_id,
    )
    .await?;

    crate::audit::service::record(
        &state.audit,
        &auth_user.tenant_id,
        &Actor::from(&auth_user),
        AuditAction::Deleted,
        ResourceType::Category,
        &category.id,
        &category.name,
        Vec::new(),
    )
    .await;

    Ok(StatusCode::NO_CONTENT)
}
