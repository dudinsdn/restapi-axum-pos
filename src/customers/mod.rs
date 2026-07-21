pub mod handler;
pub mod model;
pub mod repository;
pub mod service;
pub mod storage;

pub use model::{CreateCustomerRequest, Customer, UpdateCustomerRequest};
pub use repository::CustomerRepository;
pub use storage::postgres::PgCustomerRepository;
