#[allow(dead_code)]
mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;

use common::{
    body_json, create_customer, create_product, get_request, invite_and_login,
    json_request, json_request_with_header, register, test_app,
};

#[sqlx::test]
async fn order_uses_real_product_price_and_reduces_stock(pool: sqlx::PgPool) {
    let app = test_app(pool);
    let (token, _tenant_id) =
        register(&app, "toko-budi", "budi@example.com").await;
    create_product(&app, &token, "SKU-001", 15_000, 9_000, 10).await;
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
    assert_eq!(order["total"], 45_000);
    assert_eq!(order["items"][0]["unit_price"], 15_000);

    let products_response = app
        .oneshot(get_request("/products", Some(&token)))
        .await
        .unwrap();
    let products = body_json(products_response).await;
    assert_eq!(products[0]["stock"], 7);
}

#[sqlx::test]
async fn order_with_unknown_sku_returns_not_found(pool: sqlx::PgPool) {
    let app = test_app(pool);
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

#[sqlx::test]
async fn order_fails_when_stock_insufficient(pool: sqlx::PgPool) {
    let app = test_app(pool);
    let (token, _tenant_id) =
        register(&app, "toko-budi", "budi@example.com").await;
    create_product(&app, &token, "SKU-001", 15_000, 9_000, 2).await;
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

#[sqlx::test]
async fn cancel_order_restores_stock_and_removes_order(pool: sqlx::PgPool) {
    let app = test_app(pool);
    let (token, _tenant_id) =
        register(&app, "toko-budi", "budi@example.com").await;
    create_product(&app, &token, "SKU-001", 15_000, 9_000, 10).await;
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

#[sqlx::test]
async fn order_unit_cost_is_hidden_from_cashier_but_visible_to_owner_and_admin(
    pool: sqlx::PgPool,
) {
    let app = test_app(pool);
    let (owner_token, _tenant_id) =
        register(&app, "toko-budi", "owner@example.com").await;
    create_product(&app, &owner_token, "SKU-001", 15_000, 9_000, 10).await;
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
    assert_eq!(owner_order["items"][0]["unit_cost"], 9_000);

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

#[sqlx::test]
async fn admin_can_cancel_order_but_cashier_cannot(pool: sqlx::PgPool) {
    let app = test_app(pool);
    let (owner_token, _tenant_id) =
        register(&app, "toko-budi", "owner@example.com").await;
    create_product(&app, &owner_token, "SKU-001", 15_000, 9_000, 10).await;
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

#[sqlx::test]
async fn cashier_can_view_products_and_create_orders(pool: sqlx::PgPool) {
    let app = test_app(pool);
    let (owner_token, _tenant_id) =
        register(&app, "toko-budi", "owner@example.com").await;
    create_product(&app, &owner_token, "SKU-001", 15_000, 9_000, 10).await;
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

#[sqlx::test]
async fn order_with_unknown_customer_id_returns_not_found(pool: sqlx::PgPool) {
    let app = test_app(pool);
    let (token, _tenant_id) =
        register(&app, "toko-budi", "budi@example.com").await;
    create_product(&app, &token, "SKU-001", 15_000, 9_000, 10).await;

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

#[sqlx::test]
async fn order_cannot_use_customer_from_another_tenant(pool: sqlx::PgPool) {
    let app = test_app(pool);
    let (token_a, _) = register(&app, "toko-a", "a@example.com").await;
    let (token_b, _) = register(&app, "toko-b", "b@example.com").await;
    create_product(&app, &token_a, "SKU-001", 15_000, 9_000, 10).await;
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

#[sqlx::test]
async fn order_snapshots_cost_price_so_later_changes_dont_affect_history(pool: sqlx::PgPool) {
    let app = test_app(pool);
    let (token, _tenant_id) =
        register(&app, "toko-budi", "budi@example.com").await;
    let product_id =
        create_product(&app, &token, "SKU-001", 20_000, 12_000, 10).await;
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
    assert_eq!(order["items"][0]["unit_cost"], 12_000);

    // Change the product's cost_price AFTER the order is created.
    app.clone()
        .oneshot(json_request(
            "PATCH",
            &format!("/products/{product_id}"),
            Some(&token),
            serde_json::json!({ "cost_price": 18_000 }),
        ))
        .await
        .unwrap();

    // The profit report must still use the OLD cost_price (12_000) that
    // was snapshotted when the order was created, not the new one (18_000).
    let report_response = app
        .oneshot(get_request("/tenants/me/reports/profit", Some(&token)))
        .await
        .unwrap();
    assert_eq!(report_response.status(), StatusCode::OK);
    let report = body_json(report_response).await;
    assert_eq!(report["total_revenue"], 40_000);
    assert_eq!(report["total_cost"], 24_000);
    assert_eq!(report["total_profit"], 16_000);
}

#[sqlx::test]
async fn create_order_rejects_zero_or_negative_quantity(pool: sqlx::PgPool) {
    let app = test_app(pool);
    let (token, _tenant_id) =
        register(&app, "toko-budi", "budi@example.com").await;
    create_product(&app, &token, "SKU-001", 10_000, 6_000, 5).await;
    let customer_id = create_customer(&app, &token, "Budi").await;

    let response = app
        .oneshot(json_request(
            "POST",
            "/orders",
            Some(&token),
            serde_json::json!({
                "customer_id": customer_id,
                "items": [{ "sku": "SKU-001", "quantity": 0 }]
            }),
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn create_order_with_same_idempotency_key_returns_same_order(pool: sqlx::PgPool) {
    let app = test_app(pool);
    let (token, _tenant_id) =
        register(&app, "toko-budi", "budi@example.com").await;
    create_product(&app, &token, "SKU-001", 10_000, 6_000, 5).await;
    let customer_id = create_customer(&app, &token, "Budi").await;

    let payload = serde_json::json!({
        "customer_id": customer_id,
        "items": [{ "sku": "SKU-001", "quantity": 2 }]
    });

    let first_response = app
        .clone()
        .oneshot(json_request_with_header(
            "POST",
            "/orders",
            Some(&token),
            ("Idempotency-Key", "order-key-1"),
            payload.clone(),
        ))
        .await
        .unwrap();
    assert_eq!(first_response.status(), StatusCode::CREATED);
    let first_order = body_json(first_response).await;

    // Retry with the SAME key (e.g. a client retry after a timeout) must
    // return the SAME order, not create a second one.
    let second_response = app
        .clone()
        .oneshot(json_request_with_header(
            "POST",
            "/orders",
            Some(&token),
            ("Idempotency-Key", "order-key-1"),
            payload,
        ))
        .await
        .unwrap();
    assert_eq!(second_response.status(), StatusCode::CREATED);
    let second_order = body_json(second_response).await;
    assert_eq!(first_order["id"], second_order["id"]);

    // Only ONE order should actually exist, and stock should only have been
    // reserved once (5 - 2 = 3, not 5 - 4 = 1).
    let orders_response = app
        .clone()
        .oneshot(get_request("/orders", Some(&token)))
        .await
        .unwrap();
    let orders = body_json(orders_response).await;
    assert_eq!(orders.as_array().unwrap().len(), 1);

    let products_response = app
        .oneshot(get_request("/products", Some(&token)))
        .await
        .unwrap();
    let products = body_json(products_response).await;
    assert_eq!(products[0]["stock"], 3);
}

#[sqlx::test]
async fn create_order_without_idempotency_key_creates_separate_orders(pool: sqlx::PgPool) {
    let app = test_app(pool);
    let (token, _tenant_id) =
        register(&app, "toko-budi", "budi@example.com").await;
    create_product(&app, &token, "SKU-001", 10_000, 6_000, 5).await;
    let customer_id = create_customer(&app, &token, "Budi").await;

    let payload = serde_json::json!({
        "customer_id": customer_id,
        "items": [{ "sku": "SKU-001", "quantity": 1 }]
    });

    for _ in 0..2 {
        let response = app
            .clone()
            .oneshot(json_request(
                "POST",
                "/orders",
                Some(&token),
                payload.clone(),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
    }

    let orders_response = app
        .oneshot(get_request("/orders", Some(&token)))
        .await
        .unwrap();
    let orders = body_json(orders_response).await;
    // No idempotency key was sent -> each request is a genuinely new order.
    assert_eq!(orders.as_array().unwrap().len(), 2);
}
