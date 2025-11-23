use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Account {
    pub id: u32,
    pub name: String,
    pub balance: Amount,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, PartialOrd)]
pub struct Amount(u64);

impl Amount {
    pub fn new(value: u64) -> Self {
        Amount(value)
    }

    pub fn inner(&self) -> u64 {
        self.0
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateAccount {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Transfer {
    pub from_account_id: u32,
    pub to_account_id: u32,
    pub amount: Amount,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Deposit {
    pub amount: Amount,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Withdraw {
    pub amount: Amount,
}

#[derive(Error, Debug)]
pub enum DomainError {
    #[error("Insufficient funds")]
    InsufficientFunds,
    #[error("Account not found")]
    AccountNotFound,
    #[error("Invalid amount")]
    InvalidAmount,
}
