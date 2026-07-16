#[allow(dead_code)]
mod common;

use axum::http::StatusCode;
use tower::ServiceExt;

use common::{body_json, invite_and_login, json_request, register, test_app};

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
