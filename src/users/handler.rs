use axum::{Json, extract::State, http::StatusCode};

use crate::audit::AuditLogRepository;
use crate::categories::CategoryRepository;
use crate::customers::CustomerRepository;
use crate::error::Result;
use crate::orders::OrderRepository;
use crate::products::ProductRepository;
use crate::state::DynState;
use crate::tenants::TenantRepository;

use super::extractor::{AuthUser, OwnerUser};
use super::jwt::issue_token;
use super::model::{
    AuthResponse, InviteStaffRequest, LoginRequest, PublicUser, RegisterRequest,
};
use super::repository::UserRepository;
use super::service;

pub async fn register<TR, PR, OR, UR, AR, CR, KR>(
    State(state): State<DynState<TR, PR, OR, UR, AR, CR, KR>>,
    Json(payload): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<AuthResponse>)>
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
    UR: UserRepository,
    AR: AuditLogRepository,
    CR: CustomerRepository,
    KR: CategoryRepository,
{
    let (_tenant, user) =
        service::register(&state.users, &state.tenants, payload).await?;
    let token = issue_token(&user, &state.jwt_secret)?;

    Ok((
        StatusCode::CREATED,
        Json(AuthResponse {
            token,
            user: user.into(),
        }),
    ))
}

pub async fn login<TR, PR, OR, UR, AR, CR, KR>(
    State(state): State<DynState<TR, PR, OR, UR, AR, CR, KR>>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<AuthResponse>>
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
    UR: UserRepository,
    AR: AuditLogRepository,
    CR: CustomerRepository,
    KR: CategoryRepository,
{
    let user = service::login(&state.users, &state.login_rate_limiter, payload)
        .await?;
    let token = issue_token(&user, &state.jwt_secret)?;

    Ok(Json(AuthResponse {
        token,
        user: user.into(),
    }))
}

pub async fn logout<TR, PR, OR, UR, AR, CR, KR>(
    auth_user: AuthUser,
    State(state): State<DynState<TR, PR, OR, UR, AR, CR, KR>>,
) -> StatusCode
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
    UR: UserRepository,
    AR: AuditLogRepository,
    CR: CustomerRepository,
    KR: CategoryRepository,
{
    state.revoked_tokens.revoke(&auth_user.token_id);
    StatusCode::NO_CONTENT
}

/// Only the owner can invite a new user — and the owner chooses
/// their role (`Admin` or `Cashier`) via the `role` field in the body.
pub async fn invite_staff<TR, PR, OR, UR, AR, CR, KR>(
    OwnerUser(auth_user): OwnerUser,
    State(state): State<DynState<TR, PR, OR, UR, AR, CR, KR>>,
    Json(payload): Json<InviteStaffRequest>,
) -> Result<(StatusCode, Json<PublicUser>)>
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
    UR: UserRepository,
    AR: AuditLogRepository,
    CR: CustomerRepository,
    KR: CategoryRepository,
{
    let user =
        service::invite_staff(&state.users, &auth_user.tenant_id, payload)
            .await?;
    Ok((StatusCode::CREATED, Json(user.into())))
}
