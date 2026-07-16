use sqlx::PgPool;
use sqlx::types::Json;

use crate::users::Actor;

use super::model::Category;
use super::repository::CategoryRepository;

#[derive(Debug, Clone, sqlx::FromRow)]
struct CategoryRow {
    id: String,
    tenant_id: String,
    name: String,
    created_by: Json<Actor>,
}

impl From<CategoryRow> for Category {
    fn from(row: CategoryRow) -> Self {
        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            name: row.name,
            created_by: row.created_by.0,
        }
    }
}

/// Postgres-backed `CategoryRepository`. See `PgTenantRepository` for the
/// general shape of this pattern.
#[derive(Debug, Clone)]
pub struct PgCategoryRepository {
    pool: PgPool,
}

impl PgCategoryRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl CategoryRepository for PgCategoryRepository {
    async fn create(&self, category: Category) -> bool {
        // `(tenant_id, LOWER(name))` has a UNIQUE constraint (see the
        // migration), so the INSERT itself enforces the same
        // case-insensitive uniqueness rule
        // `InMemoryCategoryRepository::create` enforces under a
        // write-lock.
        sqlx::query(
            "INSERT INTO categories (id, tenant_id, name, created_by) \
             VALUES ($1, $2, $3, $4)",
        )
        .bind(&category.id)
        .bind(&category.tenant_id)
        .bind(&category.name)
        .bind(Json(category.created_by.clone()))
        .execute(&self.pool)
        .await
        .is_ok()
    }

    async fn list_by_tenant(&self, tenant_id: &str) -> Vec<Category> {
        sqlx::query_as::<_, CategoryRow>(
            "SELECT id, tenant_id, name, created_by \
             FROM categories WHERE tenant_id = $1",
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(Into::into)
        .collect()
    }

    async fn get(&self, id: &str) -> Option<Category> {
        sqlx::query_as::<_, CategoryRow>(
            "SELECT id, tenant_id, name, created_by \
             FROM categories WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .ok()
        .flatten()
        .map(Into::into)
    }

    async fn get_by_name(
        &self,
        tenant_id: &str,
        name: &str,
    ) -> Option<Category> {
        sqlx::query_as::<_, CategoryRow>(
            "SELECT id, tenant_id, name, created_by \
             FROM categories WHERE tenant_id = $1 AND LOWER(name) = LOWER($2)",
        )
        .bind(tenant_id)
        .bind(name)
        .fetch_optional(&self.pool)
        .await
        .ok()
        .flatten()
        .map(Into::into)
    }

    async fn update(&self, category: Category) -> bool {
        let result =
            sqlx::query("UPDATE categories SET name = $2 WHERE id = $1")
                .bind(&category.id)
                .bind(&category.name)
                .execute(&self.pool)
                .await;

        matches!(result, Ok(outcome) if outcome.rows_affected() > 0)
    }

    async fn delete(&self, id: &str) -> bool {
        let result = sqlx::query("DELETE FROM categories WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await;

        matches!(result, Ok(outcome) if outcome.rows_affected() > 0)
    }
}
