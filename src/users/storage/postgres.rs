use sqlx::PgPool;

use super::super::model::{Role, User};
use super::super::repository::UserRepository;

#[derive(Debug, Clone, sqlx::FromRow)]
struct UserRow {
    id: String,
    tenant_id: String,
    name: String,
    email: String,
    password_hash: String,
    role: String,
}

impl From<UserRow> for User {
    fn from(row: UserRow) -> Self {
        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            name: row.name,
            email: row.email,
            password_hash: row.password_hash,
            role: role_from_str(&row.role),
        }
    }
}

/// `Role` doesn't derive `sqlx::Type` (keeping the domain model free of
/// storage-layer concerns), so it's stored as plain `TEXT` with a manual
/// mapping here — kept in sync with `Role`'s `#[serde(rename_all =
/// "snake_case")]` representation so the stored value matches what the
/// JSON API already uses for this field.
fn role_to_str(role: Role) -> &'static str {
    match role {
        Role::Owner => "owner",
        Role::Admin => "admin",
        Role::Cashier => "cashier",
    }
}

fn role_from_str(value: &str) -> Role {
    match value {
        "owner" => Role::Owner,
        "admin" => Role::Admin,
        _ => Role::Cashier,
    }
}

/// Postgres-backed `UserRepository`. See `PgTenantRepository` for the
/// general shape of this pattern.
#[derive(Debug, Clone)]
pub struct PgUserRepository {
    pool: PgPool,
}

impl PgUserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl UserRepository for PgUserRepository {
    async fn create(&self, user: User) -> bool {
        // `email` has a UNIQUE constraint, so the INSERT itself enforces
        // the same global-uniqueness rule
        // `InMemoryUserRepository::create` enforces under a write-lock.
        sqlx::query(
            "INSERT INTO users (id, tenant_id, name, email, password_hash, role) \
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(&user.id)
        .bind(&user.tenant_id)
        .bind(&user.name)
        .bind(&user.email)
        .bind(&user.password_hash)
        .bind(role_to_str(user.role))
        .execute(&self.pool)
        .await
        .is_ok()
    }

    async fn get_by_email(&self, email: &str) -> Option<User> {
        sqlx::query_as::<_, UserRow>(
            "SELECT id, tenant_id, name, email, password_hash, role \
             FROM users WHERE email = $1",
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await
        .ok()
        .flatten()
        .map(Into::into)
    }
}
