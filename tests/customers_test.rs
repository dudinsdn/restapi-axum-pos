#[allow(dead_code)]
mod common;

use axum::http::StatusCode;
use tower::ServiceExt;

use common::{
    body_json, get_request, invite_and_login, json_request, register, test_app,
};

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
async fn create_customer_rejects_empty_name_and_phone() {
    let app = test_app();
    let (token, _tenant_id) =
        register(&app, "toko-budi", "budi@example.com").await;

    let response = app
        .oneshot(json_request(
            "POST",
            "/customers",
            Some(&token),
            serde_json::json!({ "name": "  ", "phone": "" }),
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
