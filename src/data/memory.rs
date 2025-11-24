use crate::domain::models::Account;
use crate::domain::repository::AccountRepository;
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

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
    async fn save(&self, account: Account) -> Result<()> {
        let mut storage = self.storage.write().await;
        storage.insert(account.id, account);
        Ok(())
    }

    async fn find_by_id(&self, id: u32) -> Result<Option<Account>> {
        let storage = self.storage.read().await;
        Ok(storage.get(&id).cloned())
    }

    async fn update(&self, account: Account) -> Result<()> {
        let mut storage = self.storage.write().await;
        storage.insert(account.id, account);
        Ok(())
    }
}
