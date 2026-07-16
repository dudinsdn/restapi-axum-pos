use sqlx::PgPool;

use super::super::model::Tenant;
use super::super::repository::TenantRepository;

#[derive(Debug, Clone, sqlx::FromRow)]
struct TenantRow {
    id: String,
    name: String,
    slug: String,
    address: Option<String>,
}

impl From<TenantRow> for Tenant {
    fn from(row: TenantRow) -> Self {
        Self {
            id: row.id,
            name: row.name,
            slug: row.slug,
            address: row.address,
        }
    }
}

/// Postgres-backed `TenantRepository`. Behaviorally identical to
/// `InMemoryTenantRepository` — same uniqueness rules, same return values
/// — so swapping between them (see `main.rs`) is purely a matter of which
/// one gets constructed at startup; nothing upstream (handlers, services)
/// needs to change.
#[derive(Debug, Clone)]
pub struct PgTenantRepository {
    pool: PgPool,
}

impl PgTenantRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl TenantRepository for PgTenantRepository {
    async fn create(&self, tenant: Tenant) -> bool {
        // `id` is the primary key and `slug` has a UNIQUE constraint (see
        // the migration), so the INSERT failing on either is exactly the
        // same atomicity the in-memory version needs a write-lock for —
        // no separate existence check needed here.
        sqlx::query(
            "INSERT INTO tenants (id, name, slug, address) \
             VALUES ($1, $2, $3, $4)",
        )
        .bind(&tenant.id)
        .bind(&tenant.name)
        .bind(&tenant.slug)
        .bind(&tenant.address)
        .execute(&self.pool)
        .await
        .is_ok()
    }

    async fn get(&self, id: &str) -> Option<Tenant> {
        sqlx::query_as::<_, TenantRow>(
            "SELECT id, name, slug, address FROM tenants WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .ok()
        .flatten()
        .map(Into::into)
    }

    async fn list(&self) -> Vec<Tenant> {
        sqlx::query_as::<_, TenantRow>(
            "SELECT id, name, slug, address FROM tenants",
        )
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(Into::into)
        .collect()
    }

    async fn delete(&self, id: &str) {
        let _ = sqlx::query("DELETE FROM tenants WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await;
    }
}
