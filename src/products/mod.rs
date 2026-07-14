pub mod handler;
pub mod model;
pub mod repository;
pub mod service;

pub use model::{CreateProductRequest, Product, ProductResponse};
pub use repository::{InMemoryProductRepository, ProductRepository};
