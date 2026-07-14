use argon2::password_hash::{SaltString, rand_core::OsRng};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};

use crate::error::{AppError, Result};
use crate::tenants::{Tenant, TenantRepository};

use super::model::{
    InviteStaffRequest, LoginRequest, RegisterRequest, Role, User,
};
use super::repository::UserRepository;
use super::session::LoginRateLimiter;

pub async fn register<UR, TR>(
    users: &UR,
    tenants: &TR,
    payload: RegisterRequest,
) -> Result<(Tenant, User)>
where
    UR: UserRepository,
    TR: TenantRepository,
{
    let tenant = Tenant {
        id: format!("tenant-{}", uuid::Uuid::new_v4().simple()),
        name: payload.tenant_name,
        slug: payload.tenant_slug,
        address: payload.tenant_address,
    };

    if !tenants.create(tenant.clone()).await {
        return Err(AppError::Conflict(format!(
            "slug '{}' already in use",
            tenant.slug
        )));
    }

    let email = payload.email.trim().to_lowercase();
    let password_hash = hash_password(&payload.password)?;

    let user = User {
        id: format!("user-{}", uuid::Uuid::new_v4().simple()),
        tenant_id: tenant.id.clone(),
        name: payload.name.trim().to_string(),
        email: email.clone(),
        password_hash,
        role: Role::Owner,
    };

    if !users.create(user.clone()).await {
        // Rollback: the tenant has already been created but the email is
        // already in use by someone else. For in-memory storage this is
        // enough, but once moved to a real database, this whole process
        // should ideally be a single transaction.
        tenants.delete(&tenant.id).await;
        return Err(AppError::Conflict(format!(
            "email '{email}' already registered"
        )));
    }

    Ok((tenant, user))
}

pub async fn login<UR>(
    users: &UR,
    rate_limiter: &LoginRateLimiter,
    payload: LoginRequest,
) -> Result<User>
where
    UR: UserRepository,
{
    let email = payload.email.trim().to_lowercase();

    if !rate_limiter.check(&email) {
        return Err(AppError::TooManyRequests(
            "too many failed login attempts, try again in a few minutes".into(),
        ));
    }

    // The error message is INTENTIONALLY identical whether the email
    // isn't found or the password is wrong, so as not to leak which
    // emails are registered.
    let invalid = || AppError::Unauthorized("invalid email or password".into());

    let user = match users.get_by_email(&email).await {
        Some(user) => user,
        None => {
            rate_limiter.record_failure(&email);
            return Err(invalid());
        }
    };

    if verify_password(&payload.password, &user.password_hash).is_err() {
        rate_limiter.record_failure(&email);
        return Err(invalid());
    }

    rate_limiter.reset(&email);
    Ok(user)
}

pub async fn invite_staff<UR>(
    users: &UR,
    tenant_id: &str,
    payload: InviteStaffRequest,
) -> Result<User>
where
    UR: UserRepository,
{
    if payload.role == Role::Owner {
        return Err(AppError::BadRequest(
            "cannot invite a user with the owner role".into(),
        ));
    }

    let email = payload.email.trim().to_lowercase();
    let password_hash = hash_password(&payload.password)?;

    let user = User {
        id: format!("user-{}", uuid::Uuid::new_v4().simple()),
        tenant_id: tenant_id.to_string(),
        name: payload.name.trim().to_string(),
        email: email.clone(),
        password_hash,
        role: payload.role,
    };

    if !users.create(user.clone()).await {
        return Err(AppError::Conflict(format!(
            "email '{email}' already registered"
        )));
    }

    Ok(user)
}

fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| AppError::BadRequest("failed to hash password".into()))
}

fn verify_password(password: &str, hash: &str) -> std::result::Result<(), ()> {
    let parsed_hash = PasswordHash::new(hash).map_err(|_| ())?;
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .map_err(|_| ())
}
