#[allow(dead_code)]
mod common;

use axum::http::StatusCode;
use tower::ServiceExt;

use common::{
    body_json, get_request, invite_and_login, json_request, register, test_app,
};

#[tokio::test]
async fn owner_can_create_list_update_and_delete_a_category() {
    let app = test_app();
    let (token, _tenant_id) =
        register(&app, "toko-budi", "budi@example.com").await;

    let create_response = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/categories",
            Some(&token),
            serde_json::json!({ "name": "Beverages" }),
        ))
        .await
        .unwrap();
    assert_eq!(create_response.status(), StatusCode::CREATED);
    let created = body_json(create_response).await;
    assert_eq!(created["name"], "Beverages");
    let category_id = created["id"].as_str().unwrap().to_string();

    let list_response = app
        .clone()
        .oneshot(get_request("/categories", Some(&token)))
        .await
        .unwrap();
    let categories = body_json(list_response).await;
    assert_eq!(categories.as_array().unwrap().len(), 1);

    let get_response = app
        .clone()
        .oneshot(get_request(
            &format!("/categories/{category_id}"),
            Some(&token),
        ))
        .await
        .unwrap();
    assert_eq!(get_response.status(), StatusCode::OK);
    let fetched = body_json(get_response).await;
    assert_eq!(fetched["name"], "Beverages");

    let update_response = app
        .clone()
        .oneshot(json_request(
            "PATCH",
            &format!("/categories/{category_id}"),
            Some(&token),
            serde_json::json!({ "name": "Drinks" }),
        ))
        .await
        .unwrap();
    assert_eq!(update_response.status(), StatusCode::OK);
    let updated = body_json(update_response).await;
    assert_eq!(updated["name"], "Drinks");

    let logs_response = app
        .clone()
        .oneshot(get_request("/tenants/me/audit-logs", Some(&token)))
        .await
        .unwrap();
    let logs = body_json(logs_response).await;
    assert_eq!(logs[0]["resource_type"], "category");
    let name_change = logs[0]["changes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|change| change["field"] == "name")
        .expect("expected a name change entry");
    assert_eq!(name_change["old_value"], "Beverages");
    assert_eq!(name_change["new_value"], "Drinks");

    let delete_response = app
        .clone()
        .oneshot(json_request(
            "DELETE",
            &format!("/categories/{category_id}"),
            Some(&token),
            serde_json::json!({}),
        ))
        .await
        .unwrap();
    assert_eq!(delete_response.status(), StatusCode::NO_CONTENT);

    let list_after_delete = body_json(
        app.oneshot(get_request("/categories", Some(&token)))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(list_after_delete.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn category_name_must_be_unique_per_tenant_case_insensitively() {
    let app = test_app();
    let (token, _tenant_id) =
        register(&app, "toko-budi", "budi@example.com").await;

    app.clone()
        .oneshot(json_request(
            "POST",
            "/categories",
            Some(&token),
            serde_json::json!({ "name": "Snacks" }),
        ))
        .await
        .unwrap();

    let duplicate = app
        .oneshot(json_request(
            "POST",
            "/categories",
            Some(&token),
            serde_json::json!({ "name": "snacks" }),
        ))
        .await
        .unwrap();
    assert_eq!(duplicate.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn create_category_rejects_blank_name() {
    let app = test_app();
    let (token, _tenant_id) =
        register(&app, "toko-budi", "budi@example.com").await;

    let response = app
        .oneshot(json_request(
            "POST",
            "/categories",
            Some(&token),
            serde_json::json!({ "name": "   " }),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn cashier_can_view_but_not_manage_categories() {
    let app = test_app();
    let (owner_token, _tenant_id) =
        register(&app, "toko-budi", "budi@example.com").await;
    let create_response = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/categories",
            Some(&owner_token),
            serde_json::json!({ "name": "Beverages" }),
        ))
        .await
        .unwrap();
    let category_id = body_json(create_response).await["id"]
        .as_str()
        .unwrap()
        .to_string();
    let cashier_token =
        invite_and_login(&app, &owner_token, "kasir@example.com", "cashier")
            .await;

    // Cashier CAN list and view categories.
    let list_response = app
        .clone()
        .oneshot(get_request("/categories", Some(&cashier_token)))
        .await
        .unwrap();
    assert_eq!(list_response.status(), StatusCode::OK);
    assert_eq!(body_json(list_response).await.as_array().unwrap().len(), 1);

    let get_response = app
        .clone()
        .oneshot(get_request(
            &format!("/categories/{category_id}"),
            Some(&cashier_token),
        ))
        .await
        .unwrap();
    assert_eq!(get_response.status(), StatusCode::OK);

    // Cashier CANNOT create, update, or delete categories.
    let create_attempt = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/categories",
            Some(&cashier_token),
            serde_json::json!({ "name": "Snacks" }),
        ))
        .await
        .unwrap();
    assert_eq!(create_attempt.status(), StatusCode::FORBIDDEN);

    let update_attempt = app
        .clone()
        .oneshot(json_request(
            "PATCH",
            &format!("/categories/{category_id}"),
            Some(&cashier_token),
            serde_json::json!({ "name": "Drinks" }),
        ))
        .await
        .unwrap();
    assert_eq!(update_attempt.status(), StatusCode::FORBIDDEN);

    let delete_attempt = app
        .oneshot(json_request(
            "DELETE",
            &format!("/categories/{category_id}"),
            Some(&cashier_token),
            serde_json::json!({}),
        ))
        .await
        .unwrap();
    assert_eq!(delete_attempt.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn category_products_endpoint_looks_up_products_by_category() {
    let app = test_app();
    let (token, _tenant_id) =
        register(&app, "toko-budi", "budi@example.com").await;

    let create_response = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/categories",
            Some(&token),
            serde_json::json!({ "name": "Beverages" }),
        ))
        .await
        .unwrap();
    let category_id = body_json(create_response).await["id"]
        .as_str()
        .unwrap()
        .to_string();

    // SKU-A is tagged with the "Beverages" category via `category_id`;
    // SKU-B is left uncategorized, so it must NOT show up in the lookup.
    for (sku, category_id_value) in
        [("SKU-A", Some(category_id.clone())), ("SKU-B", None)]
    {
        let mut payload = serde_json::json!({
            "name": format!("Produk {sku}"),
            "sku": sku,
            "price": 5_000,
            "cost_price": 2_000,
            "stock": 10
        });
        if let Some(id) = category_id_value {
            payload["category_id"] = serde_json::json!(id);
        }
        app.clone()
            .oneshot(json_request("POST", "/products", Some(&token), payload))
            .await
            .unwrap();
    }

    let response = app
        .oneshot(get_request(
            &format!("/categories/{category_id}/products"),
            Some(&token),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let products = body_json(response).await;
    let products = products.as_array().unwrap();
    assert_eq!(products.len(), 1);
    assert_eq!(products[0]["sku"], "SKU-A");
}

#[tokio::test]
async fn category_products_endpoint_hides_cost_price_from_cashier() {
    let app = test_app();
    let (owner_token, _tenant_id) =
        register(&app, "toko-budi", "budi@example.com").await;
    let create_response = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/categories",
            Some(&owner_token),
            serde_json::json!({ "name": "Beverages" }),
        ))
        .await
        .unwrap();
    let category_id = body_json(create_response).await["id"]
        .as_str()
        .unwrap()
        .to_string();
    app.clone()
        .oneshot(json_request(
            "POST",
            "/products",
            Some(&owner_token),
            serde_json::json!({
                "name": "Es Teh",
                "sku": "SKU-DRINK",
                "price": 5_000,
                "cost_price": 2_000,
                "stock": 10,
                "category_id": category_id
            }),
        ))
        .await
        .unwrap();
    let cashier_token =
        invite_and_login(&app, &owner_token, "kasir@example.com", "cashier")
            .await;

    let response = app
        .oneshot(get_request(
            &format!("/categories/{category_id}/products"),
            Some(&cashier_token),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let products = body_json(response).await;
    assert_eq!(products[0]["sku"], "SKU-DRINK");
    assert!(products[0].get("cost_price").is_none());
}

#[tokio::test]
async fn deleting_a_category_clears_category_id_but_keeps_the_display_name() {
    let app = test_app();
    let (token, _tenant_id) =
        register(&app, "toko-budi", "budi@example.com").await;

    let category_response = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/categories",
            Some(&token),
            serde_json::json!({ "name": "Beverages" }),
        ))
        .await
        .unwrap();
    let category_id = body_json(category_response).await["id"]
        .as_str()
        .unwrap()
        .to_string();

    let product_response = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/products",
            Some(&token),
            serde_json::json!({
                "name": "Es Teh",
                "sku": "SKU-DRINK",
                "price": 5_000,
                "cost_price": 2_000,
                "stock": 10,
                "category_id": category_id
            }),
        ))
        .await
        .unwrap();
    let product_id = body_json(product_response).await["id"]
        .as_str()
        .unwrap()
        .to_string();

    let delete_response = app
        .clone()
        .oneshot(json_request(
            "DELETE",
            &format!("/categories/{category_id}"),
            Some(&token),
            serde_json::json!({}),
        ))
        .await
        .unwrap();
    assert_eq!(delete_response.status(), StatusCode::NO_CONTENT);

    // The category is gone, but the product's denormalized display name
    // survives — same trade-off as an order's `customer_name` surviving a
    // deleted customer. Only `category_id` gets cleared.
    let products_response = app
        .oneshot(get_request("/products", Some(&token)))
        .await
        .unwrap();
    let products = body_json(products_response).await;
    let product = products
        .as_array()
        .unwrap()
        .iter()
        .find(|product| product["id"] == product_id)
        .expect("product should still exist");
    assert!(product["category_id"].is_null());
    assert_eq!(product["category"], "Beverages");
}
