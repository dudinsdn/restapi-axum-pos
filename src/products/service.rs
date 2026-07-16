use crate::audit::FieldChange;
use crate::error::{AppError, Result};
use crate::tenants::TenantRepository;
use crate::users::Actor;

use super::model::{
    CreateProductRequest, Product, UpdateProductRequest, DEFAULT_CATEGORY,
    DEFAULT_LOW_STOCK_THRESHOLD,
};
use super::repository::ProductRepository;

pub async fn list_products<PR, TR>(
    products: &PR,
    tenants: &TR,
    tenant_id: &str,
) -> Result<Vec<Product>>
where
    PR: ProductRepository,
    TR: TenantRepository,
{
    ensure_tenant_exists(tenants, tenant_id).await?;
    Ok(products.list_by_tenant(tenant_id).await)
}

/// Products at or below their own `low_stock_threshold` — see
/// `Product::low_stock_threshold` for why that's per-product rather than
/// one tenant-wide number. Reuses `list_products` rather than adding a
/// repository method, since filtering a tenant's (typically modest-sized)
/// catalog in memory is simple and the repository trait stays unchanged
/// either way.
pub async fn list_low_stock_products<PR, TR>(
    products: &PR,
    tenants: &TR,
    tenant_id: &str,
) -> Result<Vec<Product>>
where
    PR: ProductRepository,
    TR: TenantRepository,
{
    let all = list_products(products, tenants, tenant_id).await?;
    Ok(all
        .into_iter()
        .filter(|product| product.stock <= product.low_stock_threshold)
        .collect())
}

pub async fn create_product<PR, TR>(
    products: &PR,
    tenants: &TR,
    tenant_id: &str,
    actor: Actor,
    payload: CreateProductRequest,
) -> Result<Product>
where
    PR: ProductRepository,
    TR: TenantRepository,
{
    ensure_tenant_exists(tenants, tenant_id).await?;
    validate_name(&payload.name)?;
    validate_sku(&payload.sku)?;
    validate_price("price", payload.price)?;
    validate_price("cost_price", payload.cost_price)?;
    validate_stock(payload.stock)?;
    if let Some(category) = &payload.category {
        validate_category(category)?;
    }
    if let Some(threshold) = payload.low_stock_threshold {
        validate_low_stock_threshold(threshold)?;
    }

    let product = Product {
        id: format!("prod-{}", uuid::Uuid::new_v4().simple()),
        tenant_id: tenant_id.to_string(),
        name: payload.name,
        sku: payload.sku,
        price: payload.price,
        cost_price: payload.cost_price,
        stock: payload.stock,
        category: payload
            .category
            .unwrap_or_else(|| DEFAULT_CATEGORY.to_string()),
        low_stock_threshold: payload
            .low_stock_threshold
            .unwrap_or(DEFAULT_LOW_STOCK_THRESHOLD),
        created_by: actor,
    };

    if !products.create(product.clone()).await {
        return Err(AppError::Conflict(format!(
            "sku '{}' already in use for this tenant",
            product.sku
        )));
    }

    Ok(product)
}

