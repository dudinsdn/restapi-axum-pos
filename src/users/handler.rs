use std::sync::Arc;

use axum::{Json, extract::State, http::StatusCode};

use crate::error::Result;
use crate::orders::OrderRepository;
use crate::products::ProductRepository;
use crate::state::AppState;
use crate::tenants::TenantRepository;

use super::jwt::issue_token;
use super::model::{AuthResponse, LoginRequest, RegisterRequest};
use super::repository::UserRepository;
use super::service;

pub async fn register<TR, PR, OR, UR>(
    State(state): State<Arc<AppState<TR, PR, OR, UR>>>,
    Json(payload): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<AuthResponse>)>
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
    UR: UserRepository,
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

pub async fn login<TR, PR, OR, UR>(
    State(state): State<Arc<AppState<TR, PR, OR, UR>>>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<AuthResponse>>
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
    UR: UserRepository,
{
    let user = service::login(&state.users, payload).await?;
    let token = issue_token(&user, &state.jwt_secret)?;

    Ok(Json(AuthResponse {
        token,
        user: user.into(),
    }))
}
