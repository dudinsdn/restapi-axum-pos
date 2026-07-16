#[allow(dead_code)]
mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;

use common::{
    body_json, create_product, get_request, invite_and_login, json_request,
    register, test_app,
};

#[tokio::test]
async fn products_endpoint_requires_auth() {
    let app = test_app();
    register(&app, "toko-budi", "budi@example.com").await;

    let response = app.oneshot(get_request("/products", None)).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn duplicate_sku_is_rejected() {
    let app = test_app();
    let (token, _tenant_id) =
        register(&app, "toko-budi", "budi@example.com").await;
    create_product(&app, &token, "SKU-001", 10_000, 6_000, 5).await;

    let payload = serde_json::json!({
        "name": "Nama Berbeda",
        "sku": "SKU-001",
        "price": 12_000,
        "cost_price": 7_000,
        "stock": 3
    });
    let response = app
        .oneshot(json_request("POST", "/products", Some(&token), payload))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn update_product_changes_fields() {
    let app = test_app();
    let (token, _tenant_id) =
        register(&app, "toko-budi", "budi@example.com").await;
    let product_id =
        create_product(&app, &token, "SKU-001", 10_000, 6_000, 5).await;

    let payload = serde_json::json!({ "price": 20_000, "stock": 50 });
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
    assert_eq!(updated["price"], 20_000);
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
        create_product(&app, &token_a, "SKU-001", 10_000, 6_000, 5).await;

    let payload = serde_json::json!({ "price": 1 });
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
        create_product(&app, &token, "SKU-001", 10_000, 6_000, 5).await;

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
async fn cost_price_is_hidden_from_cashier_but_visible_to_owner_and_admin() {
    let app = test_app();
    let (owner_token, _tenant_id) =
        register(&app, "toko-budi", "owner@example.com").await;
    create_product(&app, &owner_token, "SKU-001", 15_000, 9_000, 10).await;
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
    assert_eq!(owner_products[0]["cost_price"], 9_000);

    let admin_products = body_json(
        app.clone()
            .oneshot(get_request("/products", Some(&admin_token)))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(admin_products[0]["cost_price"], 9_000);

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
        "price": 5_000,
        "cost_price": 3_000,
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

    let update_payload = serde_json::json!({ "price": 20_000 });
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
async fn update_product_can_change_cost_price_and_it_is_audited() {
    let app = test_app();
    let (token, _tenant_id) =
        register(&app, "toko-budi", "budi@example.com").await;
    let product_id =
        create_product(&app, &token, "SKU-001", 10_000, 6_000, 5).await;

    let response = app
        .clone()
        .oneshot(json_request(
            "PATCH",
            &format!("/products/{product_id}"),
            Some(&token),
            serde_json::json!({ "cost_price": 7_000 }),
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let updated = body_json(response).await;
    assert_eq!(updated["cost_price"], 7_000);
    // price wasn't sent in the payload -> must remain unchanged.
    assert_eq!(updated["price"], 10_000);

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
async fn create_product_rejects_negative_price() {
    let app = test_app();
    let (token, _tenant_id) =
        register(&app, "toko-budi", "budi@example.com").await;

    let response = app
        .oneshot(json_request(
            "POST",
            "/products",
            Some(&token),
            serde_json::json!({
                "name": "Produk A",
                "sku": "SKU-001",
                "price": -1000,
                "cost_price": 500,
                "stock": 5
            }),
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn create_product_rejects_negative_stock() {
    let app = test_app();
    let (token, _tenant_id) =
        register(&app, "toko-budi", "budi@example.com").await;

    let response = app
        .oneshot(json_request(
            "POST",
            "/products",
            Some(&token),
            serde_json::json!({
                "name": "Produk A",
                "sku": "SKU-001",
                "price": 1000,
                "cost_price": 500,
                "stock": -1
            }),
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn create_product_rejects_empty_name_and_sku() {
    let app = test_app();
    let (token, _tenant_id) =
        register(&app, "toko-budi", "budi@example.com").await;

    let empty_name = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/products",
            Some(&token),
            serde_json::json!({
                "name": "   ",
                "sku": "SKU-001",
                "price": 1000,
                "cost_price": 500,
                "stock": 5
            }),
        ))
        .await
        .unwrap();
    assert_eq!(empty_name.status(), StatusCode::BAD_REQUEST);

    let empty_sku = app
        .oneshot(json_request(
            "POST",
            "/products",
            Some(&token),
            serde_json::json!({
                "name": "Produk A",
                "sku": "",
                "price": 1000,
                "cost_price": 500,
                "stock": 5
            }),
        ))
        .await
        .unwrap();
    assert_eq!(empty_sku.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn update_product_rejects_negative_cost_price() {
    let app = test_app();
    let (token, _tenant_id) =
        register(&app, "toko-budi", "budi@example.com").await;
    let product_id =
        create_product(&app, &token, "SKU-001", 10_000, 6_000, 5).await;

    let response = app
        .oneshot(json_request(
            "PATCH",
            &format!("/products/{product_id}"),
            Some(&token),
            serde_json::json!({ "cost_price": -100 }),
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn list_products_is_paginated_via_limit_and_offset() {
    let app = test_app();
    let (token, _tenant_id) =
        register(&app, "toko-budi", "budi@example.com").await;
    for i in 0..5 {
        create_product(&app, &token, &format!("SKU-{i:03}"), 10_000, 5_000, 10)
            .await;
    }

    let response = app
        .clone()
        .oneshot(get_request("/products?limit=2&offset=1", Some(&token)))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let total_count = response
        .headers()
        .get("x-total-count")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<usize>().ok());
    assert_eq!(total_count, Some(5));

    let page = body_json(response).await;
    let page = page.as_array().unwrap();
    assert_eq!(page.len(), 2);
    // Offset 1 skips SKU-000, so the page starts at SKU-001.
    assert_eq!(page[0]["sku"], "SKU-001");
    assert_eq!(page[1]["sku"], "SKU-002");

    // Default (no query params) still returns everything and the header.
    let default_response = app
        .oneshot(get_request("/products", Some(&token)))
        .await
        .unwrap();
    let default_total_count = default_response
        .headers()
        .get("x-total-count")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<usize>().ok());
    assert_eq!(default_total_count, Some(5));
    let default_page = body_json(default_response).await;
    assert_eq!(default_page.as_array().unwrap().len(), 5);
}

#[tokio::test]
async fn product_without_category_defaults_to_uncategorized() {
    let app = test_app();
    let (token, _tenant_id) =
        register(&app, "toko-budi", "budi@example.com").await;

    // The `create_product` test helper never sends `category` or
    // `low_stock_threshold`, so this also doubles as regression coverage
    // that every OTHER existing test creating a product still works
    // unchanged now that those fields exist.
    create_product(&app, &token, "SKU-001", 10_000, 6_000, 5).await;

    let response = app
        .oneshot(get_request("/products", Some(&token)))
        .await
        .unwrap();
    let products = body_json(response).await;
    assert_eq!(products[0]["category"], "Uncategorized");
    assert_eq!(products[0]["low_stock_threshold"], 5);
}

#[tokio::test]
async fn create_product_accepts_explicit_category_and_threshold() {
    let app = test_app();
    let (token, _tenant_id) =
        register(&app, "toko-budi", "budi@example.com").await;

    let response = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/products",
            Some(&token),
            serde_json::json!({
                "name": "Es Teh",
                "sku": "SKU-DRINK-001",
                "price": 5_000,
                "cost_price": 2_000,
                "stock": 20,
                "category": "Beverages",
                "low_stock_threshold": 10
            }),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let created = body_json(response).await;
    assert_eq!(created["category"], "Beverages");
    assert_eq!(created["low_stock_threshold"], 10);
}

#[tokio::test]
async fn create_product_rejects_blank_category_and_negative_threshold() {
    let app = test_app();
    let (token, _tenant_id) =
        register(&app, "toko-budi", "budi@example.com").await;

    let blank_category = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/products",
            Some(&token),
            serde_json::json!({
                "name": "Produk A",
                "sku": "SKU-001",
                "price": 1000,
                "cost_price": 500,
                "stock": 5,
                "category": "   "
            }),
        ))
        .await
        .unwrap();
    assert_eq!(blank_category.status(), StatusCode::BAD_REQUEST);

    let negative_threshold = app
        .oneshot(json_request(
            "POST",
            "/products",
            Some(&token),
            serde_json::json!({
                "name": "Produk A",
                "sku": "SKU-001",
                "price": 1000,
                "cost_price": 500,
                "stock": 5,
                "low_stock_threshold": -1
            }),
        ))
        .await
        .unwrap();
    assert_eq!(negative_threshold.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn update_product_can_change_category_and_it_is_audited() {
    let app = test_app();
    let (token, _tenant_id) =
        register(&app, "toko-budi", "budi@example.com").await;
    let product_id =
        create_product(&app, &token, "SKU-001", 10_000, 6_000, 5).await;

    let response = app
        .clone()
        .oneshot(json_request(
            "PATCH",
            &format!("/products/{product_id}"),
            Some(&token),
            serde_json::json!({ "category": "Snacks" }),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let updated = body_json(response).await;
    assert_eq!(updated["category"], "Snacks");

    let logs_response = app
        .oneshot(get_request("/tenants/me/audit-logs", Some(&token)))
        .await
        .unwrap();
    let logs = body_json(logs_response).await;
    let category_change = logs[0]["changes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|change| change["field"] == "category")
        .expect("expected a category change entry");
    assert_eq!(category_change["old_value"], "Uncategorized");
    assert_eq!(category_change["new_value"], "Snacks");
}

#[tokio::test]
async fn list_products_can_be_filtered_by_category() {
    let app = test_app();
    let (token, _tenant_id) =
        register(&app, "toko-budi", "budi@example.com").await;

    for (sku, category) in [
        ("SKU-A", "Beverages"),
        ("SKU-B", "Beverages"),
        ("SKU-C", "Snacks"),
    ] {
        app.clone()
            .oneshot(json_request(
                "POST",
                "/products",
                Some(&token),
                serde_json::json!({
                    "name": format!("Produk {sku}"),
                    "sku": sku,
                    "price": 5_000,
                    "cost_price": 2_000,
                    "stock": 10,
                    "category": category
                }),
            ))
            .await
            .unwrap();
    }

    // Filter is case-insensitive.
    let response = app
        .clone()
        .oneshot(get_request("/products?category=beverages", Some(&token)))
        .await
        .unwrap();
    let filtered = body_json(response).await;
    let filtered = filtered.as_array().unwrap();
    assert_eq!(filtered.len(), 2);
    assert!(
        filtered
            .iter()
            .all(|product| product["category"] == "Beverages")
    );

    let unfiltered = body_json(
        app.oneshot(get_request("/products", Some(&token)))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(unfiltered.as_array().unwrap().len(), 3);
}

#[tokio::test]
async fn low_stock_endpoint_returns_only_products_at_or_below_threshold() {
    let app = test_app();
    let (token, _tenant_id) =
        register(&app, "toko-budi", "budi@example.com").await;

    // Below its own threshold (2 <= 5) -> should show up.
    app.clone()
        .oneshot(json_request(
            "POST",
            "/products",
            Some(&token),
            serde_json::json!({
                "name": "Produk Low",
                "sku": "SKU-LOW",
                "price": 5_000,
                "cost_price": 2_000,
                "stock": 2,
                "low_stock_threshold": 5
            }),
        ))
        .await
        .unwrap();

    // Well above its own threshold -> should NOT show up.
    app.clone()
        .oneshot(json_request(
            "POST",
            "/products",
            Some(&token),
            serde_json::json!({
                "name": "Produk High",
                "sku": "SKU-HIGH",
                "price": 5_000,
                "cost_price": 2_000,
                "stock": 100,
                "low_stock_threshold": 5
            }),
        ))
        .await
        .unwrap();

    let response = app
        .oneshot(get_request("/products/low-stock", Some(&token)))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let low_stock = body_json(response).await;
    let low_stock = low_stock.as_array().unwrap();
    assert_eq!(low_stock.len(), 1);
    assert_eq!(low_stock[0]["sku"], "SKU-LOW");
}

#[tokio::test]
async fn low_stock_endpoint_is_visible_to_cashier_but_hides_cost_price() {
    let app = test_app();
    let (owner_token, _tenant_id) =
        register(&app, "toko-budi", "budi@example.com").await;
    app.clone()
        .oneshot(json_request(
            "POST",
            "/products",
            Some(&owner_token),
            serde_json::json!({
                "name": "Produk Low",
                "sku": "SKU-LOW",
                "price": 5_000,
                "cost_price": 2_000,
                "stock": 1,
                "low_stock_threshold": 5
            }),
        ))
        .await
        .unwrap();
    let cashier_token =
        invite_and_login(&app, &owner_token, "kasir@example.com", "cashier")
            .await;

    let response = app
        .oneshot(get_request("/products/low-stock", Some(&cashier_token)))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let low_stock = body_json(response).await;
    assert_eq!(low_stock[0]["sku"], "SKU-LOW");
    assert!(low_stock[0].get("cost_price").is_none());
}
