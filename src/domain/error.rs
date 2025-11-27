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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insufficient_funds_display() {
        let error = DomainError::InsufficientFunds;
        assert_eq!(error.to_string(), "Insufficient funds");
    }

    #[test]
    fn test_account_not_found_display() {
        let error = DomainError::AccountNotFound;
        assert_eq!(error.to_string(), "Account not found");
    }

    #[test]
    fn test_invalid_amount_display() {
        let error = DomainError::InvalidAmount;
        assert_eq!(error.to_string(), "Invalid amount");
    }

    #[test]
    fn test_validation_error_display() {
        let error = DomainError::Validation("Email already exists".to_string());
        assert_eq!(error.to_string(), "Validation error: Email already exists");
    }

    #[test]
    fn test_not_found_error_display() {
        let error = DomainError::NotFound("User not found".to_string());
        assert_eq!(error.to_string(), "Not found: User not found");
    }

    #[test]
    fn test_unauthorized_error_display() {
        let error = DomainError::Unauthorized("Invalid token".to_string());
        assert_eq!(error.to_string(), "Unauthorized: Invalid token");
    }

    #[test]
    fn test_internal_error_display() {
        let error = DomainError::Internal("Database connection failed".to_string());
        assert_eq!(
            error.to_string(),
            "Internal error: Database connection failed"
        );
    }

    #[test]
    fn test_error_debug() {
        let error = DomainError::InsufficientFunds;
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("InsufficientFunds"));
    }
}
