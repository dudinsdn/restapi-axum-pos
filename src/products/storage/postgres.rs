use sqlx::PgPool;
use sqlx::types::Json;

use crate::users::Actor;

use super::super::model::Product;
use super::super::repository::ProductRepository;

#[derive(Debug, Clone, sqlx::FromRow)]
struct ProductRow {
    id: String,
    tenant_id: String,
    name: String,
    sku: String,
    price: i64,
    cost_price: i64,
    stock: i32,
    category: String,
    low_stock_threshold: i32,
    created_by: Json<Actor>,
}

impl From<ProductRow> for Product {
    fn from(row: ProductRow) -> Self {
        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            name: row.name,
            sku: row.sku,
            price: row.price,
            cost_price: row.cost_price,
            stock: row.stock,
            category: row.category,
            low_stock_threshold: row.low_stock_threshold,
            created_by: row.created_by.0,
        }
    }
}

/// Postgres-backed `ProductRepository`. See `PgTenantRepository` for the
/// general shape of this pattern — the interesting part here is
/// `reserve_stock`, which pushes the "enough stock?" check into the SQL
/// `WHERE` clause so the check-and-decrement stays atomic without any
/// application-level locking.
#[derive(Debug, Clone)]
pub struct PgProductRepository {
    pool: PgPool,
}

impl PgProductRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl ProductRepository for PgProductRepository {
    async fn create(&self, product: Product) -> bool {
        // `(tenant_id, sku)` has a UNIQUE constraint, so the INSERT itself
        // enforces the same rule `InMemoryProductRepository::create`
        // enforces by hand under a write-lock.
        sqlx::query(
            "INSERT INTO products \
             (id, tenant_id, name, sku, price, cost_price, stock, category, \
              low_stock_threshold, created_by) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        )
        .bind(&product.id)
        .bind(&product.tenant_id)
        .bind(&product.name)
        .bind(&product.sku)
        .bind(product.price)
        .bind(product.cost_price)
        .bind(product.stock)
        .bind(&product.category)
        .bind(product.low_stock_threshold)
        .bind(Json(product.created_by.clone()))
        .execute(&self.pool)
        .await
        .is_ok()
    }

    async fn list_by_tenant(&self, tenant_id: &str) -> Vec<Product> {
        sqlx::query_as::<_, ProductRow>(
            "SELECT id, tenant_id, name, sku, price, cost_price, stock, category, \
                    low_stock_threshold, created_by \
             FROM products WHERE tenant_id = $1",
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(Into::into)
        .collect()
    }

    async fn get_by_sku(&self, tenant_id: &str, sku: &str) -> Option<Product> {
        sqlx::query_as::<_, ProductRow>(
            "SELECT id, tenant_id, name, sku, price, cost_price, stock, category, \
                    low_stock_threshold, created_by \
             FROM products WHERE tenant_id = $1 AND sku = $2",
        )
        .bind(tenant_id)
        .bind(sku)
        .fetch_optional(&self.pool)
        .await
        .ok()
        .flatten()
        .map(Into::into)
    }

    async fn get(&self, id: &str) -> Option<Product> {
        sqlx::query_as::<_, ProductRow>(
            "SELECT id, tenant_id, name, sku, price, cost_price, stock, category, \
                    low_stock_threshold, created_by \
             FROM products WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .ok()
        .flatten()
        .map(Into::into)
    }

    async fn update(&self, product: Product) -> bool {
        let result = sqlx::query(
            "UPDATE products SET name = $2, price = $3, cost_price = $4, stock = $5, \
                                   category = $6, low_stock_threshold = $7 \
             WHERE id = $1",
        )
        .bind(&product.id)
        .bind(&product.name)
        .bind(product.price)
        .bind(product.cost_price)
        .bind(product.stock)
        .bind(&product.category)
        .bind(product.low_stock_threshold)
        .execute(&self.pool)
        .await;

        matches!(result, Ok(outcome) if outcome.rows_affected() > 0)
    }

    async fn delete(&self, id: &str) -> bool {
        let result = sqlx::query("DELETE FROM products WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await;

        matches!(result, Ok(outcome) if outcome.rows_affected() > 0)
    }

    async fn reserve_stock(&self, product_id: &str, quantity: i32) -> bool {
        // The `stock >= $2` check lives in the WHERE clause, so a
        // concurrent request for the same product can't both pass a
        // separate "is there enough?" check before either writes —
        // Postgres serializes concurrent UPDATEs to the same row, so this
        // one statement IS the lock.
        let result = sqlx::query(
            "UPDATE products SET stock = stock - $2 \
             WHERE id = $1 AND stock >= $2",
        )
        .bind(product_id)
        .bind(quantity)
        .execute(&self.pool)
        .await;

        matches!(result, Ok(outcome) if outcome.rows_affected() > 0)
    }

    async fn release_stock(&self, product_id: &str, quantity: i32) {
        let _ =
            sqlx::query("UPDATE products SET stock = stock + $2 WHERE id = $1")
                .bind(product_id)
                .bind(quantity)
                .execute(&self.pool)
                .await;
    }
}
