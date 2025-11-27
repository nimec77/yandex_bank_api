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
        trace!(account_id = account.id, new_balance = new_balance, "Updating account");
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
        trace!(account_id = account.id, new_balance = new_balance, "Updating account");
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
