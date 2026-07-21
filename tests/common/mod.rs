use std::time::{SystemTime, UNIX_EPOCH};

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde_json::Value;
use tower::ServiceExt;

use restapi_axum_pos::{
    app::create_app, audit::PgAuditLogRepository,
    categories::PgCategoryRepository, customers::PgCustomerRepository,
    orders::PgOrderRepository, products::PgProductRepository,
    state::AppState, tenants::PgTenantRepository, users::PgUserRepository,
};

#[ctor::ctor]
fn init_test_env() {
    dotenvy::dotenv().ok();
}

/// Builds the app against a real Postgres test database. The `PgPool` is
/// injected by `#[sqlx::test]` — one fresh, migrated database per test
/// function, so tests are fully isolated from each other and (unlike the
/// in-memory backend this replaced) actually exercise the same
/// `Pg*Repository` code that runs in production.
pub fn test_app(pool: sqlx::PgPool) -> Router {
    let state = AppState::new(
        PgTenantRepository::new(pool.clone()),
        PgProductRepository::new(pool.clone()),
        PgOrderRepository::new(pool.clone()),
        PgUserRepository::new(pool.clone()),
        PgAuditLogRepository::new(pool.clone()),
        PgCustomerRepository::new(pool.clone()),
        PgCategoryRepository::new(pool),
        "test-secret".to_string(),
    );
    create_app(state)
}

pub fn json_request(
    method: &str,
    uri: &str,
    token: Option<&str>,
    body: Value,
) -> Request<Body> {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json");

    if let Some(token) = token {
        builder = builder.header("authorization", format!("Bearer {token}"));
    }

    builder.body(Body::from(body.to_string())).unwrap()
}

pub fn get_request(uri: &str, token: Option<&str>) -> Request<Body> {
    let mut builder = Request::builder().method("GET").uri(uri);
    if let Some(token) = token {
        builder = builder.header("authorization", format!("Bearer {token}"));
    }
    builder.body(Body::empty()).unwrap()
}

pub fn json_request_with_header(
    method: &str,
    uri: &str,
    token: Option<&str>,
    extra_header: (&str, &str),
    body: Value,
) -> Request<Body> {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .header(extra_header.0, extra_header.1);

    if let Some(token) = token {
        builder = builder.header("authorization", format!("Bearer {token}"));
    }

    builder.body(Body::from(body.to_string())).unwrap()
}

pub async fn body_json(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

/// Helper: register a new tenant + owner, returns (token, tenant_id).
pub async fn register(
    app: &Router,
    slug: &str,
    email: &str,
) -> (String, String) {
    let payload = serde_json::json!({
        "tenant_name": "Toko Test",
        "tenant_slug": slug,
        "name": "Budi Owner",
        "email": email,
        "password": "password123"
    });
    let response = app
        .clone()
        .oneshot(json_request("POST", "/auth/register", None, payload))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    let body = body_json(response).await;
    let token = body["token"].as_str().unwrap().to_string();
    let tenant_id = body["user"]["tenant_id"].as_str().unwrap().to_string();
    (token, tenant_id)
}

/// Helper: create a product for the tenant owning `token` — no tenant_id
/// is sent, it's determined purely from the token. Returns the product's id.
pub async fn create_product(
    app: &Router,
    token: &str,
    sku: &str,
    price: i64,
    cost_price: i64,
    stock: i32,
) -> String {
    let payload = serde_json::json!({
        "name": format!("Produk {sku}"),
        "sku": sku,
        "price": price,
        "cost_price": cost_price,
        "stock": stock
    });
    let response = app
        .clone()
        .oneshot(json_request("POST", "/products", Some(token), payload))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    body_json(response).await["id"]
        .as_str()
        .unwrap()
        .to_string()
}

/// Helper: create a customer for the tenant owning `token`. Returns the
/// customer's id, used as `customer_id` when creating an order.
pub async fn create_customer(app: &Router, token: &str, name: &str) -> String {
    let payload = serde_json::json!({
        "name": name,
        "phone": format!("08{}", uuid::Uuid::new_v4().simple())
            .chars()
            .take(12)
            .collect::<String>()
    });
    let response = app
        .clone()
        .oneshot(json_request("POST", "/customers", Some(token), payload))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    body_json(response).await["id"]
        .as_str()
        .unwrap()
        .to_string()
}

/// Helper: the owner (holder of `owner_token`) invites a new user with a
/// given `role` ("admin" or "cashier"), then immediately logs in as that
/// user. Returns its token.
pub async fn invite_and_login(
    app: &Router,
    owner_token: &str,
    email: &str,
    role: &str,
) -> String {
    let invite_payload = serde_json::json!({
        "name": "Invited User",
        "email": email,
        "password": "password123",
        "role": role
    });
    let response = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/tenants/me/users",
            Some(owner_token),
            invite_payload,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    let login_payload =
        serde_json::json!({ "email": email, "password": "password123" });
    let login_response = app
        .clone()
        .oneshot(json_request("POST", "/auth/login", None, login_payload))
        .await
        .unwrap();
    body_json(login_response).await["token"]
        .as_str()
        .unwrap()
        .to_string()
}

/// Helper: current unix timestamp in seconds, used for date-range filtering
/// tests (e.g. the profit report's `from`/`to` filters).
pub fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}
