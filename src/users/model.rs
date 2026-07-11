use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Owner,
    Staff,
}

#[derive(Debug, Clone, Serialize)]
pub struct User {
    pub id: String,
    pub tenant_id: String,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub role: Role,
}

/// Registrasi sekaligus membuat tenant baru + user pertama sebagai Owner.
/// Ini satu-satunya cara membuat tenant sekarang — tidak ada lagi endpoint
/// publik untuk create tenant secara terpisah, supaya tidak ada celah
/// "siapa saja bisa bikin tenant atas nama siapa saja".
#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub tenant_name: String,
    pub tenant_slug: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct PublicUser {
    pub id: String,
    pub tenant_id: String,
    pub email: String,
    pub role: Role,
}

impl From<User> for PublicUser {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            tenant_id: user.tenant_id,
            email: user.email,
            role: user.role,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: PublicUser,
}
