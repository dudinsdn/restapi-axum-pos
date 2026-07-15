use serde::{Deserialize, Serialize};

/// Query params for filtering the report's time range. Both are optional
/// and use unix timestamp (seconds) — consistent with `AuditLogEntry::at`,
/// not a date format, so no date/time dependency needs to be added.
#[derive(Debug, Deserialize)]
pub struct ProfitReportQuery {
    pub from: Option<u64>,
    pub to: Option<u64>,
}

/// Breakdown of one product's profit contribution within the report range.
#[derive(Debug, Clone, Serialize)]
pub struct ProductProfit {
    pub sku: String,
    pub name: String,
    pub quantity_sold: i32,
    pub revenue: i64,
    pub cost: i64,
    pub profit: i64,
}

/// Profit report (revenue minus cost of goods) for one tenant, computed
/// from orders that have been created. Cancelled orders aren't counted
/// because `cancel_order` DELETES the order (see
/// `orders::service::cancel_order`) — so any order still in storage is
/// guaranteed to be a valid transaction, no status filter needed.
#[derive(Debug, Serialize)]
pub struct ProfitReport {
    /// The filter actually used for this report (`null` if not filtered
    /// on that side) — so the response is self-descriptive.
    pub from: Option<u64>,
    pub to: Option<u64>,
    pub order_count: usize,
    pub total_revenue: i64,
    pub total_cost: i64,
    pub total_profit: i64,
    /// Sorted by largest profit contribution, so the owner immediately
    /// sees the most profitable products without needing to sort themselves.
    pub by_product: Vec<ProductProfit>,
}
