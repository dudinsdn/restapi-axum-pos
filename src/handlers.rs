use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use std::sync::Arc;

use crate::{
    error::{AppError, Result},
    models::{
        CreateOrderRequest, CreateProductRequest, CreateTenantRequest, Order,
        OrderItem, Product, Tenant,
    },
    state::AppState,
};

pub async fn health_check() -> StatusCode {
    StatusCode::OK
}

pub async fn list_tenants(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<Tenant>>> {
    Ok(Json(state.list_tenants()))
}

pub async fn create_tenant(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateTenantRequest>,
) -> Result<(StatusCode, Json<Tenant>)> {
    let tenant = Tenant {
        id: format!("tenant-{}", uuid::Uuid::new_v4().simple()),
        name: payload.name,
        slug: payload.slug,
        address: payload.address,
    };

    if !state.create_tenant(tenant.clone()) {
        return Err(AppError::Conflict("tenant already exists".into()));
    }

    Ok((StatusCode::CREATED, Json(tenant)))
}

pub async fn list_products(
    Path(tenant_id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<Product>>> {
    if state.get_tenant(&tenant_id).is_none() {
        return Err(AppError::NotFound("tenant not found".into()));
    }

    Ok(Json(state.list_products(&tenant_id)))
}

pub async fn create_product(
    Path(tenant_id): Path<String>,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateProductRequest>,
) -> Result<(StatusCode, Json<Product>)> {
    if state.get_tenant(&tenant_id).is_none() {
        return Err(AppError::NotFound("tenant not found".into()));
    }

    let product = Product {
        id: format!("prod-{}", uuid::Uuid::new_v4().simple()),
        tenant_id: tenant_id.clone(),
        name: payload.name,
        sku: payload.sku,
        price: payload.price,
        stock: payload.stock,
    };

    if !state.create_product(product.clone()) {
        return Err(AppError::Conflict("product already exists".into()));
    }

    Ok((StatusCode::CREATED, Json(product)))
}

pub async fn list_orders(
    Path(tenant_id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<Order>>> {
    if state.get_tenant(&tenant_id).is_none() {
        return Err(AppError::NotFound("tenant not found".into()));
    }

    Ok(Json(state.list_orders(&tenant_id)))
}

pub async fn create_order(
    Path(tenant_id): Path<String>,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateOrderRequest>,
) -> Result<(StatusCode, Json<Order>)> {
    if state.get_tenant(&tenant_id).is_none() {
        return Err(AppError::NotFound("tenant not found".into()));
    }

    let items: Vec<OrderItem> = payload
        .items
        .into_iter()
        .map(|item| OrderItem {
            sku: item.sku,
            name: item.name,
            quantity: item.quantity,
            unit_price: item.unit_price,
        })
        .collect();

    let total: f64 = items
        .iter()
        .map(|item| item.quantity as f64 * item.unit_price)
        .sum();

    let order = Order {
        id: format!("order-{}", uuid::Uuid::new_v4().simple()),
        tenant_id: tenant_id.clone(),
        customer_name: payload.customer_name,
        items,
        total,
    };

    if !state.create_order(order.clone()) {
        return Err(AppError::Conflict("order already exists".into()));
    }

    Ok((StatusCode::CREATED, Json(order)))
}

pub async fn tenant_not_found() -> Result<Json<serde_json::Value>> {
    Err(AppError::NotFound("tenant not found".into()))
}
