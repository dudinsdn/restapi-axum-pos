#[allow(dead_code)]
mod common;

use axum::http::StatusCode;
use tower::ServiceExt;

use common::{
    body_json, create_customer, create_product, get_request, invite_and_login,
    json_request, now_unix, register, test_app,
};

#[sqlx::test]
async fn profit_report_computes_totals_and_per_product_breakdown(pool: sqlx::PgPool) {
    let app = test_app(pool);
    let (token, _tenant_id) =
        register(&app, "toko-budi", "budi@example.com").await;
    create_product(&app, &token, "SKU-A", 10_000, 6_000, 10).await;
    create_product(&app, &token, "SKU-B", 50_000, 20_000, 10).await;
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
    assert_eq!(report["total_revenue"], 140_000);
    assert_eq!(report["total_cost"], 64_000);
    assert_eq!(report["total_profit"], 76_000);

    let by_product = report["by_product"].as_array().unwrap();
    assert_eq!(by_product.len(), 2);
    // Sorted by largest profit -> SKU-B (60_000) first.
    assert_eq!(by_product[0]["sku"], "SKU-B");
    assert_eq!(by_product[0]["quantity_sold"], 2);
    assert_eq!(by_product[0]["revenue"], 100_000);
    assert_eq!(by_product[0]["cost"], 40_000);
    assert_eq!(by_product[0]["profit"], 60_000);
    assert_eq!(by_product[1]["sku"], "SKU-A");
    assert_eq!(by_product[1]["profit"], 16_000);
}

#[sqlx::test]
async fn profit_report_is_owner_only(pool: sqlx::PgPool) {
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

#[sqlx::test]
async fn profit_report_can_be_filtered_by_date_range(pool: sqlx::PgPool) {
    let app = test_app(pool);
    let (token, _tenant_id) =
        register(&app, "toko-budi", "budi@example.com").await;
    create_product(&app, &token, "SKU-001", 15_000, 9_000, 10).await;
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
    assert_eq!(out_of_range["total_profit"], 0);
}
