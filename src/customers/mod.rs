pub mod handler;
pub mod model;
pub mod repository;
pub mod service;

pub use model::{CreateCustomerRequest, Customer, UpdateCustomerRequest};
pub use repository::{CustomerRepository, InMemoryCustomerRepository};
