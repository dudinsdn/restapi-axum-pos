use std::sync::Arc;

use axum::extract::FromRequestParts;
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;

use crate::audit::AuditLogRepository;
use crate::categories::CategoryRepository;
use crate::customers::CustomerRepository;
use crate::error::{AppError, Result};
use crate::orders::OrderRepository;
use crate::products::ProductRepository;
use crate::state::AppState;
use crate::tenants::TenantRepository;

use super::jwt::decode_token;
use super::model::{Actor, Role};
use super::repository::UserRepository;

/// A verified user from the Bearer token in the `Authorization` header.
/// Used as a handler parameter for endpoints that require login.
pub struct AuthUser {
    pub user_id: String,
    pub tenant_id: String,
    pub name: String,
    pub role: Role,
    /// This token's `jti` — used by the `/auth/logout` handler to revoke
    /// the token currently in use (not all of the user's tokens).
    pub token_id: String,
}

impl From<&AuthUser> for Actor {
    fn from(auth_user: &AuthUser) -> Self {
        Actor {
            user_id: auth_user.user_id.clone(),
            name: auth_user.name.clone(),
        }
    }
}

impl<TR, PR, OR, UR, AR, CR, KR>
    FromRequestParts<Arc<AppState<TR, PR, OR, UR, AR, CR, KR>>> for AuthUser
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
    UR: UserRepository,
    AR: AuditLogRepository,
    CR: CustomerRepository,
    KR: CategoryRepository,
{
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState<TR, PR, OR, UR, AR, CR, KR>>,
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
            name: claims.name,
            role: claims.role,
            token_id: claims.jti,
        })
    }
}

/// Same as `AuthUser`, but only successfully extracts if the user's role
/// is `Owner`. Used for endpoints only the tenant owner may perform (e.g.
/// inviting a new user) — just change the handler parameter from
/// `auth_user: AuthUser` to `OwnerUser(auth_user): OwnerUser`.
///
/// The advantage over a manual `if auth_user.role != Role::Owner` check
/// inside the handler body: the check becomes part of the handler's
/// *signature*, not an optional step that could be forgotten or missed
/// when a new endpoint is added / refactored.
pub struct OwnerUser(pub AuthUser);

impl std::ops::Deref for OwnerUser {
    type Target = AuthUser;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<&OwnerUser> for Actor {
    fn from(owner: &OwnerUser) -> Self {
        Actor::from(&owner.0)
    }
}

impl<TR, PR, OR, UR, AR, CR, KR>
    FromRequestParts<Arc<AppState<TR, PR, OR, UR, AR, CR, KR>>> for OwnerUser
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
    UR: UserRepository,
    AR: AuditLogRepository,
    CR: CustomerRepository,
    KR: CategoryRepository,
{
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState<TR, PR, OR, UR, AR, CR, KR>>,
    ) -> Result<Self> {
        let auth_user = AuthUser::from_request_parts(parts, state).await?;

        if auth_user.role != Role::Owner {
            return Err(AppError::Forbidden(
                "only the tenant owner can access this resource".into(),
            ));
        }

        Ok(OwnerUser(auth_user))
    }
}

/// Same as `AuthUser`, but only successfully extracts if the user's role
/// is `Owner` or `Admin` — the two roles that manage store operations
/// (product catalog, order cancellation, audit log). `Cashier` is
/// intentionally excluded: their job is only to sell (view products,
/// create orders), not to manage the store.
pub struct ManagerUser(pub AuthUser);

impl std::ops::Deref for ManagerUser {
    type Target = AuthUser;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<&ManagerUser> for Actor {
    fn from(manager: &ManagerUser) -> Self {
        Actor::from(&manager.0)
    }
}

impl<TR, PR, OR, UR, AR, CR, KR>
    FromRequestParts<Arc<AppState<TR, PR, OR, UR, AR, CR, KR>>> for ManagerUser
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
    UR: UserRepository,
    AR: AuditLogRepository,
    CR: CustomerRepository,
    KR: CategoryRepository,
{
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState<TR, PR, OR, UR, AR, CR, KR>>,
    ) -> Result<Self> {
        let auth_user = AuthUser::from_request_parts(parts, state).await?;

        if !matches!(auth_user.role, Role::Owner | Role::Admin) {
            return Err(AppError::Forbidden(
                "only the owner or an admin can access this resource".into(),
            ));
        }

        Ok(ManagerUser(auth_user))
    }
}
