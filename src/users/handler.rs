use std::sync::Arc;

use axum::{Json, extract::State, http::StatusCode};

use crate::audit::AuditLogRepository;
use crate::error::{AppError, Result};
use crate::orders::OrderRepository;
use crate::products::ProductRepository;
use crate::state::AppState;
use crate::tenants::TenantRepository;

use super::extractor::AuthUser;
use super::jwt::issue_token;
use super::model::{
    AuthResponse, InviteStaffRequest, LoginRequest, PublicUser,
    RegisterRequest, Role,
};
use super::repository::UserRepository;
use super::service;

pub async fn register<TR, PR, OR, UR, AR>(
    State(state): State<Arc<AppState<TR, PR, OR, UR, AR>>>,
    Json(payload): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<AuthResponse>)>
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
    UR: UserRepository,
    AR: AuditLogRepository,
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

pub async fn login<TR, PR, OR, UR, AR>(
    State(state): State<Arc<AppState<TR, PR, OR, UR, AR>>>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<AuthResponse>>
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
    UR: UserRepository,
    AR: AuditLogRepository,
{
    let user = service::login(&state.users, &state.login_rate_limiter, payload)
        .await?;
    let token = issue_token(&user, &state.jwt_secret)?;

    Ok(Json(AuthResponse {
        token,
        user: user.into(),
    }))
}

pub async fn logout<TR, PR, OR, UR, AR>(
    auth_user: AuthUser,
    State(state): State<Arc<AppState<TR, PR, OR, UR, AR>>>,
) -> StatusCode
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
    UR: UserRepository,
    AR: AuditLogRepository,
{
    state.revoked_tokens.revoke(&auth_user.token_id);
    StatusCode::NO_CONTENT
}

pub async fn invite_staff<TR, PR, OR, UR, AR>(
    auth_user: AuthUser,
    State(state): State<Arc<AppState<TR, PR, OR, UR, AR>>>,
    Json(payload): Json<InviteStaffRequest>,
) -> Result<(StatusCode, Json<PublicUser>)>
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
    UR: UserRepository,
    AR: AuditLogRepository,
{
    if auth_user.role != Role::Owner {
        return Err(AppError::Forbidden(
            "only the tenant owner can invite staff".into(),
        ));
    }

    let user =
        service::invite_staff(&state.users, &auth_user.tenant_id, payload)
            .await?;
    Ok((StatusCode::CREATED, Json(user.into())))
}
