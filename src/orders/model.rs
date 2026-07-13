use serde::{Deserialize, Serialize};

use crate::users::Actor;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderItem {
    pub sku: String,
    pub name: String,
    pub quantity: i32,
    pub unit_price: f64,
    /// Snapshot `cost_price` produk pada saat order dibuat — sama alasannya
    /// dengan `unit_price`: kalau `cost_price` produk diubah belakangan,
    /// laporan profit atas order LAMA tidak boleh ikut berubah.
    pub unit_cost: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id: String,
    pub tenant_id: String,
    /// Referensi ke `Customer` yang sudah terdaftar — order TIDAK bisa lagi
    /// dibuat dengan nama pelanggan bebas, harus pelanggan yang sudah ada
    /// di `/customers`.
    pub customer_id: String,
    /// Snapshot nama pelanggan saat order dibuat — sama seperti nama/harga
    /// produk di `OrderItem`, supaya order tetap bisa ditampilkan dengan
    /// benar walau nama pelanggan diubah belakangan atau datanya dihapus.
    pub customer_name: String,
    pub items: Vec<OrderItem>,
    pub total: f64,
    pub created_by: Actor,
    /// Unix timestamp (detik) saat order dibuat — dipakai laporan profit
    /// untuk filter rentang waktu (`from`/`to`).
    pub created_at: u64,
}

#[derive(Debug, Deserialize)]
pub struct CreateOrderItemRequest {
    pub sku: String,
    pub quantity: i32,
}

#[derive(Debug, Deserialize)]
pub struct CreateOrderRequest {
    /// Id customer yang sudah terdaftar (lihat `POST /customers`).
    pub customer_id: String,
    pub items: Vec<CreateOrderItemRequest>,
}
