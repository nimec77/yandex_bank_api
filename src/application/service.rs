use crate::domain::models::{Account, Amount, CreateAccount, DomainError, Transfer};
use crate::domain::repository::AccountRepository;
use anyhow::Result;
use std::sync::Arc;

pub struct BankService<R: AccountRepository> {
    repository: Arc<R>,
}

impl<R: AccountRepository> BankService<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }

    pub async fn create_account(&self, req: CreateAccount) -> Result<Account> {
        let id = fastrand::u32(..); // Simple ID generation
        let account = Account {
            id,
            name: req.name,
            balance: Amount::new(0),
        };
        self.repository.save(account.clone()).await?;
        Ok(account)
    }

    pub async fn get_account(&self, id: u32) -> Result<Account> {
        self.repository
            .find_by_id(id)
            .await?
            .ok_or_else(|| DomainError::AccountNotFound.into())
    }

    pub async fn deposit(&self, id: u32, amount: Amount) -> Result<Account> {
        let mut account = self.get_account(id).await?;
        let new_balance = account.balance.inner() + amount.inner();
        account.balance = Amount::new(new_balance);
        self.repository.update(account.clone()).await?;
        Ok(account)
    }

    pub async fn withdraw(&self, id: u32, amount: Amount) -> Result<Account> {
        let mut account = self.get_account(id).await?;
        if account.balance.inner() < amount.inner() {
            return Err(DomainError::InsufficientFunds.into());
        }
        let new_balance = account.balance.inner() - amount.inner();
        account.balance = Amount::new(new_balance);
        self.repository.update(account.clone()).await?;
        Ok(account)
    }

    pub async fn transfer(&self, req: Transfer) -> Result<()> {
        if req.from_account_id == req.to_account_id {
            return Err(DomainError::InvalidAmount.into()); // Or specific error
        }

        // Note: This is not transactional in memory without a mutex over both,
        // but for this exercise we'll do sequential updates.
        // In a real DB, this would be a transaction.

        let mut from_account = self.get_account(req.from_account_id).await?;
        let mut to_account = self.get_account(req.to_account_id).await?;

        if from_account.balance.inner() < req.amount.inner() {
            return Err(DomainError::InsufficientFunds.into());
        }

        let new_from_balance = from_account.balance.inner() - req.amount.inner();
        from_account.balance = Amount::new(new_from_balance);

        let new_to_balance = to_account.balance.inner() + req.amount.inner();
        to_account.balance = Amount::new(new_to_balance);

        self.repository.update(from_account).await?;
        self.repository.update(to_account).await?;

        Ok(())
    }
}
