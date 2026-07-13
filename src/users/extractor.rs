use std::sync::Arc;

use axum::extract::FromRequestParts;
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;

use crate::audit::AuditLogRepository;
use crate::customers::CustomerRepository;
use crate::error::{AppError, Result};
use crate::orders::OrderRepository;
use crate::products::ProductRepository;
use crate::state::AppState;
use crate::tenants::TenantRepository;

use super::jwt::decode_token;
use super::model::{Actor, Role};
use super::repository::UserRepository;

/// User yang sudah terverifikasi dari Bearer token di header `Authorization`.
/// Dipakai sebagai parameter handler untuk endpoint yang wajib login.
pub struct AuthUser {
    pub user_id: String,
    pub tenant_id: String,
    pub name: String,
    pub role: Role,
    /// `jti` token ini — dipakai handler `/auth/logout` untuk revoke
    /// token yang sedang dipakai (bukan semua token milik user).
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

#[async_trait::async_trait]
impl<TR, PR, OR, UR, AR, CR>
    FromRequestParts<Arc<AppState<TR, PR, OR, UR, AR, CR>>> for AuthUser
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
    UR: UserRepository,
    AR: AuditLogRepository,
    CR: CustomerRepository,
{
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState<TR, PR, OR, UR, AR, CR>>,
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

/// Sama seperti `AuthUser`, tapi hanya berhasil di-extract kalau role user
/// itu `Owner`. Dipakai untuk endpoint yang cuma boleh dilakukan pemilik
/// tenant (mis. mengundang user baru) — tinggal ganti parameter handler
/// dari `auth_user: AuthUser` menjadi `OwnerUser(auth_user): OwnerUser`.
///
/// Keuntungannya dibanding cek manual `if auth_user.role != Role::Owner`
/// di dalam body handler: pengecekan jadi bagian dari *signature* handler,
/// bukan langkah opsional yang bisa lupa ditulis atau terlewat saat
/// endpoint baru ditambahkan / di-refactor.
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

#[async_trait::async_trait]
impl<TR, PR, OR, UR, AR, CR>
    FromRequestParts<Arc<AppState<TR, PR, OR, UR, AR, CR>>> for OwnerUser
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
    UR: UserRepository,
    AR: AuditLogRepository,
    CR: CustomerRepository,
{
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState<TR, PR, OR, UR, AR, CR>>,
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

/// Sama seperti `AuthUser`, tapi hanya berhasil di-extract kalau role user
/// itu `Owner` atau `Admin` — dua role yang mengelola operasional toko
/// (katalog produk, pembatalan order, audit log). `Cashier` sengaja tidak
/// termasuk: tugasnya cuma jualan (lihat produk, buat order), bukan
/// mengelola toko.
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

#[async_trait::async_trait]
impl<TR, PR, OR, UR, AR, CR>
    FromRequestParts<Arc<AppState<TR, PR, OR, UR, AR, CR>>> for ManagerUser
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
    UR: UserRepository,
    AR: AuditLogRepository,
    CR: CustomerRepository,
{
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState<TR, PR, OR, UR, AR, CR>>,
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
