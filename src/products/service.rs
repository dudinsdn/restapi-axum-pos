use crate::error::{AppError, Result};
use crate::tenants::TenantRepository;

use super::model::{CreateProductRequest, Product};
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

async fn ensure_tenant_exists<TR: TenantRepository>(
    tenants: &TR,
    tenant_id: &str,
) -> Result<()> {
    if tenants.get(tenant_id).await.is_none() {
        return Err(AppError::NotFound("tenant not found".into()));
    }
    Ok(())
}
