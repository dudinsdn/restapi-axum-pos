pub mod handler;
pub mod model;
pub mod postgres;
pub mod repository;
pub mod service;

pub use model::{CreateCustomerRequest, Customer, UpdateCustomerRequest};
pub use postgres::PgCustomerRepository;
pub use repository::{CustomerRepository, InMemoryCustomerRepository};