pub async fn update_product<PR: ProductRepository>(
    products: &PR,
    tenant_id: &str,
    product_id: &str,
    payload: UpdateProductRequest,
) -> Result<(Product, Vec<FieldChange>)> {
    let mut product =
        fetch_owned_product(products, tenant_id, product_id).await?;
    let mut changes = Vec::new();

    if let Some(name) = payload.name {
        validate_name(&name)?;
        if name != product.name {
            changes.push(FieldChange {
                field: "name".to_string(),
                old_value: product.name.clone(),
                new_value: name.clone(),
            });
            product.name = name;
        }
    }
    if let Some(price) = payload.price {
        validate_price("price", price)?;
        if price != product.price {
            changes.push(FieldChange {
                field: "price".to_string(),
                old_value: product.price.to_string(),
                new_value: price.to_string(),
            });
            product.price = price;
        }
    }
    if let Some(cost_price) = payload.cost_price {
        validate_price("cost_price", cost_price)?;
        if cost_price != product.cost_price {
            changes.push(FieldChange {
                field: "cost_price".to_string(),
                old_value: product.cost_price.to_string(),
                new_value: cost_price.to_string(),
            });
            product.cost_price = cost_price;
        }
    }
    if let Some(stock) = payload.stock {
        validate_stock(stock)?;
        if stock != product.stock {
            changes.push(FieldChange {
                field: "stock".to_string(),
                old_value: product.stock.to_string(),
                new_value: stock.to_string(),
            });
            product.stock = stock;
        }
    }
    if let Some(category) = payload.category {
        validate_category(&category)?;
        if category != product.category {
            changes.push(FieldChange {
                field: "category".to_string(),
                old_value: product.category.clone(),
                new_value: category.clone(),
            });
            product.category = category;
        }
    }
    if let Some(threshold) = payload.low_stock_threshold {
        validate_low_stock_threshold(threshold)?;
        if threshold != product.low_stock_threshold {
            changes.push(FieldChange {
                field: "low_stock_threshold".to_string(),
                old_value: product.low_stock_threshold.to_string(),
                new_value: threshold.to_string(),
            });
            product.low_stock_threshold = threshold;
        }
    }
    // `created_by` is intentionally never changed — it always records who
    // created the product FIRST. Who edited it later is recorded in the
    // audit log, not by overwriting `created_by`.

    // If not a single field actually changed value (e.g. client sent the
    // exact same value), no need to write back to storage — just return as
    // is with empty `changes`.
    if !changes.is_empty() {
        products.update(product.clone()).await;
    }

    Ok((product, changes))
}

/// Returns the deleted product (not just unit) — used by the caller to
/// write an audit log entry with the product's name/sku before its data is gone.
pub async fn delete_product<PR: ProductRepository>(
    products: &PR,
    tenant_id: &str,
    product_id: &str,
) -> Result<Product> {
    let product = fetch_owned_product(products, tenant_id, product_id).await?;
    products.delete(&product.id).await;
    Ok(product)
}

/// Fetch a product by id AND ensure it belongs to the requesting tenant.
/// If the product doesn't exist OR belongs to another tenant, both cases
/// return `NotFound` (not `Forbidden`) — so as not to leak to the client
/// whether that id actually exists but belongs to a different tenant.
async fn fetch_owned_product<PR: ProductRepository>(
    products: &PR,
    tenant_id: &str,
    product_id: &str,
) -> Result<Product> {
    products
        .get(product_id)
        .await
        .filter(|product| product.tenant_id == tenant_id)
        .ok_or_else(|| AppError::NotFound("product not found".into()))
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

fn validate_sku(sku: &str) -> Result<()> {
    if sku.trim().is_empty() {
        return Err(AppError::BadRequest("sku must not be empty".into()));
    }
    Ok(())
}

/// `field_name` is only used in the error message, so the same check works
/// for both `price` and `cost_price` without duplicating the logic.
fn validate_price(field_name: &str, value: i64) -> Result<()> {
    if value < 0 {
        return Err(AppError::BadRequest(format!(
            "{field_name} must not be negative"
        )));
    }
    Ok(())
}

fn validate_stock(stock: i32) -> Result<()> {
    if stock < 0 {
        return Err(AppError::BadRequest("stock must not be negative".into()));
    }
    Ok(())
}

fn validate_category(category: &str) -> Result<()> {
    if category.trim().is_empty() {
        return Err(AppError::BadRequest(
            "category must not be empty".into(),
        ));
    }
    Ok(())
}

fn validate_low_stock_threshold(threshold: i32) -> Result<()> {
    if threshold < 0 {
        return Err(AppError::BadRequest(
            "low_stock_threshold must not be negative".into(),
        ));
    }
    Ok(())
}
