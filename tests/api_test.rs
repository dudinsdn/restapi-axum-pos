use axum::http::{Request, StatusCode};
use serde_json::json;
use tower::ServiceExt;

#[tokio::test]
async fn test_health_check() {
    let app = restapi_axum_pos::app::create_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .method("GET")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_create_and_list_tenants() {
    let app = restapi_axum_pos::app::create_app();

    // Create tenant
    let create_tenant_body = json!({
        "name": "Acme Corp",
        "slug": "acme-corp",
        "address": "123 Main St"
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/tenants")
                .method("POST")
                .header("content-type", "application/json")
                .body(axum::body::Body::from(
                    serde_json::to_string(&create_tenant_body).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    // List tenants
    let response = app
        .oneshot(
            Request::builder()
                .uri("/tenants")
                .method("GET")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_create_product_validates_price() {
    let app = restapi_axum_pos::app::create_app();

    // First create a tenant
    let create_tenant_body = json!({
        "name": "Test Store",
        "slug": "test-store"
    });

    let tenant_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/tenants")
                .method("POST")
                .header("content-type", "application/json")
                .body(axum::body::Body::from(
                    serde_json::to_string(&create_tenant_body).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    // Extract tenant ID from response
    let body = axum::body::to_bytes(tenant_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let tenant_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let tenant_id = tenant_json["id"].as_str().unwrap();

    // Try to create product with negative price (should fail with business logic validation)
    let invalid_product = json!({
        "name": "Test Product",
        "sku": "TEST-001",
        "price": -10.0,
        "stock": 100
    });

    let response = app
        .oneshot(
            Request::builder()
                .uri(&format!("/tenants/{}/products", tenant_id))
                .method("POST")
                .header("content-type", "application/json")
                .body(axum::body::Body::from(
                    serde_json::to_string(&invalid_product).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_create_order_calculates_total() {
    let app = restapi_axum_pos::app::create_app();

    // Create tenant
    let tenant_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/tenants")
                .method("POST")
                .header("content-type", "application/json")
                .body(axum::body::Body::from(
                    serde_json::to_string(&json!({
                        "name": "Store",
                        "slug": "store"
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(tenant_resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let tenant_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let tenant_id = tenant_json["id"].as_str().unwrap();

    // Create order
    let order_body = json!({
        "customer_name": "John Doe",
        "items": [
            {
                "sku": "ITEM-1",
                "name": "Widget",
                "quantity": 2,
                "unit_price": 10.0
            },
            {
                "sku": "ITEM-2",
                "name": "Gadget",
                "quantity": 1,
                "unit_price": 5.0
            }
        ]
    });

    let response = app
        .oneshot(
            Request::builder()
                .uri(&format!("/tenants/{}/orders", tenant_id))
                .method("POST")
                .header("content-type", "application/json")
                .body(axum::body::Body::from(
                    serde_json::to_string(&order_body).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let order_json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    // Verify total is calculated correctly: (2 * 10.0) + (1 * 5.0) = 25.0
    assert_eq!(order_json["total"], 25.0);
}
