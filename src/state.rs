use std::sync::Arc;

use crate::orders::OrderRepository;
use crate::products::ProductRepository;
use crate::tenants::TenantRepository;
use crate::users::{LoginRateLimiter, TokenRevocationList, UserRepository};

/// State aplikasi, generic atas tipe repository tiap domain.
///
/// Static dispatch (bukan `Arc<dyn Trait>`) supaya tidak ada overhead
/// dynamic dispatch / boxing future — semua di-monomorphize saat compile.
/// Kalau nanti ganti backend (mis. Postgres), cukup buat impl baru dari
/// trait yang sama dan ganti tipe konkret di `main.rs`, tanpa menyentuh
/// handler atau service.
pub struct AppState<TR, PR, OR, UR> {
    pub tenants: TR,
    pub products: PR,
    pub orders: OR,
    pub users: UR,
    pub jwt_secret: String,
    pub login_rate_limiter: Arc<LoginRateLimiter>,
    pub revoked_tokens: Arc<TokenRevocationList>,
}

impl<TR, PR, OR, UR> AppState<TR, PR, OR, UR>
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
    UR: UserRepository,
{
    pub fn new(
        tenants: TR,
        products: PR,
        orders: OR,
        users: UR,
        jwt_secret: String,
    ) -> Arc<Self> {
        Arc::new(Self {
            tenants,
            products,
            orders,
            users,
            jwt_secret,
            login_rate_limiter: Arc::new(LoginRateLimiter::new()),
            revoked_tokens: Arc::new(TokenRevocationList::new()),
        })
    }
}
