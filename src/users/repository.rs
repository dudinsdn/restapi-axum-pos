use std::collections::HashMap;
use std::future::Future;

use parking_lot::RwLock;

use super::model::User;

pub trait UserRepository: Send + Sync + 'static {
    fn create(&self, user: User) -> impl Future<Output = bool> + Send;
    fn get_by_email(
        &self,
        email: &str,
    ) -> impl Future<Output = Option<User>> + Send;
}

#[derive(Debug, Default)]
pub struct InMemoryUserRepository {
    data: RwLock<HashMap<String, User>>,
}

impl InMemoryUserRepository {
    pub fn new() -> Self {
        Self::default()
    }
}

impl UserRepository for InMemoryUserRepository {
    async fn create(&self, user: User) -> bool {
        // Email must be globally unique (used as the login identity),
        // unlike slug/sku which are only unique per tenant. Check + insert
        // in a single write-lock so it's atomic.
        let mut data = self.data.write();

        let email_taken =
            data.values().any(|existing| existing.email == user.email);
        if email_taken || data.contains_key(&user.id) {
            return false;
        }

        data.insert(user.id.clone(), user);
        true
    }

    async fn get_by_email(&self, email: &str) -> Option<User> {
        self.data
            .read()
            .values()
            .find(|user| user.email == email)
            .cloned()
    }
}
