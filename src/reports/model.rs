use serde::{Deserialize, Serialize};

/// Query param untuk filter rentang waktu laporan. Keduanya opsional dan
/// berupa unix timestamp (detik) — konsisten dengan `AuditLogEntry::at`,
/// bukan format tanggal supaya tidak perlu tambah dependency date/time.
#[derive(Debug, Deserialize)]
pub struct ProfitReportQuery {
    pub from: Option<u64>,
    pub to: Option<u64>,
}

/// Rincian kontribusi profit satu produk dalam rentang laporan.
#[derive(Debug, Clone, Serialize)]
pub struct ProductProfit {
    pub sku: String,
    pub name: String,
    pub quantity_sold: i32,
    pub revenue: f64,
    pub cost: f64,
    pub profit: f64,
}

/// Laporan profit (pendapatan dikurangi HPP/harga beli) untuk satu tenant,
/// dihitung dari order yang sudah dibuat. Order yang dibatalkan tidak ikut
/// terhitung karena `cancel_order` MENGHAPUS order-nya (lihat
/// `orders::service::cancel_order`) — jadi setiap order yang masih ada di
/// storage sudah pasti transaksi yang valid, tidak perlu filter status.
#[derive(Debug, Serialize)]
pub struct ProfitReport {
    /// Filter yang benar-benar dipakai untuk laporan ini (`null` kalau
    /// tidak difilter dari sisi itu) — supaya response self-descriptive.
    pub from: Option<u64>,
    pub to: Option<u64>,
    pub order_count: usize,
    pub total_revenue: f64,
    pub total_cost: f64,
    pub total_profit: f64,
    /// Diurutkan dari kontribusi profit terbesar, supaya owner langsung
    /// lihat produk paling menguntungkan tanpa perlu sort sendiri.
    pub by_product: Vec<ProductProfit>,
}
