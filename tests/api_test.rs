use std::time::{SystemTime, UNIX_EPOCH};

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde_json::Value;
use tower::ServiceExt;

use restapi_axum_pos::{
    app::create_app, audit::InMemoryAuditLogRepository,
    customers::InMemoryCustomerRepository, orders::InMemoryOrderRepository,
    products::InMemoryProductRepository, state::AppState,
    tenants::InMemoryTenantRepository, users::InMemoryUserRepository,
};

fn test_app() -> Router {
    let state = AppState::new(
        InMemoryTenantRepository::new(),
        InMemoryProductRepository::new(),
        InMemoryOrderRepository::new(),
        InMemoryUserRepository::new(),
        InMemoryAuditLogRepository::new(),
        InMemoryCustomerRepository::new(),
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

/// Helper: register a new tenant + owner, returns (token, tenant_id).
async fn register(app: &Router, slug: &str, email: &str) -> (String, String) {
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
async fn create_product(
    app: &Router,
    token: &str,
    sku: &str,
    price: f64,
    cost_price: f64,
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
async fn create_customer(app: &Router, token: &str, name: &str) -> String {
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
async fn invite_and_login(
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
        "name": "Lain Owner",
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
        "name": "Lain Owner",
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

    create_product(&app, &token_a, "SKU-A", 10_000.0, 6_000.0, 5).await;

    // There's no tenant_id that can be "guessed" or "spoofed" from the
    // client side — the endpoint is exactly the same (`/products`), but
    // tenant B's token can NEVER see tenant A's product because the
    // scoping comes entirely from the token, not from the request.
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
    create_product(&app, &token, "SKU-001", 10_000.0, 6_000.0, 5).await;

    let payload = serde_json::json!({
        "name": "Nama Berbeda",
        "sku": "SKU-001",
        "price": 12_000.0,
        "cost_price": 7_000.0,
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
    create_product(&app, &token, "SKU-001", 15_000.0, 9_000.0, 10).await;
    let customer_id = create_customer(&app, &token, "Budi").await;

    let payload = serde_json::json!({
        "customer_id": customer_id,
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
    let customer_id = create_customer(&app, &token, "Budi").await;

    let payload = serde_json::json!({
        "customer_id": customer_id,
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
    create_product(&app, &token, "SKU-001", 15_000.0, 9_000.0, 2).await;
    let customer_id = create_customer(&app, &token, "Budi").await;

    let payload = serde_json::json!({
        "customer_id": customer_id,
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
        "name": "Budi Owner",
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
    let product_id =
        create_product(&app, &token, "SKU-001", 10_000.0, 6_000.0, 5).await;

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
    // sku & name that weren't sent must remain unchanged.
    assert_eq!(updated["sku"], "SKU-001");
}

#[tokio::test]
async fn cannot_update_other_tenants_product() {
    let app = test_app();
    let (token_a, _) = register(&app, "toko-a", "a@example.com").await;
    let (token_b, _) = register(&app, "toko-b", "b@example.com").await;
    let product_id =
        create_product(&app, &token_a, "SKU-001", 10_000.0, 6_000.0, 5).await;

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
    let product_id =
        create_product(&app, &token, "SKU-001", 10_000.0, 6_000.0, 5).await;

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
    create_product(&app, &token, "SKU-001", 15_000.0, 9_000.0, 10).await;
    let customer_id = create_customer(&app, &token, "Budi").await;

    let order_payload = serde_json::json!({
        "customer_id": customer_id,
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

    // Stock is now 6 (10 - 4) before cancellation.
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

    // Stock returns to 10 after cancellation.
    let final_products = app
        .clone()
        .oneshot(get_request("/products", Some(&token)))
        .await
        .unwrap();
    assert_eq!(body_json(final_products).await[0]["stock"], 10);

    // The order is no longer in the list.
    let orders_response = app
        .oneshot(get_request("/orders", Some(&token)))
        .await
        .unwrap();
    let orders = body_json(orders_response).await;
    assert_eq!(orders.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn product_and_order_record_who_created_them() {
    let app = test_app();
    let (token, _tenant_id) =
        register(&app, "toko-budi", "budi@example.com").await;

    let product_response = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/products",
            Some(&token),
            serde_json::json!({
                "name": "Kopi Susu",
                "sku": "SKU-001",
                "price": 15_000.0,
                "cost_price": 9_000.0,
                "stock": 10
            }),
        ))
        .await
        .unwrap();
    let product = body_json(product_response).await;
    assert_eq!(product["created_by"]["name"], "Budi Owner");

    let customer_id = create_customer(&app, &token, "Pelanggan").await;

    let order_response = app
        .oneshot(json_request(
            "POST",
            "/orders",
            Some(&token),
            serde_json::json!({
                "customer_id": customer_id,
                "items": [{ "sku": "SKU-001", "quantity": 1 }]
            }),
        ))
        .await
        .unwrap();
    let order = body_json(order_response).await;
    assert_eq!(order["created_by"]["name"], "Budi Owner");
}

#[tokio::test]
async fn audit_log_records_create_update_and_delete() {
    let app = test_app();
    let (token, _tenant_id) =
        register(&app, "toko-budi", "budi@example.com").await;
    let product_id =
        create_product(&app, &token, "SKU-001", 10_000.0, 6_000.0, 5).await;

    app.clone()
        .oneshot(json_request(
            "PATCH",
            &format!("/products/{product_id}"),
            Some(&token),
            serde_json::json!({ "price": 12_000.0 }),
        ))
        .await
        .unwrap();

    app.clone()
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

    let logs_response = app
        .oneshot(get_request("/tenants/me/audit-logs", Some(&token)))
        .await
        .unwrap();
    assert_eq!(logs_response.status(), StatusCode::OK);
    let logs = body_json(logs_response).await;
    let logs = logs.as_array().unwrap();

    // Newest first: delete, update, create.
    assert_eq!(logs.len(), 3);
    assert_eq!(logs[0]["action"], "deleted");
    assert_eq!(logs[1]["action"], "updated");
    assert_eq!(logs[2]["action"], "created");
    for entry in logs {
        assert_eq!(entry["actor"]["name"], "Budi Owner");
        assert_eq!(entry["resource_type"], "product");
    }
}

#[tokio::test]
async fn audit_logs_are_isolated_per_tenant() {
    let app = test_app();
    let (token_a, _) = register(&app, "toko-a", "a@example.com").await;
    let (token_b, _) = register(&app, "toko-b", "b@example.com").await;

    create_product(&app, &token_a, "SKU-001", 10_000.0, 6_000.0, 5).await;

    let logs_b = app
        .oneshot(get_request("/tenants/me/audit-logs", Some(&token_b)))
        .await
        .unwrap();
    let logs_b = body_json(logs_b).await;
    assert_eq!(logs_b.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn audit_log_records_field_level_changes_on_update() {
    let app = test_app();
    let (token, _tenant_id) =
        register(&app, "toko-budi", "budi@example.com").await;
    let product_id =
        create_product(&app, &token, "SKU-001", 10_000.0, 6_000.0, 5).await;

    app.clone()
        .oneshot(json_request(
            "PATCH",
            &format!("/products/{product_id}"),
            Some(&token),
            serde_json::json!({ "price": 12_000.0, "stock": 20 }),
        ))
        .await
        .unwrap();

    let logs_response = app
        .oneshot(get_request("/tenants/me/audit-logs", Some(&token)))
        .await
        .unwrap();
    let logs = body_json(logs_response).await;
    let logs = logs.as_array().unwrap();

    // logs[0] = update (newest), logs[1] = create.
    let changes = logs[0]["changes"].as_array().unwrap();
    assert_eq!(changes.len(), 2);

    let price_change = changes
        .iter()
        .find(|c| c["field"] == "price")
        .expect("ada perubahan field price");
    assert_eq!(price_change["old_value"], "10000");
    assert_eq!(price_change["new_value"], "12000");

    let stock_change = changes
        .iter()
        .find(|c| c["field"] == "stock")
        .expect("ada perubahan field stock");
    assert_eq!(stock_change["old_value"], "5");
    assert_eq!(stock_change["new_value"], "20");

    // name wasn't sent in the payload -> must not appear in changes.
    assert!(changes.iter().all(|c| c["field"] != "name"));
}

#[tokio::test]
async fn noop_update_does_not_write_audit_entry() {
    let app = test_app();
    let (token, _tenant_id) =
        register(&app, "toko-budi", "budi@example.com").await;
    let product_id =
        create_product(&app, &token, "SKU-001", 10_000.0, 6_000.0, 5).await;

    // Send a price whose VALUE IS EXACTLY THE SAME as the current one.
    let response = app
        .clone()
        .oneshot(json_request(
            "PATCH",
            &format!("/products/{product_id}"),
            Some(&token),
            serde_json::json!({ "price": 10_000.0 }),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let logs_response = app
        .oneshot(get_request("/tenants/me/audit-logs", Some(&token)))
        .await
        .unwrap();
    let logs = body_json(logs_response).await;
    // Only the "created" entry — no additional "updated" entry because no
    // field actually changed.
    assert_eq!(logs.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn audit_log_records_which_fields_changed() {
    let app = test_app();
    let (token, _tenant_id) =
        register(&app, "toko-budi", "budi@example.com").await;
    let product_id =
        create_product(&app, &token, "SKU-001", 10_000.0, 6_000.0, 5).await;

    // Only change price & stock, name isn't sent -> must not appear in
    // changes.
    app.clone()
        .oneshot(json_request(
            "PATCH",
            &format!("/products/{product_id}"),
            Some(&token),
            serde_json::json!({ "price": 12_000.0, "stock": 8 }),
        ))
        .await
        .unwrap();

    let logs_response = app
        .oneshot(get_request("/tenants/me/audit-logs", Some(&token)))
        .await
        .unwrap();
    let logs = body_json(logs_response).await;
    let update_entry = &logs[0];

    assert_eq!(update_entry["action"], "updated");
    let changes = update_entry["changes"].as_array().unwrap();
    assert_eq!(changes.len(), 2);

    let price_change = changes
        .iter()
        .find(|c| c["field"] == "price")
        .expect("price change should be recorded");
    assert_eq!(price_change["old_value"], "10000");
    assert_eq!(price_change["new_value"], "12000");

    let stock_change = changes
        .iter()
        .find(|c| c["field"] == "stock")
        .expect("stock change should be recorded");
    assert_eq!(stock_change["old_value"], "5");
    assert_eq!(stock_change["new_value"], "8");

    // name wasn't sent in the payload -> not considered "changed".
    assert!(changes.iter().all(|c| c["field"] != "name"));
}

#[tokio::test]
async fn no_op_update_does_not_create_audit_entry() {
    let app = test_app();
    let (token, _tenant_id) =
        register(&app, "toko-budi", "budi@example.com").await;
    let product_id =
        create_product(&app, &token, "SKU-001", 10_000.0, 6_000.0, 5).await;

    // Send a value that's EXACTLY THE SAME as the current one -> no real
    // change, so no new audit entry should be added.
    app.clone()
        .oneshot(json_request(
            "PATCH",
            &format!("/products/{product_id}"),
            Some(&token),
            serde_json::json!({ "price": 10_000.0 }),
        ))
        .await
        .unwrap();

    let logs_response = app
        .oneshot(get_request("/tenants/me/audit-logs", Some(&token)))
        .await
        .unwrap();
    let logs = body_json(logs_response).await;
    // Only the "created" entry from create_product earlier, no "updated".
    assert_eq!(logs.as_array().unwrap().len(), 1);
    assert_eq!(logs[0]["action"], "created");
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
async fn owner_can_invite_admin_and_cashier_and_both_can_login() {
    let app = test_app();
    let (owner_token, _tenant_id) =
        register(&app, "toko-budi", "owner@example.com").await;

    let admin_payload = serde_json::json!({
        "name": "Admin Satu", "email": "admin@example.com",
        "password": "password123", "role": "admin"
    });
    let admin_response = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/tenants/me/users",
            Some(&owner_token),
            admin_payload,
        ))
        .await
        .unwrap();
    assert_eq!(admin_response.status(), StatusCode::CREATED);
    let created_admin = body_json(admin_response).await;
    assert_eq!(created_admin["role"], "admin");
    assert_eq!(created_admin["name"], "Admin Satu");

    let cashier_payload = serde_json::json!({
        "name": "Kasir Satu", "email": "kasir@example.com",
        "password": "password123", "role": "cashier"
    });
    let cashier_response = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/tenants/me/users",
            Some(&owner_token),
            cashier_payload,
        ))
        .await
        .unwrap();
    assert_eq!(cashier_response.status(), StatusCode::CREATED);
    let created_cashier = body_json(cashier_response).await;
    assert_eq!(created_cashier["role"], "cashier");

    let login_payload = serde_json::json!({ "email": "admin@example.com", "password": "password123" });
    let login_response = app
        .oneshot(json_request("POST", "/auth/login", None, login_payload))
        .await
        .unwrap();
    assert_eq!(login_response.status(), StatusCode::OK);
}

#[tokio::test]
async fn cannot_invite_a_second_owner() {
    let app = test_app();
    let (owner_token, _tenant_id) =
        register(&app, "toko-budi", "owner@example.com").await;

    let payload = serde_json::json!({
        "name": "Owner Dua", "email": "owner2@example.com",
        "password": "password123", "role": "owner"
    });
    let response = app
        .oneshot(json_request(
            "POST",
            "/tenants/me/users",
            Some(&owner_token),
            payload,
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn only_owner_can_invite_new_users() {
    let app = test_app();
    let (owner_token, _tenant_id) =
        register(&app, "toko-budi", "owner@example.com").await;
    let admin_token =
        invite_and_login(&app, &owner_token, "admin@example.com", "admin")
            .await;
    let cashier_token =
        invite_and_login(&app, &owner_token, "kasir@example.com", "cashier")
            .await;

    let another_invite = serde_json::json!({
        "name": "Kasir Dua", "email": "kasir2@example.com",
        "password": "password123", "role": "cashier"
    });
    let admin_response = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/tenants/me/users",
            Some(&admin_token),
            another_invite.clone(),
        ))
        .await
        .unwrap();
    assert_eq!(admin_response.status(), StatusCode::FORBIDDEN);

    let cashier_response = app
        .oneshot(json_request(
            "POST",
            "/tenants/me/users",
            Some(&cashier_token),
            another_invite,
        ))
        .await
        .unwrap();

    assert_eq!(cashier_response.status(), StatusCode::FORBIDDEN);
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

#[tokio::test]
async fn cashier_can_view_products_and_create_orders() {
    let app = test_app();
    let (owner_token, _tenant_id) =
        register(&app, "toko-budi", "owner@example.com").await;
    create_product(&app, &owner_token, "SKU-001", 15_000.0, 9_000.0, 10).await;
    let customer_id = create_customer(&app, &owner_token, "Pelanggan").await;
    let cashier_token =
        invite_and_login(&app, &owner_token, "kasir@example.com", "cashier")
            .await;

    let list_response = app
        .clone()
        .oneshot(get_request("/products", Some(&cashier_token)))
        .await
        .unwrap();
    assert_eq!(list_response.status(), StatusCode::OK);

    let order_payload = serde_json::json!({
        "customer_id": customer_id,
        "items": [{ "sku": "SKU-001", "quantity": 1 }]
    });
    let order_response = app
        .oneshot(json_request(
            "POST",
            "/orders",
            Some(&cashier_token),
            order_payload,
        ))
        .await
        .unwrap();
    assert_eq!(order_response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn cost_price_is_hidden_from_cashier_but_visible_to_owner_and_admin() {
    let app = test_app();
    let (owner_token, _tenant_id) =
        register(&app, "toko-budi", "owner@example.com").await;
    create_product(&app, &owner_token, "SKU-001", 15_000.0, 9_000.0, 10).await;
    let admin_token =
        invite_and_login(&app, &owner_token, "admin@example.com", "admin")
            .await;
    let cashier_token =
        invite_and_login(&app, &owner_token, "kasir@example.com", "cashier")
            .await;

    let owner_products = body_json(
        app.clone()
            .oneshot(get_request("/products", Some(&owner_token)))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(owner_products[0]["cost_price"], 9_000.0);

    let admin_products = body_json(
        app.clone()
            .oneshot(get_request("/products", Some(&admin_token)))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(admin_products[0]["cost_price"], 9_000.0);

    let cashier_products = body_json(
        app.oneshot(get_request("/products", Some(&cashier_token)))
            .await
            .unwrap(),
    )
    .await;
    // The field should be omitted entirely, not sent as `null`.
    assert!(cashier_products[0].get("cost_price").is_none());
}

#[tokio::test]
async fn order_unit_cost_is_hidden_from_cashier_but_visible_to_owner_and_admin()
{
    let app = test_app();
    let (owner_token, _tenant_id) =
        register(&app, "toko-budi", "owner@example.com").await;
    create_product(&app, &owner_token, "SKU-001", 15_000.0, 9_000.0, 10).await;
    let customer_id = create_customer(&app, &owner_token, "Budi").await;
    let admin_token =
        invite_and_login(&app, &owner_token, "admin@example.com", "admin")
            .await;
    let cashier_token =
        invite_and_login(&app, &owner_token, "kasir@example.com", "cashier")
            .await;

    let create_order_body = serde_json::json!({
        "customer_id": customer_id,
        "items": [{ "sku": "SKU-001", "quantity": 1 }]
    });

    // Owner creates an order and sees unit_cost in the response.
    let owner_order = body_json(
        app.clone()
            .oneshot(json_request(
                "POST",
                "/orders",
                Some(&owner_token),
                create_order_body.clone(),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(owner_order["items"][0]["unit_cost"], 9_000.0);

    // Cashier creates an order (allowed) but unit_cost must not appear in
    // their own response, even though it's the same order data.
    let cashier_order = body_json(
        app.clone()
            .oneshot(json_request(
                "POST",
                "/orders",
                Some(&cashier_token),
                create_order_body,
            ))
            .await
            .unwrap(),
    )
    .await;
    assert!(cashier_order["items"][0].get("unit_cost").is_none());

    // Admin listing orders still sees unit_cost.
    let admin_orders = body_json(
        app.clone()
            .oneshot(get_request("/orders", Some(&admin_token)))
            .await
            .unwrap(),
    )
    .await;
    assert!(
        admin_orders
            .as_array()
            .unwrap()
            .iter()
            .all(|order| order["items"][0]["unit_cost"].is_number())
    );

    // Cashier listing orders never sees unit_cost, including orders
    // created by other roles.
    let cashier_orders = body_json(
        app.oneshot(get_request("/orders", Some(&cashier_token)))
            .await
            .unwrap(),
    )
    .await;
    assert!(
        cashier_orders
            .as_array()
            .unwrap()
            .iter()
            .all(|order| order["items"][0].get("unit_cost").is_none())
    );
}

#[tokio::test]
async fn admin_can_manage_product_catalog_but_cashier_cannot() {
    let app = test_app();
    let (owner_token, _tenant_id) =
        register(&app, "toko-budi", "owner@example.com").await;
    let admin_token =
        invite_and_login(&app, &owner_token, "admin@example.com", "admin")
            .await;
    let cashier_token =
        invite_and_login(&app, &owner_token, "kasir@example.com", "cashier")
            .await;

    let create_payload = serde_json::json!({
        "name": "Produk Baru",
        "sku": "SKU-002",
        "price": 5_000.0,
        "cost_price": 3_000.0,
        "stock": 1
    });
    let admin_create = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/products",
            Some(&admin_token),
            create_payload.clone(),
        ))
        .await
        .unwrap();
    assert_eq!(admin_create.status(), StatusCode::CREATED);
    let product_id = body_json(admin_create).await["id"]
        .as_str()
        .unwrap()
        .to_string();

    let cashier_create = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/products",
            Some(&cashier_token),
            create_payload,
        ))
        .await
        .unwrap();
    assert_eq!(cashier_create.status(), StatusCode::FORBIDDEN);

    let update_payload = serde_json::json!({ "price": 20_000.0 });
    let admin_update = app
        .clone()
        .oneshot(json_request(
            "PATCH",
            &format!("/products/{product_id}"),
            Some(&admin_token),
            update_payload.clone(),
        ))
        .await
        .unwrap();
    assert_eq!(admin_update.status(), StatusCode::OK);

    let cashier_update = app
        .clone()
        .oneshot(json_request(
            "PATCH",
            &format!("/products/{product_id}"),
            Some(&cashier_token),
            update_payload,
        ))
        .await
        .unwrap();
    assert_eq!(cashier_update.status(), StatusCode::FORBIDDEN);

    let cashier_delete = app
        .clone()
        .oneshot(json_request(
            "DELETE",
            &format!("/products/{product_id}"),
            Some(&cashier_token),
            serde_json::json!({}),
        ))
        .await
        .unwrap();
    assert_eq!(cashier_delete.status(), StatusCode::FORBIDDEN);

    let admin_delete = app
        .oneshot(json_request(
            "DELETE",
            &format!("/products/{product_id}"),
            Some(&admin_token),
            serde_json::json!({}),
        ))
        .await
        .unwrap();
    assert_eq!(admin_delete.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn admin_can_cancel_order_but_cashier_cannot() {
    let app = test_app();
    let (owner_token, _tenant_id) =
        register(&app, "toko-budi", "owner@example.com").await;
    create_product(&app, &owner_token, "SKU-001", 15_000.0, 9_000.0, 10).await;
    let customer_id = create_customer(&app, &owner_token, "Pelanggan").await;
    let admin_token =
        invite_and_login(&app, &owner_token, "admin@example.com", "admin")
            .await;
    let cashier_token =
        invite_and_login(&app, &owner_token, "kasir@example.com", "cashier")
            .await;

    let order_payload = serde_json::json!({
        "customer_id": customer_id,
        "items": [{ "sku": "SKU-001", "quantity": 1 }]
    });
    let order_response = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/orders",
            Some(&owner_token),
            order_payload,
        ))
        .await
        .unwrap();
    let order_id = body_json(order_response).await["id"]
        .as_str()
        .unwrap()
        .to_string();

    let cashier_cancel = app
        .clone()
        .oneshot(json_request(
            "DELETE",
            &format!("/orders/{order_id}"),
            Some(&cashier_token),
            serde_json::json!({}),
        ))
        .await
        .unwrap();
    assert_eq!(cashier_cancel.status(), StatusCode::FORBIDDEN);

    let admin_cancel = app
        .oneshot(json_request(
            "DELETE",
            &format!("/orders/{order_id}"),
            Some(&admin_token),
            serde_json::json!({}),
        ))
        .await
        .unwrap();
    assert_eq!(admin_cancel.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn admin_can_view_audit_logs_but_cashier_cannot() {
    let app = test_app();
    let (owner_token, _tenant_id) =
        register(&app, "toko-budi", "owner@example.com").await;
    create_product(&app, &owner_token, "SKU-001", 15_000.0, 9_000.0, 10).await;
    let admin_token =
        invite_and_login(&app, &owner_token, "admin@example.com", "admin")
            .await;
    let cashier_token =
        invite_and_login(&app, &owner_token, "kasir@example.com", "cashier")
            .await;

    let cashier_response = app
        .clone()
        .oneshot(get_request("/tenants/me/audit-logs", Some(&cashier_token)))
        .await
        .unwrap();
    assert_eq!(cashier_response.status(), StatusCode::FORBIDDEN);

    let admin_response = app
        .oneshot(get_request("/tenants/me/audit-logs", Some(&admin_token)))
        .await
        .unwrap();
    assert_eq!(admin_response.status(), StatusCode::OK);
}

#[tokio::test]
async fn cashier_can_create_view_and_update_customers() {
    let app = test_app();
    let (owner_token, _tenant_id) =
        register(&app, "toko-budi", "owner@example.com").await;
    let cashier_token =
        invite_and_login(&app, &owner_token, "kasir@example.com", "cashier")
            .await;

    let create_payload = serde_json::json!({
        "name": "Pelanggan Satu", "phone": "081234567890"
    });
    let create_response = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/customers",
            Some(&cashier_token),
            create_payload,
        ))
        .await
        .unwrap();
    assert_eq!(create_response.status(), StatusCode::CREATED);
    let customer = body_json(create_response).await;
    let customer_id = customer["id"].as_str().unwrap().to_string();

    let list_response = app
        .clone()
        .oneshot(get_request("/customers", Some(&cashier_token)))
        .await
        .unwrap();
    assert_eq!(list_response.status(), StatusCode::OK);
    let list = body_json(list_response).await;
    assert_eq!(list.as_array().unwrap().len(), 1);

    let get_response = app
        .clone()
        .oneshot(get_request(
            &format!("/customers/{customer_id}"),
            Some(&cashier_token),
        ))
        .await
        .unwrap();
    assert_eq!(get_response.status(), StatusCode::OK);

    let update_payload = serde_json::json!({ "address": "Jl. Merdeka No. 1" });
    let update_response = app
        .oneshot(json_request(
            "PATCH",
            &format!("/customers/{customer_id}"),
            Some(&cashier_token),
            update_payload,
        ))
        .await
        .unwrap();
    assert_eq!(update_response.status(), StatusCode::OK);
    let updated = body_json(update_response).await;
    assert_eq!(updated["address"], "Jl. Merdeka No. 1");
}

#[tokio::test]
async fn cashier_cannot_delete_customer_but_admin_can() {
    let app = test_app();
    let (owner_token, _tenant_id) =
        register(&app, "toko-budi", "owner@example.com").await;
    let admin_token =
        invite_and_login(&app, &owner_token, "admin@example.com", "admin")
            .await;
    let cashier_token =
        invite_and_login(&app, &owner_token, "kasir@example.com", "cashier")
            .await;

    let create_payload = serde_json::json!({
        "name": "Pelanggan Dua", "phone": "081234500000"
    });
    let create_response = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/customers",
            Some(&owner_token),
            create_payload,
        ))
        .await
        .unwrap();
    let customer_id = body_json(create_response).await["id"]
        .as_str()
        .unwrap()
        .to_string();

    let cashier_delete = app
        .clone()
        .oneshot(json_request(
            "DELETE",
            &format!("/customers/{customer_id}"),
            Some(&cashier_token),
            serde_json::json!({}),
        ))
        .await
        .unwrap();
    assert_eq!(cashier_delete.status(), StatusCode::FORBIDDEN);

    let admin_delete = app
        .oneshot(json_request(
            "DELETE",
            &format!("/customers/{customer_id}"),
            Some(&admin_token),
            serde_json::json!({}),
        ))
        .await
        .unwrap();
    assert_eq!(admin_delete.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn cannot_create_customer_with_duplicate_phone_in_same_tenant() {
    let app = test_app();
    let (owner_token, _tenant_id) =
        register(&app, "toko-budi", "owner@example.com").await;

    let payload = serde_json::json!({
        "name": "Pelanggan Satu", "phone": "081234567890"
    });
    let first = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/customers",
            Some(&owner_token),
            payload.clone(),
        ))
        .await
        .unwrap();
    assert_eq!(first.status(), StatusCode::CREATED);

    let second = app
        .oneshot(json_request(
            "POST",
            "/customers",
            Some(&owner_token),
            payload,
        ))
        .await
        .unwrap();
    assert_eq!(second.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn customer_endpoints_require_authentication() {
    let app = test_app();

    let response = app.oneshot(get_request("/customers", None)).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn order_with_unknown_customer_id_returns_not_found() {
    let app = test_app();
    let (token, _tenant_id) =
        register(&app, "toko-budi", "budi@example.com").await;
    create_product(&app, &token, "SKU-001", 15_000.0, 9_000.0, 10).await;

    let payload = serde_json::json!({
        "customer_id": "cust-tidak-ada",
        "items": [{ "sku": "SKU-001", "quantity": 1 }]
    });
    let response = app
        .oneshot(json_request("POST", "/orders", Some(&token), payload))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn order_cannot_use_customer_from_another_tenant() {
    let app = test_app();
    let (token_a, _) = register(&app, "toko-a", "a@example.com").await;
    let (token_b, _) = register(&app, "toko-b", "b@example.com").await;
    create_product(&app, &token_a, "SKU-001", 15_000.0, 9_000.0, 10).await;
    let customer_id_b = create_customer(&app, &token_b, "Pelanggan B").await;

    let payload = serde_json::json!({
        "customer_id": customer_id_b,
        "items": [{ "sku": "SKU-001", "quantity": 1 }]
    });
    let response = app
        .oneshot(json_request("POST", "/orders", Some(&token_a), payload))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn update_product_can_change_cost_price_and_it_is_audited() {
    let app = test_app();
    let (token, _tenant_id) =
        register(&app, "toko-budi", "budi@example.com").await;
    let product_id =
        create_product(&app, &token, "SKU-001", 10_000.0, 6_000.0, 5).await;

    let response = app
        .clone()
        .oneshot(json_request(
            "PATCH",
            &format!("/products/{product_id}"),
            Some(&token),
            serde_json::json!({ "cost_price": 7_000.0 }),
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let updated = body_json(response).await;
    assert_eq!(updated["cost_price"], 7_000.0);
    // price wasn't sent in the payload -> must remain unchanged.
    assert_eq!(updated["price"], 10_000.0);

    let logs_response = app
        .oneshot(get_request("/tenants/me/audit-logs", Some(&token)))
        .await
        .unwrap();
    let logs = body_json(logs_response).await;
    let changes = logs[0]["changes"].as_array().unwrap();
    let cost_change = changes
        .iter()
        .find(|c| c["field"] == "cost_price")
        .expect("ada perubahan field cost_price");
    assert_eq!(cost_change["old_value"], "6000");
    assert_eq!(cost_change["new_value"], "7000");
}

#[tokio::test]
async fn order_snapshots_cost_price_so_later_changes_dont_affect_history() {
    let app = test_app();
    let (token, _tenant_id) =
        register(&app, "toko-budi", "budi@example.com").await;
    let product_id =
        create_product(&app, &token, "SKU-001", 20_000.0, 12_000.0, 10).await;
    let customer_id = create_customer(&app, &token, "Budi").await;

    let order_response = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/orders",
            Some(&token),
            serde_json::json!({
                "customer_id": customer_id,
                "items": [{ "sku": "SKU-001", "quantity": 2 }]
            }),
        ))
        .await
        .unwrap();
    assert_eq!(order_response.status(), StatusCode::CREATED);
    let order = body_json(order_response).await;
    assert_eq!(order["items"][0]["unit_cost"], 12_000.0);

    // Change the product's cost_price AFTER the order is created.
    app.clone()
        .oneshot(json_request(
            "PATCH",
            &format!("/products/{product_id}"),
            Some(&token),
            serde_json::json!({ "cost_price": 18_000.0 }),
        ))
        .await
        .unwrap();

    // The profit report must still use the OLD cost_price (12_000.0) that
    // was snapshotted when the order was created, not the new one (18_000.0).
    let report_response = app
        .oneshot(get_request("/tenants/me/reports/profit", Some(&token)))
        .await
        .unwrap();
    assert_eq!(report_response.status(), StatusCode::OK);
    let report = body_json(report_response).await;
    assert_eq!(report["total_revenue"], 40_000.0);
    assert_eq!(report["total_cost"], 24_000.0);
    assert_eq!(report["total_profit"], 16_000.0);
}

#[tokio::test]
async fn profit_report_computes_totals_and_per_product_breakdown() {
    let app = test_app();
    let (token, _tenant_id) =
        register(&app, "toko-budi", "budi@example.com").await;
    create_product(&app, &token, "SKU-A", 10_000.0, 6_000.0, 10).await;
    create_product(&app, &token, "SKU-B", 50_000.0, 20_000.0, 10).await;
    let customer_id = create_customer(&app, &token, "Budi").await;

    let response = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/orders",
            Some(&token),
            serde_json::json!({
                "customer_id": customer_id,
                "items": [
                    { "sku": "SKU-A", "quantity": 4 },
                    { "sku": "SKU-B", "quantity": 2 }
                ]
            }),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    // SKU-A: revenue 40_000, cost 24_000, profit 16_000.
    // SKU-B: revenue 100_000, cost 40_000, profit 60_000.
    let report_response = app
        .oneshot(get_request("/tenants/me/reports/profit", Some(&token)))
        .await
        .unwrap();
    assert_eq!(report_response.status(), StatusCode::OK);
    let report = body_json(report_response).await;

    assert_eq!(report["order_count"], 1);
    assert_eq!(report["total_revenue"], 140_000.0);
    assert_eq!(report["total_cost"], 64_000.0);
    assert_eq!(report["total_profit"], 76_000.0);

    let by_product = report["by_product"].as_array().unwrap();
    assert_eq!(by_product.len(), 2);
    // Sorted by largest profit -> SKU-B (60_000) first.
    assert_eq!(by_product[0]["sku"], "SKU-B");
    assert_eq!(by_product[0]["quantity_sold"], 2);
    assert_eq!(by_product[0]["revenue"], 100_000.0);
    assert_eq!(by_product[0]["cost"], 40_000.0);
    assert_eq!(by_product[0]["profit"], 60_000.0);
    assert_eq!(by_product[1]["sku"], "SKU-A");
    assert_eq!(by_product[1]["profit"], 16_000.0);
}

#[tokio::test]
async fn profit_report_is_owner_only() {
    let app = test_app();
    let (owner_token, _tenant_id) =
        register(&app, "toko-budi", "owner@example.com").await;
    create_product(&app, &owner_token, "SKU-001", 15_000.0, 9_000.0, 10).await;
    let admin_token =
        invite_and_login(&app, &owner_token, "admin@example.com", "admin")
            .await;
    let cashier_token =
        invite_and_login(&app, &owner_token, "kasir@example.com", "cashier")
            .await;

    // Admin CAN manage the product catalog & view the audit log, but the
    // profit report is intentionally stricter -> still 403 for admin.
    let admin_response = app
        .clone()
        .oneshot(get_request(
            "/tenants/me/reports/profit",
            Some(&admin_token),
        ))
        .await
        .unwrap();
    assert_eq!(admin_response.status(), StatusCode::FORBIDDEN);

    let cashier_response = app
        .clone()
        .oneshot(get_request(
            "/tenants/me/reports/profit",
            Some(&cashier_token),
        ))
        .await
        .unwrap();
    assert_eq!(cashier_response.status(), StatusCode::FORBIDDEN);

    let owner_response = app
        .oneshot(get_request(
            "/tenants/me/reports/profit",
            Some(&owner_token),
        ))
        .await
        .unwrap();
    assert_eq!(owner_response.status(), StatusCode::OK);
}

#[tokio::test]
async fn profit_report_can_be_filtered_by_date_range() {
    let app = test_app();
    let (token, _tenant_id) =
        register(&app, "toko-budi", "budi@example.com").await;
    create_product(&app, &token, "SKU-001", 15_000.0, 9_000.0, 10).await;
    let customer_id = create_customer(&app, &token, "Budi").await;

    let before = now_unix();
    let response = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/orders",
            Some(&token),
            serde_json::json!({
                "customer_id": customer_id,
                "items": [{ "sku": "SKU-001", "quantity": 1 }]
            }),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let after = now_unix();

    // A range that includes the order's creation time -> the order is counted.
    let in_range_response = app
        .clone()
        .oneshot(get_request(
            &format!("/tenants/me/reports/profit?from={before}&to={after}"),
            Some(&token),
        ))
        .await
        .unwrap();
    let in_range = body_json(in_range_response).await;
    assert_eq!(in_range["order_count"], 1);

    // A range entirely in the future -> the order is not counted.
    let future = after + 100_000;
    let out_of_range_response = app
        .oneshot(get_request(
            &format!("/tenants/me/reports/profit?from={future}"),
            Some(&token),
        ))
        .await
        .unwrap();
    let out_of_range = body_json(out_of_range_response).await;
    assert_eq!(out_of_range["order_count"], 0);
    assert_eq!(out_of_range["total_profit"], 0.0);
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}
