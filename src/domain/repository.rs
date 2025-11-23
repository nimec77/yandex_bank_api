use crate::domain::models::Account;
use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait AccountRepository: Send + Sync {
    async fn save(&self, account: Account) -> Result<()>;
    async fn find_by_id(&self, id: u32) -> Result<Option<Account>>;
    async fn update(&self, account: Account) -> Result<()>;
}
