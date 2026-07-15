pub mod handler;
pub mod model;
pub mod postgres;
pub mod repository;
pub mod service;

pub use model::{CreateProductRequest, Product, ProductResponse};
pub use postgres::PgProductRepository;
pub use repository::{InMemoryProductRepository, ProductRepository};
