use std::time::{Duration, SystemTime, UNIX_EPOCH};

use jsonwebtoken::{
    DecodingKey, EncodingKey, Header, Validation, decode, encode,
};
use serde::{Deserialize, Serialize};

use crate::error::{AppError, Result};

use super::model::{Role, User};

const TOKEN_LIFETIME_SECS: u64 = 24 * 60 * 60; // 24 jam

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // user id
    pub tenant_id: String,
    pub name: String,
    pub role: Role,
    /// ID unik per token (bukan per user) — dipakai untuk revoke token
    /// spesifik ini saat logout, tanpa mempengaruhi token lain milik user
    /// yang sama (mis. kalau dia login dari 2 device berbeda).
    pub jti: String,
    pub exp: usize,
}

pub fn issue_token(user: &User, secret: &str) -> Result<String> {
    let expires_at =
        SystemTime::now() + Duration::from_secs(TOKEN_LIFETIME_SECS);
    let exp = expires_at
        .duration_since(UNIX_EPOCH)
        .map_err(|_| {
            AppError::BadRequest("failed to compute token expiry".into())
        })?
        .as_secs() as usize;

    let claims = Claims {
        sub: user.id.clone(),
        tenant_id: user.tenant_id.clone(),
        name: user.name.clone(),
        role: user.role,
        jti: uuid::Uuid::new_v4().to_string(),
        exp,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|_| AppError::BadRequest("failed to issue token".into()))
}

pub fn decode_token(token: &str, secret: &str) -> Result<Claims> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map(|data| data.claims)
    .map_err(|_| AppError::Unauthorized("invalid or expired token".into()))
}
