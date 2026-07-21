use std::sync::Arc;

use crate::audit::AuditLogRepository;
use crate::categories::CategoryRepository;
use crate::customers::CustomerRepository;
use crate::orders::{IdempotencyStore, OrderRepository};
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
pub struct AppState<TR, PR, OR, UR, AR, CR, KR> {
    pub tenants: TR,
    pub products: PR,
    pub orders: OR,
    pub users: UR,
    pub audit: AR,
    pub customers: CR,
    pub categories: KR,
    pub jwt_secret: String,
    pub login_rate_limiter: Arc<LoginRateLimiter>,
    pub revoked_tokens: Arc<TokenRevocationList>,
    pub idempotency_store: Arc<IdempotencyStore>,
}

pub type DynState<TR, PR, OR, UR, AR, CR, KR> =
    Arc<AppState<TR, PR, OR, UR, AR, CR, KR>>;

pub struct Repositories<TR, PR, OR, UR, AR, CR, KR> {
    pub tenants: TR,
    pub products: PR,
    pub orders: OR,
    pub users: UR,
    pub audit: AR,
    pub customers: CR,
    pub categories: KR,
}

impl<TR, PR, OR, UR, AR, CR, KR> AppState<TR, PR, OR, UR, AR, CR, KR>
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
    UR: UserRepository,
    AR: AuditLogRepository,
    CR: CustomerRepository,
    KR: CategoryRepository,
{
    pub fn new(
        repos: Repositories<TR, PR, OR, UR, AR, CR, KR>,
        jwt_secret: String,
    ) -> Arc<Self> {
        Arc::new(Self {
            tenants: repos.tenants,
            products: repos.products,
            orders: repos.orders,
            users: repos.users,
            audit: repos.audit,
            customers: repos.customers,
            categories: repos.categories,
            jwt_secret,
            login_rate_limiter: Arc::new(LoginRateLimiter::new()),
            revoked_tokens: Arc::new(TokenRevocationList::new()),
            idempotency_store: Arc::new(IdempotencyStore::new()),
        })
    }
}
