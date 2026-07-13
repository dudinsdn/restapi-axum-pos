use crate::audit::FieldChange;
use crate::error::{AppError, Result};
use crate::tenants::TenantRepository;
use crate::users::Actor;

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
    actor: Actor,
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
        cost_price: payload.cost_price,
        stock: payload.stock,
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
        if stock != product.stock {
            changes.push(FieldChange {
                field: "stock".to_string(),
                old_value: product.stock.to_string(),
                new_value: stock.to_string(),
            });
            product.stock = stock;
        }
    }
    // `created_by` sengaja tidak berubah — itu tetap mencatat siapa yang
    // PERTAMA KALI bikin produknya. Siapa yang mengedit belakangan tercatat
    // di audit log, bukan menimpa `created_by`.

    // Kalau tidak ada satu pun field yang benar-benar berubah nilainya
    // (mis. client kirim value yang sama persis), tidak perlu tulis ulang
    // ke storage — cukup return apa adanya tanpa `changes`.
    if !changes.is_empty() {
        products.update(product.clone()).await;
    }

    Ok((product, changes))
}

/// Return product yang dihapus (bukan cuma unit) — dipakai caller untuk
/// menulis audit log dengan nama/sku produk itu sebelum datanya hilang.
pub async fn delete_product<PR: ProductRepository>(
    products: &PR,
    tenant_id: &str,
    product_id: &str,
) -> Result<Product> {
    let product = fetch_owned_product(products, tenant_id, product_id).await?;
    products.delete(&product.id).await;
    Ok(product)
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
