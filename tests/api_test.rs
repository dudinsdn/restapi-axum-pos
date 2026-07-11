use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde_json::Value;
use tower::ServiceExt;

use restapi_axum_pos::{
    app::create_app, orders::InMemoryOrderRepository,
    products::InMemoryProductRepository, state::AppState,
    tenants::InMemoryTenantRepository, users::InMemoryUserRepository,
};

fn test_app() -> Router {
    let state = AppState::new(
        InMemoryTenantRepository::new(),
        InMemoryProductRepository::new(),
        InMemoryOrderRepository::new(),
        InMemoryUserRepository::new(),
        "test-secret".to_string(),
    );
    create_app(state)
}

fn json_request(
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

fn get_request(uri: &str, token: Option<&str>) -> Request<Body> {
    let mut builder = Request::builder().method("GET").uri(uri);
    if let Some(token) = token {
        builder = builder.header("authorization", format!("Bearer {token}"));
    }
    builder.body(Body::empty()).unwrap()
}

async fn body_json(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

/// Helper: register tenant + owner baru, return (token, tenant_id).
async fn register(app: &Router, slug: &str, email: &str) -> (String, String) {
    let payload = serde_json::json!({
        "tenant_name": "Toko Test",
        "tenant_slug": slug,
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

async fn create_product(
    app: &Router,
    tenant_id: &str,
    token: &str,
    sku: &str,
    price: f64,
    stock: i32,
) {
    let payload = serde_json::json!({
        "name": format!("Produk {sku}"),
        "sku": sku,
        "price": price,
        "stock": stock
    });
    let response = app
        .clone()
        .oneshot(json_request(
            "POST",
            &format!("/tenants/{tenant_id}/products"),
            Some(token),
            payload,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn health_check_returns_ok() {
    let app = test_app();
    let response = app.oneshot(get_request("/health", None)).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn register_creates_tenant_and_owner_with_token() {
    let app = test_app();
    let (token, _tenant_id) =
        register(&app, "toko-budi", "budi@example.com").await;
    assert!(!token.is_empty());
}

#[tokio::test]
async fn duplicate_slug_on_register_is_rejected() {
    let app = test_app();
    register(&app, "toko-budi", "budi@example.com").await;

    let payload = serde_json::json!({
        "tenant_name": "Toko Lain",
        "tenant_slug": "toko-budi",
        "email": "lain@example.com",
        "password": "password123"
    });
    let response = app
        .oneshot(json_request("POST", "/auth/register", None, payload))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn duplicate_email_on_register_is_rejected() {
    let app = test_app();
    register(&app, "toko-budi", "budi@example.com").await;

    let payload = serde_json::json!({
        "tenant_name": "Toko Lain",
        "tenant_slug": "toko-lain",
        "email": "budi@example.com",
        "password": "password123"
    });
    let response = app
        .oneshot(json_request("POST", "/auth/register", None, payload))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn login_with_correct_credentials_returns_token() {
    let app = test_app();
    register(&app, "toko-budi", "budi@example.com").await;

    let payload = serde_json::json!({ "email": "budi@example.com", "password": "password123" });
    let response = app
        .oneshot(json_request("POST", "/auth/login", None, payload))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    assert!(body["token"].as_str().unwrap().len() > 0);
}

#[tokio::test]
async fn login_with_wrong_password_is_unauthorized() {
    let app = test_app();
    register(&app, "toko-budi", "budi@example.com").await;

    let payload = serde_json::json!({ "email": "budi@example.com", "password": "salah-password" });
    let response = app
        .oneshot(json_request("POST", "/auth/login", None, payload))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn products_endpoint_requires_auth() {
    let app = test_app();
    let (_token, tenant_id) =
        register(&app, "toko-budi", "budi@example.com").await;

    let response = app
        .oneshot(get_request(&format!("/tenants/{tenant_id}/products"), None))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn cannot_access_other_tenants_data() {
    let app = test_app();
    let (token_a, _tenant_a) = register(&app, "toko-a", "a@example.com").await;
    let (_token_b, tenant_b) = register(&app, "toko-b", "b@example.com").await;

    // Token milik tenant A dipakai buat akses data tenant B -> harus ditolak.
    let response = app
        .oneshot(get_request(
            &format!("/tenants/{tenant_b}/products"),
            Some(&token_a),
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn duplicate_sku_is_rejected() {
    let app = test_app();
    let (token, tenant_id) =
        register(&app, "toko-budi", "budi@example.com").await;
    create_product(&app, &tenant_id, &token, "SKU-001", 10_000.0, 5).await;

    let payload = serde_json::json!({
        "name": "Nama Berbeda",
        "sku": "SKU-001",
        "price": 12_000.0,
        "stock": 3
    });
    let response = app
        .oneshot(json_request(
            "POST",
            &format!("/tenants/{tenant_id}/products"),
            Some(&token),
            payload,
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn order_uses_real_product_price_and_reduces_stock() {
    let app = test_app();
    let (token, tenant_id) =
        register(&app, "toko-budi", "budi@example.com").await;
    create_product(&app, &tenant_id, &token, "SKU-001", 15_000.0, 10).await;

    let payload = serde_json::json!({
        "customer_name": "Budi",
        "items": [{ "sku": "SKU-001", "quantity": 3 }]
    });
    let response = app
        .clone()
        .oneshot(json_request(
            "POST",
            &format!("/tenants/{tenant_id}/orders"),
            Some(&token),
            payload,
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
    let order = body_json(response).await;
    assert_eq!(order["total"], 45_000.0);
    assert_eq!(order["items"][0]["unit_price"], 15_000.0);

    let products_response = app
        .oneshot(get_request(
            &format!("/tenants/{tenant_id}/products"),
            Some(&token),
        ))
        .await
        .unwrap();
    let products = body_json(products_response).await;
    assert_eq!(products[0]["stock"], 7);
}

#[tokio::test]
async fn order_with_unknown_sku_returns_not_found() {
    let app = test_app();
    let (token, tenant_id) =
        register(&app, "toko-budi", "budi@example.com").await;

    let payload = serde_json::json!({
        "customer_name": "Budi",
        "items": [{ "sku": "SKU-TIDAK-ADA", "quantity": 1 }]
    });
    let response = app
        .oneshot(json_request(
            "POST",
            &format!("/tenants/{tenant_id}/orders"),
            Some(&token),
            payload,
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn order_fails_when_stock_insufficient() {
    let app = test_app();
    let (token, tenant_id) =
        register(&app, "toko-budi", "budi@example.com").await;
    create_product(&app, &tenant_id, &token, "SKU-001", 15_000.0, 2).await;

    let payload = serde_json::json!({
        "customer_name": "Budi",
        "items": [{ "sku": "SKU-001", "quantity": 5 }]
    });
    let response = app
        .clone()
        .oneshot(json_request(
            "POST",
            &format!("/tenants/{tenant_id}/orders"),
            Some(&token),
            payload,
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CONFLICT);

    let products_response = app
        .oneshot(get_request(
            &format!("/tenants/{tenant_id}/products"),
            Some(&token),
        ))
        .await
        .unwrap();
    let products = body_json(products_response).await;
    assert_eq!(products[0]["stock"], 2);
}
