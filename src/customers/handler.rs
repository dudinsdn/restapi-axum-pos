use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::Response,
};

use crate::audit::{AuditAction, AuditLogRepository, ResourceType};
use crate::categories::CategoryRepository;
use crate::error::Result;
use crate::orders::OrderRepository;
use crate::pagination::{PaginationQuery, paginated_response};
use crate::products::ProductRepository;
use crate::state::AppState;
use crate::tenants::TenantRepository;
use crate::users::{Actor, AuthUser, ManagerUser, UserRepository};

use super::model::{CreateCustomerRequest, Customer, UpdateCustomerRequest};
use super::repository::CustomerRepository;
use super::service;

/// All roles (Owner/Admin/Cashier) can view the customer list — a cashier
/// needs to look up existing customers during a transaction.
///
/// Paginated via `?limit=&offset=` (see `pagination` module) — the total
/// count before slicing is returned in the `X-Total-Count` header.
pub async fn list_customers<TR, PR, OR, UR, AR, CR, KR>(
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
    let customers = service::list_customers(
        &state.customers,
        &state.tenants,
        &auth_user.tenant_id,
    )
    .await?;
    Ok(paginated_response(customers, &pagination))
}

/// All roles can view a single customer's detail.
pub async fn get_customer<TR, PR, OR, UR, AR, CR, KR>(
    auth_user: AuthUser,
    Path(customer_id): Path<String>,
    State(state): State<Arc<AppState<TR, PR, OR, UR, AR, CR, KR>>>,
) -> Result<Json<Customer>>
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
    UR: UserRepository,
    AR: AuditLogRepository,
    CR: CustomerRepository,
    KR: CategoryRepository,
{
    let customer = service::get_customer(
        &state.customers,
        &auth_user.tenant_id,
        &customer_id,
    )
    .await?;
    Ok(Json(customer))
}

/// All roles can register a new customer — a cashier often registers a
/// customer on the spot during their first transaction.
pub async fn create_customer<TR, PR, OR, UR, AR, CR, KR>(
    auth_user: AuthUser,
    State(state): State<Arc<AppState<TR, PR, OR, UR, AR, CR, KR>>>,
    Json(payload): Json<CreateCustomerRequest>,
) -> Result<(StatusCode, Json<Customer>)>
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

/// All roles can update a customer's contact info (e.g. a cashier fixing a
/// wrong phone number/address while serving the customer).
pub async fn update_customer<TR, PR, OR, UR, AR, CR, KR>(
    auth_user: AuthUser,
    Path(customer_id): Path<String>,
    State(state): State<Arc<AppState<TR, PR, OR, UR, AR, CR, KR>>>,
    Json(payload): Json<UpdateCustomerRequest>,
) -> Result<Json<Customer>>
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
    UR: UserRepository,
    AR: AuditLogRepository,
    CR: CustomerRepository,
    KR: CategoryRepository,
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

/// Only Owner and Admin can delete a customer — this is a destructive,
/// permanent action, so it's not allowed for Cashier even though they can
/// create & edit customer data day-to-day.
pub async fn delete_customer<TR, PR, OR, UR, AR, CR, KR>(
    ManagerUser(auth_user): ManagerUser,
    Path(customer_id): Path<String>,
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
