use serde::{Deserialize, Serialize};

/// Three access levels:
/// - `Owner`: the tenant's owner, created automatically during `register`.
///   Can do anything, including inviting new `Admin`/`Cashier` users.
/// - `Admin`: manages day-to-day store operations — manage the product
///   catalog, cancel orders, view the audit log. Cannot invite other users.
/// - `Cashier`: a cashier, can only view products & create orders (sales
///   transactions). Cannot modify the catalog, cancel orders, or view the
///   audit log.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Owner,
    Admin,
    Cashier,
}

#[derive(Debug, Clone, Serialize)]
pub struct User {
    pub id: String,
    pub tenant_id: String,
    pub name: String,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub role: Role,
}

/// A brief identity for a user, attached to a resource (product, order,
/// etc) and to the audit log — so it's always clear who did what, even
/// after the original resource has been deleted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Actor {
    pub user_id: String,
    pub name: String,
}

/// Registration simultaneously creates a new tenant + the first user as
/// Owner. This is now the only way to create a tenant — there's no more
/// separate public endpoint to create a tenant, so there's no loophole of
/// "anyone can create a tenant on anyone's behalf".
#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub tenant_name: String,
    pub tenant_slug: String,
    pub tenant_address: Option<String>,
    pub name: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

/// The owner invites a new user (Admin or Cashier) into their own tenant.
/// `tenant_id` is NOT accepted from the body — always taken from the
/// caller's own tenant (`AuthUser`), so an owner can't casually invite
/// into another tenant. `role` is validated in the service: it cannot be
/// `Owner` (there's only one owner per tenant, created via `register`).
#[derive(Debug, Deserialize)]
pub struct InviteStaffRequest {
    pub name: String,
    pub email: String,
    pub password: String,
    pub role: Role,
}

#[derive(Debug, Serialize)]
pub struct PublicUser {
    pub id: String,
    pub tenant_id: String,
    pub name: String,
    pub email: String,
    pub role: Role,
}

impl From<User> for PublicUser {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            tenant_id: user.tenant_id,
            name: user.name,
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
