#[allow(dead_code)]
mod common;

use axum::http::StatusCode;
use tower::ServiceExt;

use common::{get_request, test_app};

#[tokio::test]
async fn health_check_returns_ok() {
    let app = test_app();
    let response = app.oneshot(get_request("/health", None)).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}
