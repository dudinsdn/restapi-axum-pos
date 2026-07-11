use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub id: String,
    pub tenant_id: String,
    pub name: String,
    pub sku: String,
    pub price: f64,
    pub stock: i32,
}

#[derive(Debug, Deserialize)]
pub struct CreateProductRequest {
    pub name: String,
    pub sku: String,
    pub price: f64,
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
    pub stock: Option<i32>,
}
