#[allow(dead_code)]
mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;

use common::{
    body_json, create_customer, create_product, get_request, invite_and_login,
    json_request, register, test_app,
};

#[sqlx::test]
async fn product_and_order_record_who_created_them(pool: sqlx::PgPool) {
    let app = test_app(pool);
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
                "price": 15_000,
                "cost_price": 9_000,
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

#[sqlx::test]
async fn audit_log_records_create_update_and_delete(pool: sqlx::PgPool) {
    let app = test_app(pool);
    let (token, _tenant_id) =
        register(&app, "toko-budi", "budi@example.com").await;
    let product_id =
        create_product(&app, &token, "SKU-001", 10_000, 6_000, 5).await;

    app.clone()
        .oneshot(json_request(
            "PATCH",
            &format!("/products/{product_id}"),
            Some(&token),
            serde_json::json!({ "price": 12_000 }),
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

#[sqlx::test]
async fn audit_logs_are_isolated_per_tenant(pool: sqlx::PgPool) {
    let app = test_app(pool);
    let (token_a, _) = register(&app, "toko-a", "a@example.com").await;
    let (token_b, _) = register(&app, "toko-b", "b@example.com").await;

    create_product(&app, &token_a, "SKU-001", 10_000, 6_000, 5).await;

    let logs_b = app
        .oneshot(get_request("/tenants/me/audit-logs", Some(&token_b)))
        .await
        .unwrap();
    let logs_b = body_json(logs_b).await;
    assert_eq!(logs_b.as_array().unwrap().len(), 0);
}

#[sqlx::test]
async fn audit_log_records_field_level_changes_on_update(pool: sqlx::PgPool) {
    let app = test_app(pool);
    let (token, _tenant_id) =
        register(&app, "toko-budi", "budi@example.com").await;
    let product_id =
        create_product(&app, &token, "SKU-001", 10_000, 6_000, 5).await;

    app.clone()
        .oneshot(json_request(
            "PATCH",
            &format!("/products/{product_id}"),
            Some(&token),
            serde_json::json!({ "price": 12_000, "stock": 20 }),
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

#[sqlx::test]
async fn noop_update_does_not_write_audit_entry(pool: sqlx::PgPool) {
    let app = test_app(pool);
    let (token, _tenant_id) =
        register(&app, "toko-budi", "budi@example.com").await;
    let product_id =
        create_product(&app, &token, "SKU-001", 10_000, 6_000, 5).await;

    // Send a price whose VALUE IS EXACTLY THE SAME as the current one.
    let response = app
        .clone()
        .oneshot(json_request(
            "PATCH",
            &format!("/products/{product_id}"),
            Some(&token),
            serde_json::json!({ "price": 10_000 }),
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

#[sqlx::test]
async fn audit_log_records_which_fields_changed(pool: sqlx::PgPool) {
    let app = test_app(pool);
    let (token, _tenant_id) =
        register(&app, "toko-budi", "budi@example.com").await;
    let product_id =
        create_product(&app, &token, "SKU-001", 10_000, 6_000, 5).await;

    // Only change price & stock, name isn't sent -> must not appear in
    // changes.
    app.clone()
        .oneshot(json_request(
            "PATCH",
            &format!("/products/{product_id}"),
            Some(&token),
            serde_json::json!({ "price": 12_000, "stock": 8 }),
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

#[sqlx::test]
async fn no_op_update_does_not_create_audit_entry(pool: sqlx::PgPool) {
    let app = test_app(pool);
    let (token, _tenant_id) =
        register(&app, "toko-budi", "budi@example.com").await;
    let product_id =
        create_product(&app, &token, "SKU-001", 10_000, 6_000, 5).await;

    // Send a value that's EXACTLY THE SAME as the current one -> no real
    // change, so no new audit entry should be added.
    app.clone()
        .oneshot(json_request(
            "PATCH",
            &format!("/products/{product_id}"),
            Some(&token),
            serde_json::json!({ "price": 10_000 }),
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

#[sqlx::test]
async fn admin_can_view_audit_logs_but_cashier_cannot(pool: sqlx::PgPool) {
    let app = test_app(pool);
    let (owner_token, _tenant_id) =
        register(&app, "toko-budi", "owner@example.com").await;
    create_product(&app, &owner_token, "SKU-001", 15_000, 9_000, 10).await;
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
