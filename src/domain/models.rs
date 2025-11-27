use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Account {
    pub id: u32,
    pub name: String,
    pub balance: Amount,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, PartialOrd)]
#[serde(transparent)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_amount_new() {
        let amount = Amount::new(100);
        assert_eq!(amount.inner(), 100);
    }

    #[test]
    fn test_amount_inner() {
        let amount = Amount::new(500);
        assert_eq!(amount.inner(), 500);
    }

    #[test]
    fn test_amount_partial_eq() {
        let amount1 = Amount::new(100);
        let amount2 = Amount::new(100);
        let amount3 = Amount::new(200);

        assert_eq!(amount1, amount2);
        assert_ne!(amount1, amount3);
    }

    #[test]
    fn test_amount_partial_ord() {
        let amount1 = Amount::new(100);
        let amount2 = Amount::new(200);
        let amount3 = Amount::new(100);

        assert!(amount2 > amount1);
        assert!(amount1 < amount2);
        assert!(amount1 <= amount3);
        assert!(amount1 >= amount3);
    }

    #[test]
    fn test_amount_serialization() {
        let amount = Amount::new(12345);
        let json = serde_json::to_string(&amount).unwrap();
        assert_eq!(json, "12345");
    }

    #[test]
    fn test_amount_deserialization() {
        let json = "67890";
        let amount: Amount = serde_json::from_str(json).unwrap();
        assert_eq!(amount.inner(), 67890);
    }

    #[test]
    fn test_amount_zero() {
        let amount = Amount::new(0);
        assert_eq!(amount.inner(), 0);
    }

    #[test]
    fn test_amount_large_value() {
        let amount = Amount::new(u64::MAX);
        assert_eq!(amount.inner(), u64::MAX);
    }
}
