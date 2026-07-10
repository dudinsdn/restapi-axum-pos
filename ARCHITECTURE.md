# Rust Axum POS API - Clean Architecture Refactoring

A RESTful Point-of-Sale API built with Axum following **clean architecture** principles. The project is organized by domain boundaries with clear separation of concerns.

## Project Structure

```
src/
├── main.rs               # Entry point, bootstrap
├── lib.rs                # Module exports
├── config.rs             # Configuration from environment
├── error.rs              # Centralized AppError handling
├── state.rs              # AppState with domain services
├── app.rs                # Unified router composition
├── tenants/              # Tenant domain
│   ├── mod.rs           # Module exports
│   ├── model.rs         # Tenant, CreateTenantRequest
│   ├── handler.rs       # Thin HTTP handlers
│   ├── service.rs       # Business logic & validation
│   └── repository.rs    # TenantRepository trait + in-memory impl
├── products/            # Product domain
│   ├── mod.rs
│   ├── model.rs         # Product, CreateProductRequest
│   ├── handler.rs       # Thin HTTP handlers
│   ├── service.rs       # Business logic & validation
│   └── repository.rs    # ProductRepository trait + in-memory impl
└── orders/              # Order domain
    ├── mod.rs
    ├── model.rs         # Order, OrderItem, CreateOrderRequest
    ├── handler.rs       # Thin HTTP handlers
    ├── service.rs       # Business logic & validation
    └── repository.rs    # OrderRepository trait + in-memory impl

tests/
└── api_test.rs          # Integration tests using tower::ServiceExt::oneshot
```

## Architecture Principles

### 1. **Repository Pattern with Traits**

Instead of direct HashMap access in services, each domain uses a repository trait:

```rust
#[async_trait]
pub trait TenantRepository: Send + Sync {
    async fn create(&self, tenant: Tenant) -> Result<()>;
    async fn get(&self, id: &str) -> Result<Tenant>;
    async fn list(&self) -> Result<Vec<Tenant>>;
}

pub struct InMemoryTenantRepository {
    data: RwLock<HashMap<String, Tenant>>,
}
```

**Benefits:**
- Easy to swap implementations (in-memory → database, cache, etc.)
- Testable without spinning up a server
- Defines clear contracts between layers

### 2. **Thin Handlers**

Handlers only handle **HTTP concerns**: extraction, status codes, response mapping.

```rust
pub async fn create_tenant(
    State(service): State<Arc<TenantService>>,
    Json(payload): Json<CreateTenantRequest>,
) -> Result<(StatusCode, Json<Tenant>)> {
    let tenant = service.create_tenant(payload).await?;
    Ok((StatusCode::CREATED, Json(tenant)))
}
```

**Responsibilities:**
- Parse HTTP request bodies
- Extract path parameters
- Call a single service function
- Map response to HTTP status/JSON

### 3. **Service Layer with Business Logic**

All business rules, validation, and orchestration happen in the **service layer**. Services are unit-testable without HTTP.

```rust
pub struct TenantService {
    repository: Arc<dyn TenantRepository>,
}

impl TenantService {
    pub async fn create_tenant(&self, req: CreateTenantRequest) -> Result<Tenant> {
        // Validation: slug uniqueness
        let existing = self.repository.list().await?;
        if existing.iter().any(|t| t.slug == req.slug) {
            return Err(AppError::Conflict(
                format!("slug '{}' already exists", req.slug),
            ));
        }

        let tenant = Tenant {
            id: format!("tenant-{}", Uuid::new_v4().simple()),
            name: req.name,
            slug: req.slug,
            address: req.address,
        };

        self.repository.create(tenant.clone()).await?;
        Ok(tenant)
    }
}
```

**Benefits:**
- Business logic is isolated and testable
- Can be unit-tested without HTTP server
- Reusable by multiple handlers or CLI commands
- Clear audit trail of business decisions

### 4. **Domain-Driven Design**

Each domain (tenants, products, orders) is self-contained:
- Models belong to their domain
- Business logic is domain-specific
- New domains can be added without touching existing code

**Adding a new domain** (e.g., `payments/`):
1. Create `src/payments/` folder
2. Implement: `model.rs`, `repository.rs`, `service.rs`, `handler.rs`, `mod.rs`
3. Initialize service in `state.rs`
4. Register routes in `app.rs`
5. ✅ No changes to existing domains!

### 5. **Centralized Error Handling**

Single `AppError` enum converts to HTTP responses:

```rust
pub enum AppError {
    NotFound(String),
    Conflict(String),
    BadRequest(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let body = Json(json!({ "error": self.to_string() }));
        (status, body).into_response()
    }
}
```

### 6. **Configuration from Environment**

```rust
pub struct Config {
    pub port: u16,
    pub rust_log: String,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            port: std::env::var("PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(3000),
            rust_log: std::env::var("RUST_LOG").unwrap_or_else(|_| "info,axum=debug".into()),
        }
    }
}
```

## Running the Application

### Development
```bash
cargo run
# Or with custom port:
PORT=8080 cargo run
```

### Tests
```bash
# Run integration tests
cargo test

# Run specific test
cargo test test_create_order_calculates_total
```

### Building
```bash
cargo build --release
```

## Example API Endpoints

### Tenants
```
GET  /tenants              # List all tenants
POST /tenants              # Create tenant
```

### Products
```
GET  /tenants/:tenant_id/products      # List products for tenant
POST /tenants/:tenant_id/products      # Create product
```

### Orders
```
GET  /tenants/:tenant_id/orders        # List orders for tenant
POST /tenants/:tenant_id/orders        # Create order
```

### Health
```
GET  /health               # Health check
```

## Key Design Decisions

| Aspect | Decision | Rationale |
|--------|----------|-----------|
| **Repository Pattern** | Traits, not direct HashMap | Enables easy implementation swaps (DB, cache, etc.) |
| **Async-Trait** | Using `#[async_trait]` macro | Trait objects with async methods |
| **In-Memory Storage** | `parking_lot::RwLock` | Lock-free reads, better concurrency than `Mutex` |
| **Error Handling** | Result<T, AppError> | Unified error handling with HTTP mapping |
| **Service Injection** | Arc<Services> in state | Shared, thread-safe service instances |
| **Handler Design** | Thin wrappers | Business logic testable without HTTP |

## Testing Architecture

Integration tests use `tower::ServiceExt::oneshot()` to test the app without:
- Starting an actual server
- Network calls
- Port binding

```rust
#[tokio::test]
async fn test_health_check() {
    let app = restapi_axum_pos::app::create_app();
    
    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .method("GET")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}
```

## Next Steps for Production

To take this architecture to production:

1. **Database Integration**
   - Implement repository traits using SQLx, Diesel, or ORM
   - Add database migrations

2. **Authentication**
   - Add JWT token validation middleware
   - Implement tenant isolation checks

3. **Logging & Monitoring**
   - Integrate structured logging (tracing is already configured)
   - Add metrics collection

4. **API Documentation**
   - Add `utoipa` for automatic OpenAPI/Swagger docs

5. **Validation**
   - Use `validator` crate for request validation
   - Add custom validation rules in services

6. **Caching**
   - Replace `RwLock<HashMap>` with Redis or cache layer
   - Implement cache invalidation strategies

---

**Architecture Style:** Domain-Driven Design + Clean Architecture  
**Framework:** Axum (async Rust web framework)  
**Async Runtime:** Tokio  
**Lock-free Concurrency:** parking_lot  
