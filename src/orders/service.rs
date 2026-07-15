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

    // The customer MUST already be registered (via `/customers`) — same as
    // products, the customer name is no longer accepted as free-form text
    // from the client, so orders always stay consistent with stored
    // customer data and there's no order "misfiled" under a mistyped name.
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

    // (product_id, quantity) pairs successfully reserved so far, used to
    // roll back if one of the next items fails partway through.
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

        // Fetch the actual product data from the repository — name & price
        // are NOT accepted from client input, so orders always stay
        // consistent with the stored product catalog.
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

    let total: i64 = items
        .iter()
        .map(|item| item.quantity as i64 * item.unit_price)
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

/// Cancel an order: delete it and return each item's stock to its
/// respective product. This is the only way to "change" an order —
/// there's intentionally no endpoint to edit the items/quantity of an
/// order that's already been made, because an order is a historical
/// record (like a transaction receipt), not a draft meant to be freely
/// edited. If the order is wrong, cancel it and create a new one.
///
/// Returns the cancelled order — used by the caller to write an audit log
/// entry before its data is gone.
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
        // If the product has already been deleted, its stock doesn't need
        // to be returned (there's nowhere left to store it) — the order is
        // still cancelled, the stock reconciliation just becomes a no-op
        // for that item.
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
