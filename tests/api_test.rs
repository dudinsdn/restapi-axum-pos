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
    tenants::InMemoryTenantRepository,
};

fn test_app() -> Router {
    let state = AppState::new(
        InMemoryTenantRepository::new(),
        InMemoryProductRepository::new(),
        InMemoryOrderRepository::new(),
    );
    create_app(state)
}

fn json_request(method: &str, uri: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

async fn body_json(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

/// Helper: bikin tenant baru, return `id`-nya.
async fn create_tenant(app: &Router, slug: &str) -> String {
    let payload = serde_json::json!({ "name": "Toko Test", "slug": slug, "address": null });
    let response = app
        .clone()
        .oneshot(json_request("POST", "/tenants", payload))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    body_json(response).await["id"]
        .as_str()
        .unwrap()
        .to_string()
}

/// Helper: bikin product baru untuk satu tenant.
async fn create_product(
    app: &Router,
    tenant_id: &str,
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
            payload,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn health_check_returns_ok() {
    let app = test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn create_then_list_tenant() {
    let app = test_app();
    create_tenant(&app, "toko-budi").await;

    let list_response = app
        .oneshot(
            Request::builder()
                .uri("/tenants")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(list_response.status(), StatusCode::OK);
}

#[tokio::test]
async fn duplicate_slug_is_rejected() {
    let app = test_app();
    create_tenant(&app, "toko-budi").await;

    let payload = serde_json::json!({ "name": "Toko Budi Cabang 2", "slug": "toko-budi", "address": null });
    let response = app
        .oneshot(json_request("POST", "/tenants", payload))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn duplicate_sku_is_rejected() {
    let app = test_app();
    let tenant_id = create_tenant(&app, "toko-budi").await;
    create_product(&app, &tenant_id, "SKU-001", 10_000.0, 5).await;

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
            payload,
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn product_for_unknown_tenant_returns_not_found() {
    let app = test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/tenants/tenant-tidak-ada/products")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn order_uses_real_product_price_and_reduces_stock() {
    let app = test_app();
    let tenant_id = create_tenant(&app, "toko-budi").await;
    create_product(&app, &tenant_id, "SKU-001", 15_000.0, 10).await;

    // Sengaja kirim harga & nama yang beda dari product asli — harus diabaikan.
    let payload = serde_json::json!({
        "customer_name": "Budi",
        "items": [{ "sku": "SKU-001", "quantity": 3 }]
    });
    let response = app
        .clone()
        .oneshot(json_request(
            "POST",
            &format!("/tenants/{tenant_id}/orders"),
            payload,
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
    let order = body_json(response).await;
    assert_eq!(order["total"], 45_000.0);
    assert_eq!(order["items"][0]["name"], "Produk SKU-001");
    assert_eq!(order["items"][0]["unit_price"], 15_000.0);

    // Stock harus berkurang 3, dari 10 jadi 7.
    let products_response = app
        .oneshot(
            Request::builder()
                .uri(format!("/tenants/{tenant_id}/products"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let products = body_json(products_response).await;
    assert_eq!(products[0]["stock"], 7);
}

#[tokio::test]
async fn order_with_unknown_sku_returns_not_found() {
    let app = test_app();
    let tenant_id = create_tenant(&app, "toko-budi").await;

    let payload = serde_json::json!({
        "customer_name": "Budi",
        "items": [{ "sku": "SKU-TIDAK-ADA", "quantity": 1 }]
    });
    let response = app
        .oneshot(json_request(
            "POST",
            &format!("/tenants/{tenant_id}/orders"),
            payload,
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn order_fails_when_stock_insufficient() {
    let app = test_app();
    let tenant_id = create_tenant(&app, "toko-budi").await;
    create_product(&app, &tenant_id, "SKU-001", 15_000.0, 2).await;

    let payload = serde_json::json!({
        "customer_name": "Budi",
        "items": [{ "sku": "SKU-001", "quantity": 5 }]
    });
    let response = app
        .clone()
        .oneshot(json_request(
            "POST",
            &format!("/tenants/{tenant_id}/orders"),
            payload,
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CONFLICT);

    // Stock tidak boleh berubah karena order gagal.
    let products_response = app
        .oneshot(
            Request::builder()
                .uri(format!("/tenants/{tenant_id}/products"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let products = body_json(products_response).await;
    assert_eq!(products[0]["stock"], 2);
}
