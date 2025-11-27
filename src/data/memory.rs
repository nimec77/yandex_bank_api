use crate::domain::models::Account;
use crate::domain::repository::AccountRepository;
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, instrument, trace};

#[derive(Clone)]
pub struct InMemoryAccountRepository {
    storage: Arc<RwLock<HashMap<u32, Account>>>,
}

impl InMemoryAccountRepository {
    pub fn new() -> Self {
        Self {
            storage: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for InMemoryAccountRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AccountRepository for InMemoryAccountRepository {
    #[instrument(skip(self), fields(account_id = account.id))]
    async fn save(&self, account: Account) -> Result<()> {
        trace!("Acquiring write lock for storage");
        let mut storage = self.storage.write().await;
        trace!(account_id = account.id, "Inserting account into storage");
        storage.insert(account.id, account.clone());
        debug!(
            account_id = account.id,
            name = %account.name,
            balance = account.balance.inner(),
            "Account saved to memory storage"
        );
        Ok(())
    }

    #[instrument(skip(self), fields(account_id = id))]
    async fn find_by_id(&self, id: u32) -> Result<Option<Account>> {
        trace!("Acquiring read lock for storage");
        let storage = self.storage.read().await;
        trace!(account_id = id, "Looking up account in storage");
        let account = storage.get(&id).cloned();
        match &account {
            Some(acc) => {
                debug!(
                    account_id = acc.id,
                    balance = acc.balance.inner(),
                    "Account found in storage"
                );
            }
            None => {
                trace!(account_id = id, "Account not found in storage");
            }
        }
        Ok(account)
    }

    #[instrument(skip(self), fields(account_id = account.id))]
    async fn update(&self, account: Account) -> Result<()> {
        trace!("Acquiring write lock for storage");
        let mut storage = self.storage.write().await;
        trace!(account_id = account.id, "Updating account in storage");
        storage.insert(account.id, account.clone());
        debug!(
            account_id = account.id,
            balance = account.balance.inner(),
            "Account updated in memory storage"
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::models::{Account, Amount};

    #[tokio::test]
    async fn test_save_saves_account_correctly() {
        let repo = InMemoryAccountRepository::new();
        let account = Account {
            id: 1,
            name: "Test Account".to_string(),
            balance: Amount::new(100),
        };

        repo.save(account.clone()).await.unwrap();

        let retrieved = repo.find_by_id(1).await.unwrap();
        assert!(retrieved.is_some());
        let retrieved_account = retrieved.unwrap();
        assert_eq!(retrieved_account.id, account.id);
        assert_eq!(retrieved_account.name, account.name);
        assert_eq!(retrieved_account.balance.inner(), account.balance.inner());
    }

    #[tokio::test]
    async fn test_find_by_id_finds_existing_account() {
        let repo = InMemoryAccountRepository::new();
        let account = Account {
            id: 42,
            name: "Found Account".to_string(),
            balance: Amount::new(500),
        };

        repo.save(account.clone()).await.unwrap();
        let found = repo.find_by_id(42).await.unwrap();

        assert!(found.is_some());
        let found_account = found.unwrap();
        assert_eq!(found_account.id, 42);
        assert_eq!(found_account.name, "Found Account");
    }

    #[tokio::test]
    async fn test_find_by_id_returns_none_for_nonexistent_account() {
        let repo = InMemoryAccountRepository::new();

        let found = repo.find_by_id(999).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_update_updates_existing_account() {
        let repo = InMemoryAccountRepository::new();
        let mut account = Account {
            id: 1,
            name: "Original Name".to_string(),
            balance: Amount::new(100),
        };

        repo.save(account.clone()).await.unwrap();

        // Update account
        account.name = "Updated Name".to_string();
        account.balance = Amount::new(200);
        repo.update(account.clone()).await.unwrap();

        let retrieved = repo.find_by_id(1).await.unwrap().unwrap();
        assert_eq!(retrieved.name, "Updated Name");
        assert_eq!(retrieved.balance.inner(), 200);
    }

    #[tokio::test]
    async fn test_save_overwrites_existing_account() {
        let repo = InMemoryAccountRepository::new();
        let account1 = Account {
            id: 1,
            name: "First".to_string(),
            balance: Amount::new(100),
        };
        let account2 = Account {
            id: 1,
            name: "Second".to_string(),
            balance: Amount::new(200),
        };

        repo.save(account1).await.unwrap();
        repo.save(account2.clone()).await.unwrap();

        let retrieved = repo.find_by_id(1).await.unwrap().unwrap();
        assert_eq!(retrieved.name, "Second");
        assert_eq!(retrieved.balance.inner(), 200);
    }

    #[tokio::test]
    async fn test_concurrent_reads() {
        let repo = InMemoryAccountRepository::new();
        let account = Account {
            id: 1,
            name: "Concurrent".to_string(),
            balance: Amount::new(100),
        };

        repo.save(account).await.unwrap();

        // Spawn multiple concurrent reads
        let handles: Vec<_> = (0..10)
            .map(|_| {
                let repo_clone = repo.clone();
                tokio::spawn(async move { repo_clone.find_by_id(1).await })
            })
            .collect();

        for handle in handles {
            let result = handle.await.unwrap().unwrap();
            assert!(result.is_some());
            assert_eq!(result.unwrap().id, 1);
        }
    }

    #[tokio::test]
    async fn test_concurrent_writes() {
        let repo = InMemoryAccountRepository::new();

        // Spawn multiple concurrent writes
        let handles: Vec<_> = (0..10)
            .map(|i| {
                let repo_clone = repo.clone();
                let account = Account {
                    id: i,
                    name: format!("Account {}", i),
                    balance: Amount::new(i as u64 * 10),
                };
                tokio::spawn(async move { repo_clone.save(account).await })
            })
            .collect();

        for handle in handles {
            assert!(handle.await.unwrap().is_ok());
        }

        // Verify all accounts were saved
        for i in 0..10 {
            let found = repo.find_by_id(i).await.unwrap();
            assert!(found.is_some());
            assert_eq!(found.unwrap().id, i);
        }
    }

    #[tokio::test]
    async fn test_multiple_accounts() {
        let repo = InMemoryAccountRepository::new();

        for i in 1..=5 {
            let account = Account {
                id: i,
                name: format!("Account {}", i),
                balance: Amount::new(i as u64 * 100),
            };
            repo.save(account).await.unwrap();
        }

        // Verify all accounts exist
        for i in 1..=5 {
            let found = repo.find_by_id(i).await.unwrap();
            assert!(found.is_some());
            assert_eq!(found.unwrap().balance.inner(), i as u64 * 100);
        }
    }
}
