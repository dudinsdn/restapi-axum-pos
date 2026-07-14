use std::collections::HashMap;

use crate::error::{AppError, Result};
use crate::orders::{Order, OrderRepository};
use crate::tenants::TenantRepository;

use super::model::{ProductProfit, ProfitReport};

pub async fn profit_report<OR, TR>(
    orders: &OR,
    tenants: &TR,
    tenant_id: &str,
    from: Option<u64>,
    to: Option<u64>,
) -> Result<ProfitReport>
where
    OR: OrderRepository,
    TR: TenantRepository,
{
    ensure_tenant_exists(tenants, tenant_id).await?;

    let all_orders = orders.list_by_tenant(tenant_id).await;

    let mut by_sku: HashMap<String, ProductProfit> = HashMap::new();
    let mut total_revenue = 0.0;
    let mut total_cost = 0.0;
    let mut order_count = 0usize;

    for order in all_orders.iter().filter(|order| in_range(order, from, to)) {
        order_count += 1;

        for item in &order.items {
            let revenue = item.quantity as f64 * item.unit_price;
            let cost = item.quantity as f64 * item.unit_cost;
            total_revenue += revenue;
            total_cost += cost;

            let entry = by_sku.entry(item.sku.clone()).or_insert_with(|| {
                ProductProfit {
                    sku: item.sku.clone(),
                    name: item.name.clone(),
                    quantity_sold: 0,
                    revenue: 0.0,
                    cost: 0.0,
                    profit: 0.0,
                }
            });
            entry.quantity_sold += item.quantity;
            entry.revenue += revenue;
            entry.cost += cost;
            entry.profit += revenue - cost;
        }
    }

    let mut by_product: Vec<ProductProfit> = by_sku.into_values().collect();
    by_product.sort_by(|a, b| b.profit.total_cmp(&a.profit));

    Ok(ProfitReport {
        from,
        to,
        order_count,
        total_revenue,
        total_cost,
        total_profit: total_revenue - total_cost,
        by_product,
    })
}

/// An order is counted if its `created_at` falls within [from, to] — an
/// unset bound is treated as unrestricted (`from` = since the beginning
/// of time, `to` = up to now).
fn in_range(order: &Order, from: Option<u64>, to: Option<u64>) -> bool {
    if let Some(from) = from {
        if order.created_at < from {
            return false;
        }
    }
    if let Some(to) = to {
        if order.created_at > to {
            return false;
        }
    }
    true
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
