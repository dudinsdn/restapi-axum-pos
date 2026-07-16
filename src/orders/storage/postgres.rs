use sqlx::PgPool;
use sqlx::types::Json;

use crate::users::Actor;

use super::super::model::{Order, OrderItem};
use super::super::repository::OrderRepository;

#[derive(Debug, Clone, sqlx::FromRow)]
struct OrderRow {
    id: String,
    tenant_id: String,
    customer_id: String,
    customer_name: String,
    items: Json<Vec<OrderItem>>,
    total: i64,
    created_by: Json<Actor>,
    created_at: i64,
}

impl From<OrderRow> for Order {
    fn from(row: OrderRow) -> Self {
        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            customer_id: row.customer_id,
            customer_name: row.customer_name,
            items: row.items.0,
            total: row.total,
            created_by: row.created_by.0,
            // `created_at` is stored as BIGINT (Postgres has no unsigned
            // integer type) but is always a valid Unix second-timestamp
            // in practice, so this cast back to `u64` never actually loses
            // anything short of the year 2262.
            created_at: row.created_at as u64,
        }
    }
}

/// Postgres-backed `OrderRepository`. `items` (each a full snapshot of
/// sku/name/unit_price/unit_cost, per `OrderItem`) is stored as JSONB
/// rather than a child table — see the migration's comment on the
/// `orders` table for why.
#[derive(Debug, Clone)]
pub struct PgOrderRepository {
    pool: PgPool,
}

impl PgOrderRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl OrderRepository for PgOrderRepository {
    async fn create(&self, order: Order) -> bool {
        sqlx::query(
            "INSERT INTO orders \
             (id, tenant_id, customer_id, customer_name, items, total, created_by, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(&order.id)
        .bind(&order.tenant_id)
        .bind(&order.customer_id)
        .bind(&order.customer_name)
        .bind(Json(order.items.clone()))
        .bind(order.total)
        .bind(Json(order.created_by.clone()))
        .bind(order.created_at as i64)
        .execute(&self.pool)
        .await
        .is_ok()
    }

    async fn list_by_tenant(&self, tenant_id: &str) -> Vec<Order> {
        sqlx::query_as::<_, OrderRow>(
            "SELECT id, tenant_id, customer_id, customer_name, items, total, \
                    created_by, created_at \
             FROM orders WHERE tenant_id = $1",
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(Into::into)
        .collect()
    }

    async fn get(&self, id: &str) -> Option<Order> {
        sqlx::query_as::<_, OrderRow>(
            "SELECT id, tenant_id, customer_id, customer_name, items, total, \
                    created_by, created_at \
             FROM orders WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .ok()
        .flatten()
        .map(Into::into)
    }

    async fn delete(&self, id: &str) -> bool {
        let result = sqlx::query("DELETE FROM orders WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await;

        matches!(result, Ok(outcome) if outcome.rows_affected() > 0)
    }
}
