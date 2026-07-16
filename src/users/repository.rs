use std::future::Future;

use super::model::User;

pub trait UserRepository: Send + Sync + 'static {
    fn create(&self, user: User) -> impl Future<Output = bool> + Send;
    fn get_by_email(
        &self,
        email: &str,
    ) -> impl Future<Output = Option<User>> + Send;
}
