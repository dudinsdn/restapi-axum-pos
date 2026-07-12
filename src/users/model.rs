use serde::{Deserialize, Serialize};

/// Tiga tingkat akses:
/// - `Owner`: pemilik tenant, dibuat otomatis saat `register`. Bisa
///   melakukan apa saja, termasuk mengundang `Admin`/`Cashier` baru.
/// - `Admin`: mengelola operasional toko sehari-hari — atur katalog produk,
///   batalkan order, lihat audit log. Tidak bisa mengundang user lain.
/// - `Cashier`: kasir, cuma boleh lihat produk & buat order (transaksi
///   jualan). Tidak bisa mengubah katalog, membatalkan order, atau melihat
///   audit log.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Owner,
    Admin,
    Cashier,
}

#[derive(Debug, Clone, Serialize)]
pub struct User {
    pub id: String,
    pub tenant_id: String,
    pub name: String,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub role: Role,
}

/// Identitas ringkas seorang user, ditempelkan ke resource (product, order,
/// dst) dan ke audit log — supaya selalu jelas siapa yang melakukan apa,
/// bahkan setelah resource aslinya dihapus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Actor {
    pub user_id: String,
    pub name: String,
}

/// Registrasi sekaligus membuat tenant baru + user pertama sebagai Owner.
/// Ini satu-satunya cara membuat tenant sekarang — tidak ada lagi endpoint
/// publik untuk create tenant secara terpisah, supaya tidak ada celah
/// "siapa saja bisa bikin tenant atas nama siapa saja".
#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub tenant_name: String,
    pub tenant_slug: String,
    pub tenant_address: Option<String>,
    pub name: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

/// Owner mengundang user baru (Admin atau Cashier) ke tenant-nya sendiri.
/// `tenant_id` TIDAK diterima dari body — selalu diambil dari tenant milik
/// pemanggil (`AuthUser`), supaya owner tidak bisa iseng invite ke tenant
/// lain. `role` divalidasi di service: tidak boleh `Owner` (cuma ada satu
/// owner per tenant, dibuat lewat `register`).
#[derive(Debug, Deserialize)]
pub struct InviteStaffRequest {
    pub name: String,
    pub email: String,
    pub password: String,
    pub role: Role,
}

#[derive(Debug, Serialize)]
pub struct PublicUser {
    pub id: String,
    pub tenant_id: String,
    pub name: String,
    pub email: String,
    pub role: Role,
}

impl From<User> for PublicUser {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            tenant_id: user.tenant_id,
            name: user.name,
            email: user.email,
            role: user.role,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: PublicUser,
}
