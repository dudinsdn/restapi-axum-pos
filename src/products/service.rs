use crate::error::{AppError, Result};
use crate::tenants::TenantRepository;

use super::model::{CreateProductRequest, Product, UpdateProductRequest};
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

pub async fn create_product<PR, TR>(
    products: &PR,
    tenants: &TR,
    tenant_id: &str,
    payload: CreateProductRequest,
) -> Result<Product>
where
    PR: ProductRepository,
    TR: TenantRepository,
{
    ensure_tenant_exists(tenants, tenant_id).await?;

    let product = Product {
        id: format!("prod-{}", uuid::Uuid::new_v4().simple()),
        tenant_id: tenant_id.to_string(),
        name: payload.name,
        sku: payload.sku,
        price: payload.price,
        stock: payload.stock,
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
) -> Result<Product> {
    let mut product =
        fetch_owned_product(products, tenant_id, product_id).await?;

    if let Some(name) = payload.name {
        product.name = name;
    }
    if let Some(price) = payload.price {
        product.price = price;
    }
    if let Some(stock) = payload.stock {
        product.stock = stock;
    }

    products.update(product.clone()).await;
    Ok(product)
}

pub async fn delete_product<PR: ProductRepository>(
    products: &PR,
    tenant_id: &str,
    product_id: &str,
) -> Result<()> {
    let product = fetch_owned_product(products, tenant_id, product_id).await?;
    products.delete(&product.id).await;
    Ok(())
}

/// Ambil product by id DAN pastikan milik tenant yang meminta. Kalau
/// product tidak ada ATAU milik tenant lain, sama-sama return `NotFound`
/// (bukan `Forbidden`) — supaya tidak bocorkan ke client apakah id itu
/// sebenarnya ada tapi kepunyaan tenant lain.
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
