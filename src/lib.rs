pub mod app;
pub mod config;
pub mod error;
pub mod state;

pub mod audit;
pub mod orders;
pub mod products;
pub mod tenants;
pub mod users;

use axum::http::StatusCode;

pub async fn health_check() -> StatusCode {
    StatusCode::OK
}
