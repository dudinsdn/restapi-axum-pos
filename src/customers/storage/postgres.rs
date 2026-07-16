use sqlx::PgPool;
use sqlx::types::Json;

use crate::users::Actor;

use super::super::model::Customer;
use super::super::repository::CustomerRepository;

#[derive(Debug, Clone, sqlx::FromRow)]
struct CustomerRow {
    id: String,
    tenant_id: String,
    name: String,
    phone: String,
    email: Option<String>,
    address: Option<String>,
    created_by: Json<Actor>,
}

impl From<CustomerRow> for Customer {
    fn from(row: CustomerRow) -> Self {
        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            name: row.name,
            phone: row.phone,
            email: row.email,
            address: row.address,
            created_by: row.created_by.0,
        }
    }
}

/// Postgres-backed `CustomerRepository`. See `PgTenantRepository` for the
/// general shape of this pattern.
#[derive(Debug, Clone)]
pub struct PgCustomerRepository {
    pool: PgPool,
}

impl PgCustomerRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl CustomerRepository for PgCustomerRepository {
    async fn create(&self, customer: Customer) -> bool {
        // `(tenant_id, phone)` has a UNIQUE constraint, so the INSERT
        // itself enforces the same rule
        // `InMemoryCustomerRepository::create` enforces under a write-lock.
        sqlx::query(
            "INSERT INTO customers \
             (id, tenant_id, name, phone, email, address, created_by) \
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(&customer.id)
        .bind(&customer.tenant_id)
        .bind(&customer.name)
        .bind(&customer.phone)
        .bind(&customer.email)
        .bind(&customer.address)
        .bind(Json(customer.created_by.clone()))
        .execute(&self.pool)
        .await
        .is_ok()
    }

    async fn list_by_tenant(&self, tenant_id: &str) -> Vec<Customer> {
        sqlx::query_as::<_, CustomerRow>(
            "SELECT id, tenant_id, name, phone, email, address, created_by \
             FROM customers WHERE tenant_id = $1",
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(Into::into)
        .collect()
    }

    async fn get(&self, id: &str) -> Option<Customer> {
        sqlx::query_as::<_, CustomerRow>(
            "SELECT id, tenant_id, name, phone, email, address, created_by \
             FROM customers WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .ok()
        .flatten()
        .map(Into::into)
    }

    async fn get_by_phone(
        &self,
        tenant_id: &str,
        phone: &str,
    ) -> Option<Customer> {
        sqlx::query_as::<_, CustomerRow>(
            "SELECT id, tenant_id, name, phone, email, address, created_by \
             FROM customers WHERE tenant_id = $1 AND phone = $2",
        )
        .bind(tenant_id)
        .bind(phone)
        .fetch_optional(&self.pool)
        .await
        .ok()
        .flatten()
        .map(Into::into)
    }

    async fn update(&self, customer: Customer) -> bool {
        let result = sqlx::query(
            "UPDATE customers SET name = $2, phone = $3, email = $4, address = $5 \
             WHERE id = $1",
        )
        .bind(&customer.id)
        .bind(&customer.name)
        .bind(&customer.phone)
        .bind(&customer.email)
        .bind(&customer.address)
        .execute(&self.pool)
        .await;

        matches!(result, Ok(outcome) if outcome.rows_affected() > 0)
    }

    async fn delete(&self, id: &str) -> bool {
        let result = sqlx::query("DELETE FROM customers WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await;

        matches!(result, Ok(outcome) if outcome.rows_affected() > 0)
    }
}
