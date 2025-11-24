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
