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

/// Helper: bikin product buat tenant pemilik `token` — tidak ada tenant_id
/// yang dikirim, murni ditentukan dari token. Return id product-nya.
async fn create_product(
    app: &Router,
    token: &str,
    sku: &str,
    price: f64,
    stock: i32,
) -> String {
    let payload = serde_json::json!({
        "name": format!("Produk {sku}"),
        "sku": sku,
        "price": price,
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
    register(&app, "toko-budi", "budi@example.com").await;

    let response = app.oneshot(get_request("/products", None)).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn tenant_data_is_isolated_by_token_not_by_request() {
    let app = test_app();
    let (token_a, _tenant_a) = register(&app, "toko-a", "a@example.com").await;
    let (token_b, _tenant_b) = register(&app, "toko-b", "b@example.com").await;

    create_product(&app, &token_a, "SKU-A", 10_000.0, 5).await;

    // Tidak ada tenant_id yang bisa "ditebak" atau "dipalsukan" dari sisi
    // client — endpoint-nya sama persis (`/products`), tapi token tenant B
    // TIDAK PERNAH bisa melihat product tenant A karena scoping-nya
    // sepenuhnya berasal dari token, bukan dari request.
    let response = app
        .oneshot(get_request("/products", Some(&token_b)))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let products = body_json(response).await;
    assert_eq!(products.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn duplicate_sku_is_rejected() {
    let app = test_app();
    let (token, _tenant_id) =
        register(&app, "toko-budi", "budi@example.com").await;
    create_product(&app, &token, "SKU-001", 10_000.0, 5).await;

    let payload = serde_json::json!({
        "name": "Nama Berbeda",
        "sku": "SKU-001",
        "price": 12_000.0,
        "stock": 3
    });
    let response = app
        .oneshot(json_request("POST", "/products", Some(&token), payload))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn order_uses_real_product_price_and_reduces_stock() {
    let app = test_app();
    let (token, _tenant_id) =
        register(&app, "toko-budi", "budi@example.com").await;
    create_product(&app, &token, "SKU-001", 15_000.0, 10).await;

    let payload = serde_json::json!({
        "customer_name": "Budi",
        "items": [{ "sku": "SKU-001", "quantity": 3 }]
    });
    let response = app
        .clone()
        .oneshot(json_request("POST", "/orders", Some(&token), payload))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
    let order = body_json(response).await;
    assert_eq!(order["total"], 45_000.0);
    assert_eq!(order["items"][0]["unit_price"], 15_000.0);

    let products_response = app
        .oneshot(get_request("/products", Some(&token)))
        .await
        .unwrap();
    let products = body_json(products_response).await;
    assert_eq!(products[0]["stock"], 7);
}

#[tokio::test]
async fn order_with_unknown_sku_returns_not_found() {
    let app = test_app();
    let (token, _tenant_id) =
        register(&app, "toko-budi", "budi@example.com").await;

    let payload = serde_json::json!({
        "customer_name": "Budi",
        "items": [{ "sku": "SKU-TIDAK-ADA", "quantity": 1 }]
    });
    let response = app
        .oneshot(json_request("POST", "/orders", Some(&token), payload))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn order_fails_when_stock_insufficient() {
    let app = test_app();
    let (token, _tenant_id) =
        register(&app, "toko-budi", "budi@example.com").await;
    create_product(&app, &token, "SKU-001", 15_000.0, 2).await;

    let payload = serde_json::json!({
        "customer_name": "Budi",
        "items": [{ "sku": "SKU-001", "quantity": 5 }]
    });
    let response = app
        .clone()
        .oneshot(json_request("POST", "/orders", Some(&token), payload))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CONFLICT);

    let products_response = app
        .oneshot(get_request("/products", Some(&token)))
        .await
        .unwrap();
    let products = body_json(products_response).await;
    assert_eq!(products[0]["stock"], 2);
}

#[tokio::test]
async fn register_persists_tenant_address() {
    let app = test_app();

    let payload = serde_json::json!({
        "tenant_name": "Toko Budi",
        "tenant_slug": "toko-budi",
        "tenant_address": "Jl. Merdeka No. 10, Bandung",
        "email": "budi@example.com",
        "password": "password123"
    });
    let register_response = app
        .clone()
        .oneshot(json_request("POST", "/auth/register", None, payload))
        .await
        .unwrap();
    assert_eq!(register_response.status(), StatusCode::CREATED);
    let token = body_json(register_response).await["token"]
        .as_str()
        .unwrap()
        .to_string();

    let me_response = app
        .oneshot(get_request("/tenants/me", Some(&token)))
        .await
        .unwrap();
    assert_eq!(me_response.status(), StatusCode::OK);
    let tenant = body_json(me_response).await;
    assert_eq!(tenant["address"], "Jl. Merdeka No. 10, Bandung");
}

#[tokio::test]
async fn register_without_address_leaves_it_null() {
    let app = test_app();
    let (token, _tenant_id) =
        register(&app, "toko-tanpa-alamat", "notaddr@example.com").await;

    let me_response = app
        .oneshot(get_request("/tenants/me", Some(&token)))
        .await
        .unwrap();
    let tenant = body_json(me_response).await;
    assert_eq!(tenant["address"], Value::Null);
}

#[tokio::test]
async fn update_product_changes_fields() {
    let app = test_app();
    let (token, _tenant_id) =
        register(&app, "toko-budi", "budi@example.com").await;
    let product_id = create_product(&app, &token, "SKU-001", 10_000.0, 5).await;

    let payload = serde_json::json!({ "price": 20_000.0, "stock": 50 });
    let response = app
        .clone()
        .oneshot(json_request(
            "PATCH",
            &format!("/products/{product_id}"),
            Some(&token),
            payload,
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let updated = body_json(response).await;
    assert_eq!(updated["price"], 20_000.0);
    assert_eq!(updated["stock"], 50);
    // sku & name yang tidak dikirim harus tetap sama seperti sebelumnya.
    assert_eq!(updated["sku"], "SKU-001");
}

#[tokio::test]
async fn cannot_update_other_tenants_product() {
    let app = test_app();
    let (token_a, _) = register(&app, "toko-a", "a@example.com").await;
    let (token_b, _) = register(&app, "toko-b", "b@example.com").await;
    let product_id =
        create_product(&app, &token_a, "SKU-001", 10_000.0, 5).await;

    let payload = serde_json::json!({ "price": 1.0 });
    let response = app
        .oneshot(json_request(
            "PATCH",
            &format!("/products/{product_id}"),
            Some(&token_b),
            payload,
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn delete_product_removes_it_from_list() {
    let app = test_app();
    let (token, _tenant_id) =
        register(&app, "toko-budi", "budi@example.com").await;
    let product_id = create_product(&app, &token, "SKU-001", 10_000.0, 5).await;

    let delete_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/products/{product_id}"))
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(delete_response.status(), StatusCode::NO_CONTENT);

    let list_response = app
        .oneshot(get_request("/products", Some(&token)))
        .await
        .unwrap();
    let products = body_json(list_response).await;
    assert_eq!(products.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn cancel_order_restores_stock_and_removes_order() {
    let app = test_app();
    let (token, _tenant_id) =
        register(&app, "toko-budi", "budi@example.com").await;
    create_product(&app, &token, "SKU-001", 15_000.0, 10).await;

    let order_payload = serde_json::json!({
        "customer_name": "Budi",
        "items": [{ "sku": "SKU-001", "quantity": 4 }]
    });
    let order_response = app
        .clone()
        .oneshot(json_request("POST", "/orders", Some(&token), order_payload))
        .await
        .unwrap();
    let order_id = body_json(order_response).await["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Stock sekarang 6 (10 - 4) sebelum dibatalkan.
    let mid_products = app
        .clone()
        .oneshot(get_request("/products", Some(&token)))
        .await
        .unwrap();
    assert_eq!(body_json(mid_products).await[0]["stock"], 6);

    let cancel_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/orders/{order_id}"))
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(cancel_response.status(), StatusCode::NO_CONTENT);

    // Stock kembali ke 10 setelah dibatalkan.
    let final_products = app
        .clone()
        .oneshot(get_request("/products", Some(&token)))
        .await
        .unwrap();
    assert_eq!(body_json(final_products).await[0]["stock"], 10);

    // Order sudah tidak ada lagi di list.
    let orders_response = app
        .oneshot(get_request("/orders", Some(&token)))
        .await
        .unwrap();
    let orders = body_json(orders_response).await;
    assert_eq!(orders.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn login_is_rate_limited_after_too_many_failures() {
    let app = test_app();
    register(&app, "toko-budi", "budi@example.com").await;

    for _ in 0..5 {
        let payload = serde_json::json!({ "email": "budi@example.com", "password": "salah-terus" });
        let response = app
            .clone()
            .oneshot(json_request("POST", "/auth/login", None, payload))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    let payload = serde_json::json!({ "email": "budi@example.com", "password": "password123" });
    let response = app
        .oneshot(json_request("POST", "/auth/login", None, payload))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn owner_can_invite_staff_and_staff_can_login() {
    let app = test_app();
    let (owner_token, _tenant_id) =
        register(&app, "toko-budi", "owner@example.com").await;

    let payload = serde_json::json!({ "email": "staff@example.com", "password": "password123" });
    let response = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/tenants/me/users",
            Some(&owner_token),
            payload,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let created = body_json(response).await;
    assert_eq!(created["role"], "staff");

    let login_payload = serde_json::json!({ "email": "staff@example.com", "password": "password123" });
    let login_response = app
        .oneshot(json_request("POST", "/auth/login", None, login_payload))
        .await
        .unwrap();
    assert_eq!(login_response.status(), StatusCode::OK);
}

#[tokio::test]
async fn staff_cannot_invite_other_staff() {
    let app = test_app();
    let (owner_token, _tenant_id) =
        register(&app, "toko-budi", "owner@example.com").await;

    let invite_payload = serde_json::json!({ "email": "staff@example.com", "password": "password123" });
    app.clone()
        .oneshot(json_request(
            "POST",
            "/tenants/me/users",
            Some(&owner_token),
            invite_payload,
        ))
        .await
        .unwrap();

    let login_payload = serde_json::json!({ "email": "staff@example.com", "password": "password123" });
    let login_response = app
        .clone()
        .oneshot(json_request("POST", "/auth/login", None, login_payload))
        .await
        .unwrap();
    let staff_token = body_json(login_response).await["token"]
        .as_str()
        .unwrap()
        .to_string();

    let another_invite = serde_json::json!({ "email": "staff2@example.com", "password": "password123" });
    let response = app
        .oneshot(json_request(
            "POST",
            "/tenants/me/users",
            Some(&staff_token),
            another_invite,
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn logout_revokes_the_token() {
    let app = test_app();
    let (token, _tenant_id) =
        register(&app, "toko-budi", "budi@example.com").await;

    let before = app
        .clone()
        .oneshot(get_request("/products", Some(&token)))
        .await
        .unwrap();
    assert_eq!(before.status(), StatusCode::OK);

    let logout_response = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/auth/logout",
            Some(&token),
            serde_json::json!({}),
        ))
        .await
        .unwrap();
    assert_eq!(logout_response.status(), StatusCode::NO_CONTENT);

    let after = app
        .oneshot(get_request("/products", Some(&token)))
        .await
        .unwrap();
    assert_eq!(after.status(), StatusCode::UNAUTHORIZED);
}
