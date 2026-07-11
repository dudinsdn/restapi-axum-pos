use std::sync::Arc;

use axum::extract::FromRequestParts;
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;

use crate::error::{AppError, Result};
use crate::orders::OrderRepository;
use crate::products::ProductRepository;
use crate::state::AppState;
use crate::tenants::TenantRepository;

use super::jwt::decode_token;
use super::model::Role;
use super::repository::UserRepository;

/// User yang sudah terverifikasi dari Bearer token di header `Authorization`.
/// Dipakai sebagai parameter handler untuk endpoint yang wajib login.
pub struct AuthUser {
    pub user_id: String,
    pub tenant_id: String,
    pub role: Role,
    /// `jti` token ini — dipakai handler `/auth/logout` untuk revoke
    /// token yang sedang dipakai (bukan semua token milik user).
    pub token_id: String,
}

#[async_trait::async_trait]
impl<TR, PR, OR, UR> FromRequestParts<Arc<AppState<TR, PR, OR, UR>>>
    for AuthUser
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
    UR: UserRepository,
{
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState<TR, PR, OR, UR>>,
    ) -> Result<Self> {
        let header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| {
                AppError::Unauthorized("missing authorization header".into())
            })?;

        let token = header.strip_prefix("Bearer ").ok_or_else(|| {
            AppError::Unauthorized("expected bearer token".into())
        })?;

        let claims = decode_token(token, &state.jwt_secret)?;

        if state.revoked_tokens.is_revoked(&claims.jti) {
            return Err(AppError::Unauthorized(
                "token has been revoked".into(),
            ));
        }

        Ok(AuthUser {
            user_id: claims.sub,
            tenant_id: claims.tenant_id,
            role: claims.role,
            token_id: claims.jti,
        })
    }
}
