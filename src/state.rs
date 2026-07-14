use std::sync::Arc;

use crate::audit::AuditLogRepository;
use crate::customers::CustomerRepository;
use crate::orders::OrderRepository;
use crate::products::ProductRepository;
use crate::tenants::TenantRepository;
use crate::users::{LoginRateLimiter, TokenRevocationList, UserRepository};

/// Application state, generic over each domain's repository type.
///
/// Static dispatch (instead of `Arc<dyn Trait>`) so there's no dynamic
/// dispatch / future-boxing overhead — everything is monomorphized at
/// compile time. If the backend is swapped later (e.g. Postgres), just
/// build a new impl of the same trait and change the concrete type in
/// `main.rs`, without touching the handler or service.
pub struct AppState<TR, PR, OR, UR, AR, CR> {
    pub tenants: TR,
    pub products: PR,
    pub orders: OR,
    pub users: UR,
    pub audit: AR,
    pub customers: CR,
    pub jwt_secret: String,
    pub login_rate_limiter: Arc<LoginRateLimiter>,
    pub revoked_tokens: Arc<TokenRevocationList>,
}

impl<TR, PR, OR, UR, AR, CR> AppState<TR, PR, OR, UR, AR, CR>
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
    UR: UserRepository,
    AR: AuditLogRepository,
    CR: CustomerRepository,
{
    pub fn new(
        tenants: TR,
        products: PR,
        orders: OR,
        users: UR,
        audit: AR,
        customers: CR,
        jwt_secret: String,
    ) -> Arc<Self> {
        Arc::new(Self {
            tenants,
            products,
            orders,
            users,
            audit,
            customers,
            jwt_secret,
            login_rate_limiter: Arc::new(LoginRateLimiter::new()),
            revoked_tokens: Arc::new(TokenRevocationList::new()),
        })
    }
}
