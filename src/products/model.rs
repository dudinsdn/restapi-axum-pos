use serde::{Deserialize, Serialize};

use crate::users::Actor;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub id: String,
    pub tenant_id: String,
    pub name: String,
    pub sku: String,
    pub price: f64,
    /// Harga beli (HPP) — dasar perhitungan laporan profit. Bukan `price`
    /// jual yang dilihat pelanggan, jadi tetap ditampilkan di endpoint
    /// produk biasa (siapa saja yang boleh lihat produk, boleh lihat ini),
    /// tapi laporan yang MENGOLAHNYA jadi angka profit dibatasi ke owner
    /// lewat `OwnerUser` di endpoint `/tenants/me/reports/profit`.
    pub cost_price: f64,
    pub stock: i32,
    pub created_by: Actor,
}

#[derive(Debug, Deserialize)]
pub struct CreateProductRequest {
    pub name: String,
    pub sku: String,
    pub price: f64,
    pub cost_price: f64,
    pub stock: i32,
}

/// Update sebagian (semua field opsional). `sku` sengaja TIDAK bisa
/// diubah lewat sini — sku dianggap identifier tetap begitu product
/// dibuat, supaya order historis yang menyimpan sku sebagai snapshot
/// tidak jadi ambigu.
#[derive(Debug, Deserialize)]
pub struct UpdateProductRequest {
    pub name: Option<String>,
    pub price: Option<f64>,
    pub cost_price: Option<f64>,
    pub stock: Option<i32>,
}
