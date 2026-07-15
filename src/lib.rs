pub mod app;
pub mod config;
pub mod error;
pub mod pagination;
pub mod state;

pub mod audit;
pub mod customers;
pub mod orders;
pub mod products;
pub mod reports;
pub mod tenants;
pub mod users;

use axum::http::StatusCode;

pub async fn health_check() -> StatusCode {
    StatusCode::OK
}
