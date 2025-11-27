use thiserror::Error;

#[derive(Error, Debug)]
pub enum DomainError {
    #[error("Insufficient funds")]
    InsufficientFunds,
    #[error("Account not found")]
    AccountNotFound,
    #[error("Invalid amount")]
    InvalidAmount,
    #[error("Validation error: {0}")]
    Validation(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Unauthorized: {0}")]
    Unauthorized(String),
    #[error("Internal error: {0}")]
    Internal(String),
}

