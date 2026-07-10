pub mod handler;
pub mod model;
pub mod repository;
pub mod service;

pub use handler::{create_product, list_products};
pub use model::{CreateProductRequest, Product};
pub use repository::{InMemoryProductRepository, ProductRepository};
pub use service::ProductService;
