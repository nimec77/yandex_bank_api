use crate::domain::repository::UserRepository;
use crate::domain::user::User;
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, instrument, trace};

#[derive(Clone)]
pub struct InMemoryUserRepository {
    storage: Arc<RwLock<HashMap<String, User>>>,
}

impl InMemoryUserRepository {
    pub fn new() -> Self {
        Self {
            storage: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for InMemoryUserRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl UserRepository for InMemoryUserRepository {
    #[instrument(skip(self), fields(user_id = %user.id, email = %user.email))]
    async fn save_user(&self, user: User) -> Result<()> {
        trace!("Acquiring write lock for user storage");
        let mut storage = self.storage.write().await;
        trace!(user_id = %user.id, email = %user.email, "Inserting user into storage");
        storage.insert(user.id.clone(), user.clone());
        debug!(
            user_id = %user.id,
            email = %user.email,
            "User saved to memory storage"
        );
        Ok(())
    }

    #[instrument(skip(self), fields(email = email))]
    async fn find_user_by_email(&self, email: &str) -> Result<Option<User>> {
        trace!("Acquiring read lock for user storage");
        let storage = self.storage.read().await;
        trace!(email = email, "Looking up user by email in storage");
        let user = storage.values().find(|u| u.email == email).cloned();
        match &user {
            Some(u) => {
                debug!(
                    user_id = %u.id,
                    email = %u.email,
                    "User found in storage"
                );
            }
            None => {
                trace!(email = email, "User not found in storage");
            }
        }
        Ok(user)
    }

    #[instrument(skip(self), fields(user_id = id))]
    async fn find_user_by_id(&self, id: &str) -> Result<Option<User>> {
        trace!("Acquiring read lock for user storage");
        let storage = self.storage.read().await;
        trace!(user_id = id, "Looking up user by ID in storage");
        let user = storage.get(id).cloned();
        match &user {
            Some(u) => {
                debug!(
                    user_id = %u.id,
                    email = %u.email,
                    "User found in storage"
                );
            }
            None => {
                trace!(user_id = id, "User not found in storage");
            }
        }
        Ok(user)
    }
}

