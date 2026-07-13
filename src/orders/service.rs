use std::time::{SystemTime, UNIX_EPOCH};

use crate::customers::CustomerRepository;
use crate::error::{AppError, Result};
use crate::products::ProductRepository;
use crate::tenants::TenantRepository;
use crate::users::Actor;

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

pub async fn create_order<OR, PR, CR, TR>(
    orders: &OR,
    products: &PR,
    customers: &CR,
    tenants: &TR,
    tenant_id: &str,
    actor: Actor,
    payload: CreateOrderRequest,
) -> Result<Order>
where
    OR: OrderRepository,
    PR: ProductRepository,
    CR: CustomerRepository,
    TR: TenantRepository,
{
    ensure_tenant_exists(tenants, tenant_id).await?;

    // Customer HARUS sudah terdaftar (lewat `/customers`) — sama seperti
    // product, nama pelanggan tidak lagi diterima sebagai teks bebas dari
    // client, supaya order selalu konsisten dengan data pelanggan yang
    // tersimpan dan tidak ada order "nyasar" ke pelanggan yang salah ketik.
    let customer = customers
        .get(&payload.customer_id)
        .await
        .filter(|customer| customer.tenant_id == tenant_id)
        .ok_or_else(|| AppError::NotFound("customer not found".into()))?;

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
            unit_cost: product.cost_price,
        });
    }

    let total: f64 = items
        .iter()
        .map(|item| item.quantity as f64 * item.unit_price)
        .sum();

    let order = Order {
        id: format!("order-{}", uuid::Uuid::new_v4().simple()),
        tenant_id: tenant_id.to_string(),
        customer_id: customer.id.clone(),
        customer_name: customer.name.clone(),
        items,
        total,
        created_by: actor,
        created_at: now_unix(),
    };

    if !orders.create(order.clone()).await {
        rollback(products, &reserved).await;
        return Err(AppError::Conflict("order already exists".into()));
    }

    Ok(order)
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

async fn rollback<PR: ProductRepository>(
    products: &PR,
    reserved: &[(String, i32)],
) {
    for (product_id, quantity) in reserved {
        products.release_stock(product_id, *quantity).await;
    }
}

/// Batalkan order: hapus dan kembalikan stock tiap item ke product
/// masing-masing. Ini satu-satunya cara "mengubah" order — sengaja tidak
/// ada endpoint untuk edit item/quantity order yang sudah dibuat, karena
/// order adalah catatan historis (mirip nota transaksi), bukan draft yang
/// pantas diedit bebas. Kalau pesanannya salah, batalkan lalu buat ulang.
///
/// Return order yang dibatalkan — dipakai caller untuk menulis audit log
/// sebelum datanya hilang.
pub async fn cancel_order<OR, PR>(
    orders: &OR,
    products: &PR,
    tenant_id: &str,
    order_id: &str,
) -> Result<Order>
where
    OR: OrderRepository,
    PR: ProductRepository,
{
    let order = orders
        .get(order_id)
        .await
        .filter(|order| order.tenant_id == tenant_id)
        .ok_or_else(|| AppError::NotFound("order not found".into()))?;

    for item in &order.items {
        // Kalau product-nya sudah kadung dihapus duluan, stock tidak perlu
        // dikembalikan (tidak ada lagi tempat menyimpannya) — order tetap
        // batal, reconciliation stock-nya cuma jadi no-op untuk item itu.
        if let Some(product) = products.get_by_sku(tenant_id, &item.sku).await {
            products.release_stock(&product.id, item.quantity).await;
        }
    }

    orders.delete(order_id).await;
    Ok(order)
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
