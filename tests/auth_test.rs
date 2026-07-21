#[allow(dead_code)]
mod common;

use axum::http::StatusCode;
use serde_json::Value;
use tower::ServiceExt;

use common::{
    body_json, create_product, get_request, json_request, register, test_app,
};

#[sqlx::test]
async fn register_creates_tenant_and_owner_with_token(pool: sqlx::PgPool) {
    let app = test_app(pool);
    let (token, _tenant_id) =
        register(&app, "toko-budi", "budi@example.com").await;
    assert!(!token.is_empty());
}

#[sqlx::test]
async fn duplicate_slug_on_register_is_rejected(pool: sqlx::PgPool) {
    let app = test_app(pool);
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

#[sqlx::test]
async fn duplicate_email_on_register_is_rejected(pool: sqlx::PgPool) {
    let app = test_app(pool);
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

#[sqlx::test]
async fn login_with_correct_credentials_returns_token(pool: sqlx::PgPool) {
    let app = test_app(pool);
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

#[sqlx::test]
async fn login_with_wrong_password_is_unauthorized(pool: sqlx::PgPool) {
    let app = test_app(pool);
    register(&app, "toko-budi", "budi@example.com").await;

    let payload = serde_json::json!({ "email": "budi@example.com", "password": "salah-password" });
    let response = app
        .oneshot(json_request("POST", "/auth/login", None, payload))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn register_persists_tenant_address(pool: sqlx::PgPool) {
    let app = test_app(pool);

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

#[sqlx::test]
async fn register_without_address_leaves_it_null(pool: sqlx::PgPool) {
    let app = test_app(pool);
    let (token, _tenant_id) =
        register(&app, "toko-tanpa-alamat", "notaddr@example.com").await;

    let me_response = app
        .oneshot(get_request("/tenants/me", Some(&token)))
        .await
        .unwrap();
    let tenant = body_json(me_response).await;
    assert_eq!(tenant["address"], Value::Null);
}

#[sqlx::test]
async fn login_is_rate_limited_after_too_many_failures(pool: sqlx::PgPool) {
    let app = test_app(pool);
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

#[sqlx::test]
async fn logout_revokes_the_token(pool: sqlx::PgPool) {
    let app = test_app(pool);
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

#[sqlx::test]
async fn tenant_data_is_isolated_by_token_not_by_request(pool: sqlx::PgPool) {
    let app = test_app(pool);
    let (token_a, _tenant_a) = register(&app, "toko-a", "a@example.com").await;
    let (token_b, _tenant_b) = register(&app, "toko-b", "b@example.com").await;

    create_product(&app, &token_a, "SKU-A", 10_000, 6_000, 5).await;

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
