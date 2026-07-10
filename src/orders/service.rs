use crate::error::{AppError, Result};
use crate::products::ProductRepository;
use crate::tenants::TenantRepository;

use super::model::{CreateOrderRequest, Order, OrderItem};
use super::repository::OrderRepository;

pub async fn list_orders<OR, TR>(
    orders: &OR,
    tenants: &TR,
    tenant_id: &str,
) -> Result<Vec<Order>>
where
    OR: OrderRepository,
    TR: TenantRepository,
{
    ensure_tenant_exists(tenants, tenant_id).await?;
    Ok(orders.list_by_tenant(tenant_id).await)
}

pub async fn create_order<OR, PR, TR>(
    orders: &OR,
    products: &PR,
    tenants: &TR,
    tenant_id: &str,
    payload: CreateOrderRequest,
) -> Result<Order>
where
    OR: OrderRepository,
    PR: ProductRepository,
    TR: TenantRepository,
{
    ensure_tenant_exists(tenants, tenant_id).await?;

    if payload.items.is_empty() {
        return Err(AppError::BadRequest(
            "order must have at least one item".into(),
        ));
    }

    // (product_id, quantity) yang sudah berhasil di-reserve, dipakai untuk
    // rollback kalau salah satu item berikutnya gagal di tengah jalan.
    let mut reserved: Vec<(String, i32)> = Vec::new();
    let mut items: Vec<OrderItem> = Vec::new();

    for requested in payload.items {
        if requested.quantity <= 0 {
            rollback(products, &reserved).await;
            return Err(AppError::BadRequest(format!(
                "quantity for sku '{}' must be greater than zero",
                requested.sku
            )));
        }

        // Ambil data product yang sebenarnya dari repository — nama & harga
        // TIDAK diterima dari input client, supaya order selalu konsisten
        // dengan katalog product yang tersimpan.
        let Some(product) =
            products.get_by_sku(tenant_id, &requested.sku).await
        else {
            rollback(products, &reserved).await;
            return Err(AppError::NotFound(format!(
                "product with sku '{}' not found for this tenant",
                requested.sku
            )));
        };

        if !products
            .reserve_stock(&product.id, requested.quantity)
            .await
        {
            rollback(products, &reserved).await;
            return Err(AppError::Conflict(format!(
                "insufficient stock for sku '{}'",
                requested.sku
            )));
        }

        reserved.push((product.id.clone(), requested.quantity));

        items.push(OrderItem {
            sku: product.sku,
            name: product.name,
            quantity: requested.quantity,
            unit_price: product.price,
        });
    }

    let total: f64 = items
        .iter()
        .map(|item| item.quantity as f64 * item.unit_price)
        .sum();

    let order = Order {
        id: format!("order-{}", uuid::Uuid::new_v4().simple()),
        tenant_id: tenant_id.to_string(),
        customer_name: payload.customer_name,
        items,
        total,
    };

    if !orders.create(order.clone()).await {
        rollback(products, &reserved).await;
        return Err(AppError::Conflict("order already exists".into()));
    }

    Ok(order)
}

async fn rollback<PR: ProductRepository>(
    products: &PR,
    reserved: &[(String, i32)],
) {
    for (product_id, quantity) in reserved {
        products.release_stock(product_id, *quantity).await;
    }
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
