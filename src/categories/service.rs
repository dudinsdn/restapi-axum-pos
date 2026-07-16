use crate::audit::FieldChange;
use crate::error::{AppError, Result};
use crate::products::{Product, ProductRepository};
use crate::tenants::TenantRepository;
use crate::users::Actor;

use super::model::{Category, CreateCategoryRequest, UpdateCategoryRequest};
use super::repository::CategoryRepository;

pub async fn list_categories<KR, TR>(
    categories: &KR,
    tenants: &TR,
    tenant_id: &str,
) -> Result<Vec<Category>>
where
    KR: CategoryRepository,
    TR: TenantRepository,
{
    ensure_tenant_exists(tenants, tenant_id).await?;
    Ok(categories.list_by_tenant(tenant_id).await)
}

pub async fn get_category<KR: CategoryRepository>(
    categories: &KR,
    tenant_id: &str,
    category_id: &str,
) -> Result<Category> {
    fetch_owned_category(categories, tenant_id, category_id).await
}

pub async fn create_category<KR, TR>(
    categories: &KR,
    tenants: &TR,
    tenant_id: &str,
    actor: Actor,
    payload: CreateCategoryRequest,
) -> Result<Category>
where
    KR: CategoryRepository,
    TR: TenantRepository,
{
    ensure_tenant_exists(tenants, tenant_id).await?;
    validate_name(&payload.name)?;

    let category = Category {
        id: format!("cat-{}", uuid::Uuid::new_v4().simple()),
        tenant_id: tenant_id.to_string(),
        name: payload.name,
        created_by: actor,
    };

    if !categories.create(category.clone()).await {
        return Err(AppError::Conflict(format!(
            "category '{}' already exists for this tenant",
            category.name
        )));
    }

    Ok(category)
}

pub async fn update_category<KR: CategoryRepository>(
    categories: &KR,
    tenant_id: &str,
    category_id: &str,
    payload: UpdateCategoryRequest,
) -> Result<(Category, Vec<FieldChange>)> {
    let mut category =
        fetch_owned_category(categories, tenant_id, category_id).await?;
    let mut changes = Vec::new();

    if let Some(name) = payload.name {
        validate_name(&name)?;
        if !name.eq_ignore_ascii_case(&category.name) {
            if let Some(existing) =
                categories.get_by_name(tenant_id, &name).await
            {
                if existing.id != category.id {
                    return Err(AppError::Conflict(format!(
                        "category '{name}' already exists for this tenant"
                    )));
                }
            }
            changes.push(FieldChange {
                field: "name".to_string(),
                old_value: category.name.clone(),
                new_value: name.clone(),
            });
            category.name = name;
        }
    }
    // `created_by` is intentionally never changed, same as `Product` and
    // `Customer` — it always records who created the data FIRST.

    if !changes.is_empty() {
        categories.update(category.clone()).await;
    }

    Ok((category, changes))
}

/// Returns the deleted category (not just unit) — used by the caller to
/// write an audit log entry with the category's name before its data is
/// gone.
///
/// Also orphans any product currently referencing this category: their
/// `category_id` is cleared back to `None`, but `Product::category` (the
/// denormalized display name) is left untouched — same trade-off as
/// `Order::customer_name` surviving a deleted `Customer`. This is done
/// explicitly here (rather than relying solely on Postgres's
/// `ON DELETE SET NULL`) so the in-memory backend, which has no real FK,
/// behaves identically.
pub async fn delete_category<KR, PR>(
    categories: &KR,
    products: &PR,
    tenant_id: &str,
    category_id: &str,
) -> Result<Category>
where
    KR: CategoryRepository,
    PR: ProductRepository,
{
    let category =
        fetch_owned_category(categories, tenant_id, category_id).await?;
    categories.delete(&category.id).await;

    let affected_products = products.list_by_tenant(tenant_id).await;
    for mut product in affected_products {
        if product.category_id.as_deref() == Some(category.id.as_str()) {
            product.category_id = None;
            products.update(product).await;
        }
    }

    Ok(category)
}

/// Products currently referencing this category via `Product::category_id`
/// — this is the "look up by product" side of category CRUD: given a
/// category, find what's in it.
pub async fn list_products_in_category<KR, PR, TR>(
    categories: &KR,
    products: &PR,
    tenants: &TR,
    tenant_id: &str,
    category_id: &str,
) -> Result<Vec<Product>>
where
    KR: CategoryRepository,
    PR: ProductRepository,
    TR: TenantRepository,
{
    let category =
        fetch_owned_category(categories, tenant_id, category_id).await?;
    ensure_tenant_exists(tenants, tenant_id).await?;

    let all = products.list_by_tenant(tenant_id).await;
    Ok(all
        .into_iter()
        .filter(|product| {
            product.category_id.as_deref() == Some(category.id.as_str())
        })
        .collect())
}

/// Fetch a category by id AND ensure it belongs to the requesting tenant.
/// If the category doesn't exist OR belongs to another tenant, both cases
/// return `NotFound` (not `Forbidden`) — so as not to leak to the client
/// whether that id actually exists but belongs to a different tenant.
async fn fetch_owned_category<KR: CategoryRepository>(
    categories: &KR,
    tenant_id: &str,
    category_id: &str,
) -> Result<Category> {
    categories
        .get(category_id)
        .await
        .filter(|category| category.tenant_id == tenant_id)
        .ok_or_else(|| AppError::NotFound("category not found".into()))
}

async fn ensure_tenant_exists<TR: TenantRepository>(
    tenants: &TR,
    tenant_id: &str,
) -> Result<()> {
    if tenants.get(tenant_id).await.is_none() {
        return Err(AppError::NotFound("tenant not found".into()));
    }
    Ok(())
}

fn validate_name(name: &str) -> Result<()> {
    if name.trim().is_empty() {
        return Err(AppError::BadRequest("name must not be empty".into()));
    }
    Ok(())
}
