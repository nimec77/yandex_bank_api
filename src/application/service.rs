use crate::domain::error::DomainError;
use crate::domain::models::{Account, Amount, CreateAccount, Transfer};
use crate::domain::repository::AccountRepository;
use anyhow::Result;
use std::sync::Arc;
use tracing::{debug, info, instrument, trace, warn};

pub struct BankService<R: AccountRepository> {
    repository: Arc<R>,
}

impl<R: AccountRepository> BankService<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }

    #[instrument(skip(self), fields(name = %req.name))]
    pub async fn create_account(&self, req: CreateAccount) -> Result<Account> {
        trace!("Starting account creation");
        let id = fastrand::u32(..); // Simple ID generation
        debug!(account_id = id, "Generated account ID");
        let account = Account {
            id,
            name: req.name,
            balance: Amount::new(0),
        };
        trace!(account_id = account.id, "Saving account to repository");
        self.repository.save(account.clone()).await?;
        info!(
            account_id = account.id,
            name = %account.name,
            balance = account.balance.inner(),
            "Account created successfully"
        );
        Ok(account)
    }

    #[instrument(skip(self), fields(account_id = id))]
    pub async fn get_account(&self, id: u32) -> Result<Account> {
        trace!("Fetching account from repository");
        match self.repository.find_by_id(id).await? {
            Some(account) => {
                debug!(
                    account_id = account.id,
                    balance = account.balance.inner(),
                    "Account found"
                );
                Ok(account)
            }
            None => {
                warn!(account_id = id, "Account not found");
                Err(DomainError::AccountNotFound.into())
            }
        }
    }

    #[instrument(skip(self), fields(account_id = id, amount = amount.inner()))]
    pub async fn deposit(&self, id: u32, amount: Amount) -> Result<Account> {
        trace!("Starting deposit operation");
        let mut account = self.get_account(id).await?;
        let old_balance = account.balance.inner();
        let deposit_amount = amount.inner();
        debug!(
            account_id = account.id,
            old_balance = old_balance,
            deposit_amount = deposit_amount,
            "Calculating new balance"
        );
        let new_balance = old_balance + deposit_amount;
        account.balance = Amount::new(new_balance);
        trace!(
            account_id = account.id,
            new_balance = new_balance,
            "Updating account"
        );
        self.repository.update(account.clone()).await?;
        info!(
            account_id = account.id,
            old_balance = old_balance,
            deposit_amount = deposit_amount,
            new_balance = new_balance,
            "Deposit completed"
        );
        Ok(account)
    }

    #[instrument(skip(self), fields(account_id = id, amount = amount.inner()))]
    pub async fn withdraw(&self, id: u32, amount: Amount) -> Result<Account> {
        trace!("Starting withdrawal operation");
        let mut account = self.get_account(id).await?;
        let current_balance = account.balance.inner();
        let withdrawal_amount = amount.inner();
        debug!(
            account_id = account.id,
            current_balance = current_balance,
            withdrawal_amount = withdrawal_amount,
            "Checking sufficient funds"
        );
        if current_balance < withdrawal_amount {
            warn!(
                account_id = account.id,
                current_balance = current_balance,
                withdrawal_amount = withdrawal_amount,
                "Insufficient funds for withdrawal"
            );
            return Err(DomainError::InsufficientFunds.into());
        }
        let new_balance = current_balance - withdrawal_amount;
        account.balance = Amount::new(new_balance);
        trace!(
            account_id = account.id,
            new_balance = new_balance,
            "Updating account"
        );
        self.repository.update(account.clone()).await?;
        info!(
            account_id = account.id,
            old_balance = current_balance,
            withdrawal_amount = withdrawal_amount,
            new_balance = new_balance,
            "Withdrawal completed"
        );
        Ok(account)
    }

    #[instrument(skip(self), fields(
        from_account_id = req.from_account_id,
        to_account_id = req.to_account_id,
        amount = req.amount.inner()
    ))]
    pub async fn transfer(&self, req: Transfer) -> Result<()> {
        trace!("Starting transfer operation");
        if req.from_account_id == req.to_account_id {
            warn!(
                from_account_id = req.from_account_id,
                to_account_id = req.to_account_id,
                "Transfer to same account attempted"
            );
            return Err(DomainError::InvalidAmount.into());
        }

        // Note: This is not transactional in memory without a mutex over both,
        // but for this exercise we'll do sequential updates.
        // In a real DB, this would be a transaction.

        debug!(
            from_account_id = req.from_account_id,
            "Fetching source account"
        );
        let mut from_account = self.get_account(req.from_account_id).await?;
        debug!(
            to_account_id = req.to_account_id,
            "Fetching destination account"
        );
        let mut to_account = self.get_account(req.to_account_id).await?;

        let transfer_amount = req.amount.inner();
        let from_balance = from_account.balance.inner();
        let to_balance = to_account.balance.inner();

        debug!(
            from_account_id = from_account.id,
            from_balance = from_balance,
            transfer_amount = transfer_amount,
            "Checking sufficient funds in source account"
        );

        if from_balance < transfer_amount {
            warn!(
                from_account_id = from_account.id,
                from_balance = from_balance,
                transfer_amount = transfer_amount,
                "Insufficient funds for transfer"
            );
            return Err(DomainError::InsufficientFunds.into());
        }

        let new_from_balance = from_balance - transfer_amount;
        from_account.balance = Amount::new(new_from_balance);

        let new_to_balance = to_balance + transfer_amount;
        to_account.balance = Amount::new(new_to_balance);

        trace!(
            from_account_id = from_account.id,
            new_from_balance = new_from_balance,
            "Updating source account"
        );
        self.repository.update(from_account).await?;
        trace!(
            to_account_id = to_account.id,
            new_to_balance = new_to_balance,
            "Updating destination account"
        );
        self.repository.update(to_account).await?;

        info!(
            from_account_id = req.from_account_id,
            to_account_id = req.to_account_id,
            transfer_amount = transfer_amount,
            "Transfer completed successfully"
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::memory::InMemoryAccountRepository;
    use crate::domain::models::{Account, Amount, CreateAccount, Transfer};

    #[tokio::test]
    async fn test_create_account_creates_account_with_zero_balance() {
        let repo = Arc::new(InMemoryAccountRepository::new());
        let service = BankService::new(repo);

        let req = CreateAccount {
            name: "Test Account".to_string(),
        };

        let account = service.create_account(req).await.unwrap();
        assert_eq!(account.name, "Test Account");
        assert_eq!(account.balance.inner(), 0);
    }

    #[tokio::test]
    async fn test_create_account_generates_unique_ids() {
        let repo = Arc::new(InMemoryAccountRepository::new());
        let service = BankService::new(repo);

        let req1 = CreateAccount {
            name: "Account 1".to_string(),
        };
        let req2 = CreateAccount {
            name: "Account 2".to_string(),
        };

        let account1 = service.create_account(req1).await.unwrap();
        let account2 = service.create_account(req2).await.unwrap();

        // IDs might be the same due to randomness, but accounts should be different
        assert_ne!(account1.id, account2.id);
    }

    #[tokio::test]
    async fn test_get_account_retrieves_existing_account() {
        let repo = Arc::new(InMemoryAccountRepository::new());
        let service = BankService::new(repo.clone());

        // Create account directly in repository
        let account = Account {
            id: 42,
            name: "Existing Account".to_string(),
            balance: Amount::new(100),
        };
        repo.save(account.clone()).await.unwrap();

        let retrieved = service.get_account(42).await.unwrap();
        assert_eq!(retrieved.id, 42);
        assert_eq!(retrieved.name, "Existing Account");
        assert_eq!(retrieved.balance.inner(), 100);
    }

    #[tokio::test]
    async fn test_get_account_returns_error_for_nonexistent_account() {
        let repo = Arc::new(InMemoryAccountRepository::new());
        let service = BankService::new(repo);

        let result = service.get_account(999).await;
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert!(error.downcast_ref::<DomainError>().is_some());
        if let Ok(domain_error) = error.downcast::<DomainError>() {
            matches!(domain_error, DomainError::AccountNotFound);
        }
    }

    #[tokio::test]
    async fn test_deposit_adds_amount_correctly() {
        let repo = Arc::new(InMemoryAccountRepository::new());
        let service = BankService::new(repo.clone());

        let account = Account {
            id: 1,
            name: "Test".to_string(),
            balance: Amount::new(100),
        };
        repo.save(account).await.unwrap();

        let updated = service.deposit(1, Amount::new(50)).await.unwrap();
        assert_eq!(updated.balance.inner(), 150);
    }

    #[tokio::test]
    async fn test_deposit_returns_error_for_nonexistent_account() {
        let repo = Arc::new(InMemoryAccountRepository::new());
        let service = BankService::new(repo);

        let result = service.deposit(999, Amount::new(100)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_withdraw_subtracts_amount_correctly() {
        let repo = Arc::new(InMemoryAccountRepository::new());
        let service = BankService::new(repo.clone());

        let account = Account {
            id: 1,
            name: "Test".to_string(),
            balance: Amount::new(100),
        };
        repo.save(account).await.unwrap();

        let updated = service.withdraw(1, Amount::new(30)).await.unwrap();
        assert_eq!(updated.balance.inner(), 70);
    }

    #[tokio::test]
    async fn test_withdraw_returns_error_for_insufficient_funds() {
        let repo = Arc::new(InMemoryAccountRepository::new());
        let service = BankService::new(repo.clone());

        let account = Account {
            id: 1,
            name: "Test".to_string(),
            balance: Amount::new(50),
        };
        repo.save(account).await.unwrap();

        let result = service.withdraw(1, Amount::new(100)).await;
        assert!(result.is_err());

        let error = result.unwrap_err();
        if let Ok(domain_error) = error.downcast::<DomainError>() {
            matches!(domain_error, DomainError::InsufficientFunds);
        }
    }

    #[tokio::test]
    async fn test_withdraw_returns_error_for_nonexistent_account() {
        let repo = Arc::new(InMemoryAccountRepository::new());
        let service = BankService::new(repo);

        let result = service.withdraw(999, Amount::new(100)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_withdraw_allows_withdrawing_exact_balance() {
        let repo = Arc::new(InMemoryAccountRepository::new());
        let service = BankService::new(repo.clone());

        let account = Account {
            id: 1,
            name: "Test".to_string(),
            balance: Amount::new(100),
        };
        repo.save(account).await.unwrap();

        let updated = service.withdraw(1, Amount::new(100)).await.unwrap();
        assert_eq!(updated.balance.inner(), 0);
    }

    #[tokio::test]
    async fn test_transfer_transfers_between_accounts_correctly() {
        let repo = Arc::new(InMemoryAccountRepository::new());
        let service = BankService::new(repo.clone());

        let account1 = Account {
            id: 1,
            name: "Alice".to_string(),
            balance: Amount::new(100),
        };
        let account2 = Account {
            id: 2,
            name: "Bob".to_string(),
            balance: Amount::new(50),
        };
        repo.save(account1).await.unwrap();
        repo.save(account2).await.unwrap();

        let transfer = Transfer {
            from_account_id: 1,
            to_account_id: 2,
            amount: Amount::new(30),
        };

        service.transfer(transfer).await.unwrap();

        let alice = service.get_account(1).await.unwrap();
        let bob = service.get_account(2).await.unwrap();

        assert_eq!(alice.balance.inner(), 70);
        assert_eq!(bob.balance.inner(), 80);
    }

    #[tokio::test]
    async fn test_transfer_returns_error_for_same_account() {
        let repo = Arc::new(InMemoryAccountRepository::new());
        let service = BankService::new(repo.clone());

        let account = Account {
            id: 1,
            name: "Test".to_string(),
            balance: Amount::new(100),
        };
        repo.save(account).await.unwrap();

        let transfer = Transfer {
            from_account_id: 1,
            to_account_id: 1,
            amount: Amount::new(50),
        };

        let result = service.transfer(transfer).await;
        assert!(result.is_err());

        let error = result.unwrap_err();
        if let Ok(domain_error) = error.downcast::<DomainError>() {
            matches!(domain_error, DomainError::InvalidAmount);
        }
    }

    #[tokio::test]
    async fn test_transfer_returns_error_for_insufficient_funds() {
        let repo = Arc::new(InMemoryAccountRepository::new());
        let service = BankService::new(repo.clone());

        let account1 = Account {
            id: 1,
            name: "Alice".to_string(),
            balance: Amount::new(50),
        };
        let account2 = Account {
            id: 2,
            name: "Bob".to_string(),
            balance: Amount::new(100),
        };
        repo.save(account1).await.unwrap();
        repo.save(account2).await.unwrap();

        let transfer = Transfer {
            from_account_id: 1,
            to_account_id: 2,
            amount: Amount::new(100),
        };

        let result = service.transfer(transfer).await;
        assert!(result.is_err());

        let error = result.unwrap_err();
        if let Ok(domain_error) = error.downcast::<DomainError>() {
            matches!(domain_error, DomainError::InsufficientFunds);
        }
    }

    #[tokio::test]
    async fn test_transfer_returns_error_for_nonexistent_from_account() {
        let repo = Arc::new(InMemoryAccountRepository::new());
        let service = BankService::new(repo.clone());

        let account2 = Account {
            id: 2,
            name: "Bob".to_string(),
            balance: Amount::new(100),
        };
        repo.save(account2).await.unwrap();

        let transfer = Transfer {
            from_account_id: 999,
            to_account_id: 2,
            amount: Amount::new(50),
        };

        let result = service.transfer(transfer).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_transfer_returns_error_for_nonexistent_to_account() {
        let repo = Arc::new(InMemoryAccountRepository::new());
        let service = BankService::new(repo.clone());

        let account1 = Account {
            id: 1,
            name: "Alice".to_string(),
            balance: Amount::new(100),
        };
        repo.save(account1).await.unwrap();

        let transfer = Transfer {
            from_account_id: 1,
            to_account_id: 999,
            amount: Amount::new(50),
        };

        let result = service.transfer(transfer).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_multiple_deposits() {
        let repo = Arc::new(InMemoryAccountRepository::new());
        let service = BankService::new(repo.clone());

        let account = Account {
            id: 1,
            name: "Test".to_string(),
            balance: Amount::new(100),
        };
        repo.save(account).await.unwrap();

        service.deposit(1, Amount::new(50)).await.unwrap();
        service.deposit(1, Amount::new(25)).await.unwrap();
        service.deposit(1, Amount::new(10)).await.unwrap();

        let final_account = service.get_account(1).await.unwrap();
        assert_eq!(final_account.balance.inner(), 185);
    }

    #[tokio::test]
    async fn test_multiple_withdrawals() {
        let repo = Arc::new(InMemoryAccountRepository::new());
        let service = BankService::new(repo.clone());

        let account = Account {
            id: 1,
            name: "Test".to_string(),
            balance: Amount::new(100),
        };
        repo.save(account).await.unwrap();

        service.withdraw(1, Amount::new(30)).await.unwrap();
        service.withdraw(1, Amount::new(20)).await.unwrap();

        let final_account = service.get_account(1).await.unwrap();
        assert_eq!(final_account.balance.inner(), 50);
    }
}
