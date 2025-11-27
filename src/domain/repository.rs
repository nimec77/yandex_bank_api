use crate::domain::models::Account;
use crate::domain::user::User;
use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait AccountRepository: Send + Sync {
    async fn save(&self, account: Account) -> Result<()>;
    async fn find_by_id(&self, id: u32) -> Result<Option<Account>>;
    async fn update(&self, account: Account) -> Result<()>;
}

#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn save_user(&self, user: User) -> Result<()>;
    async fn find_user_by_email(&self, email: &str) -> Result<Option<User>>;
    async fn find_user_by_id(&self, id: &str) -> Result<Option<User>>;
}
