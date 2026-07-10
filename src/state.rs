use std::sync::Arc;

use crate::orders::OrderRepository;
use crate::products::ProductRepository;
use crate::tenants::TenantRepository;

/// State aplikasi, generic atas tipe repository tiap domain.
///
/// Static dispatch (bukan `Arc<dyn Trait>`) supaya tidak ada overhead
/// dynamic dispatch / boxing future — semua di-monomorphize saat compile.
/// Kalau nanti ganti backend (mis. Postgres), cukup buat impl baru dari
/// trait yang sama dan ganti tipe konkret di `main.rs`, tanpa menyentuh
/// handler atau service.
pub struct AppState<TR, PR, OR> {
    pub tenants: TR,
    pub products: PR,
    pub orders: OR,
}

impl<TR, PR, OR> AppState<TR, PR, OR>
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
{
    pub fn new(tenants: TR, products: PR, orders: OR) -> Arc<Self> {
        Arc::new(Self {
            tenants,
            products,
            orders,
        })
    }
}
