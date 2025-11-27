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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::user::User;

    #[tokio::test]
    async fn test_save_user_saves_user_correctly() {
        let repo = InMemoryUserRepository::new();
        let user = User {
            id: "user-1".to_string(),
            email: "test@example.com".to_string(),
            password_hash: "hash123".to_string(),
        };

        repo.save_user(user.clone()).await.unwrap();

        let retrieved = repo.find_user_by_id("user-1").await.unwrap();
        assert!(retrieved.is_some());
        let retrieved_user = retrieved.unwrap();
        assert_eq!(retrieved_user.id, user.id);
        assert_eq!(retrieved_user.email, user.email);
        assert_eq!(retrieved_user.password_hash, user.password_hash);
    }

    #[tokio::test]
    async fn test_find_user_by_email_finds_user_by_email() {
        let repo = InMemoryUserRepository::new();
        let user = User {
            id: "user-2".to_string(),
            email: "alice@example.com".to_string(),
            password_hash: "hash456".to_string(),
        };

        repo.save_user(user.clone()).await.unwrap();
        let found = repo.find_user_by_email("alice@example.com").await.unwrap();

        assert!(found.is_some());
        let found_user = found.unwrap();
        assert_eq!(found_user.id, "user-2");
        assert_eq!(found_user.email, "alice@example.com");
    }

    #[tokio::test]
    async fn test_find_user_by_email_returns_none_for_nonexistent_email() {
        let repo = InMemoryUserRepository::new();

        let found = repo
            .find_user_by_email("nonexistent@example.com")
            .await
            .unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_find_user_by_id_finds_user_by_id() {
        let repo = InMemoryUserRepository::new();
        let user = User {
            id: "user-3".to_string(),
            email: "bob@example.com".to_string(),
            password_hash: "hash789".to_string(),
        };

        repo.save_user(user.clone()).await.unwrap();
        let found = repo.find_user_by_id("user-3").await.unwrap();

        assert!(found.is_some());
        let found_user = found.unwrap();
        assert_eq!(found_user.id, "user-3");
        assert_eq!(found_user.email, "bob@example.com");
    }

    #[tokio::test]
    async fn test_find_user_by_id_returns_none_for_nonexistent_id() {
        let repo = InMemoryUserRepository::new();

        let found = repo.find_user_by_id("nonexistent-id").await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_save_user_overwrites_existing_user() {
        let repo = InMemoryUserRepository::new();
        let user1 = User {
            id: "user-4".to_string(),
            email: "first@example.com".to_string(),
            password_hash: "hash1".to_string(),
        };
        let user2 = User {
            id: "user-4".to_string(),
            email: "second@example.com".to_string(),
            password_hash: "hash2".to_string(),
        };

        repo.save_user(user1).await.unwrap();
        repo.save_user(user2.clone()).await.unwrap();

        let retrieved = repo.find_user_by_id("user-4").await.unwrap().unwrap();
        assert_eq!(retrieved.email, "second@example.com");
        assert_eq!(retrieved.password_hash, "hash2");
    }

    #[tokio::test]
    async fn test_find_user_by_email_case_sensitive() {
        let repo = InMemoryUserRepository::new();
        let user = User {
            id: "user-5".to_string(),
            email: "Test@Example.com".to_string(),
            password_hash: "hash".to_string(),
        };

        repo.save_user(user).await.unwrap();

        // Exact match should work
        let found = repo.find_user_by_email("Test@Example.com").await.unwrap();
        assert!(found.is_some());

        // Different case should not match
        let not_found = repo.find_user_by_email("test@example.com").await.unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_concurrent_reads() {
        let repo = InMemoryUserRepository::new();
        let user = User {
            id: "user-6".to_string(),
            email: "concurrent@example.com".to_string(),
            password_hash: "hash".to_string(),
        };

        repo.save_user(user).await.unwrap();

        // Spawn multiple concurrent reads
        let handles: Vec<_> = (0..10)
            .map(|_| {
                let repo_clone = repo.clone();
                tokio::spawn(async move { repo_clone.find_user_by_id("user-6").await })
            })
            .collect();

        for handle in handles {
            let result = handle.await.unwrap().unwrap();
            assert!(result.is_some());
            assert_eq!(result.unwrap().id, "user-6");
        }
    }

    #[tokio::test]
    async fn test_concurrent_writes() {
        let repo = InMemoryUserRepository::new();

        // Spawn multiple concurrent writes
        let handles: Vec<_> = (0..10)
            .map(|i| {
                let repo_clone = repo.clone();
                let user = User {
                    id: format!("user-{}", i),
                    email: format!("user{}@example.com", i),
                    password_hash: format!("hash{}", i),
                };
                tokio::spawn(async move { repo_clone.save_user(user).await })
            })
            .collect();

        for handle in handles {
            assert!(handle.await.unwrap().is_ok());
        }

        // Verify all users were saved
        for i in 0..10 {
            let found = repo.find_user_by_id(&format!("user-{}", i)).await.unwrap();
            assert!(found.is_some());
            assert_eq!(found.unwrap().email, format!("user{}@example.com", i));
        }
    }

    #[tokio::test]
    async fn test_multiple_users() {
        let repo = InMemoryUserRepository::new();

        for i in 1..=5 {
            let user = User {
                id: format!("user-{}", i),
                email: format!("user{}@example.com", i),
                password_hash: format!("hash{}", i),
            };
            repo.save_user(user).await.unwrap();
        }

        // Verify all users exist by ID
        for i in 1..=5 {
            let found = repo.find_user_by_id(&format!("user-{}", i)).await.unwrap();
            assert!(found.is_some());
        }

        // Verify all users exist by email
        for i in 1..=5 {
            let found = repo
                .find_user_by_email(&format!("user{}@example.com", i))
                .await
                .unwrap();
            assert!(found.is_some());
        }
    }
}
