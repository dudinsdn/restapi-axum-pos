#[allow(dead_code)]
mod common;

use axum::http::StatusCode;
use tower::ServiceExt;

use common::{get_request, test_app};

#[sqlx::test]
async fn health_check_returns_ok(pool: sqlx::PgPool) {
    let app = test_app(pool);
    let response = app.oneshot(get_request("/health", None)).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}
