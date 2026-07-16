pub mod handler;
pub mod model;
pub mod repository;
pub mod service;
pub mod storage;

pub use model::{CreateProductRequest, Product, ProductResponse};
pub use repository::ProductRepository;
pub use storage::{
    inmemory::InMemoryProductRepository, postgres::PgProductRepository,
};
