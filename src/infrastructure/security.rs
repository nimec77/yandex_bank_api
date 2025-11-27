use argon2::Argon2;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use rand_core::OsRng;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

// Argon2 parameters for 50-150ms target latency
const ARGON2_M_COST: u32 = 19456; // 19 MB
const ARGON2_T_COST: u32 = 2; // 2 iterations
const ARGON2_P_COST: u32 = 1; // 1 parallelism

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String, // user_id
    exp: usize,
    iat: usize,
}

pub fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::new(
        argon2::Algorithm::Argon2id,
        argon2::Version::V0x13,
        argon2::Params::new(ARGON2_M_COST, ARGON2_T_COST, ARGON2_P_COST, None)
            .map_err(argon2::password_hash::Error::from)?,
    );

    let password_hash = argon2.hash_password(password.as_bytes(), &salt)?;
    Ok(password_hash.to_string())
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool, argon2::password_hash::Error> {
    let parsed_hash = PasswordHash::new(hash)?;
    let argon2 = Argon2::new(
        argon2::Algorithm::Argon2id,
        argon2::Version::V0x13,
        argon2::Params::new(ARGON2_M_COST, ARGON2_T_COST, ARGON2_P_COST, None)
            .map_err(argon2::password_hash::Error::from)?,
    );

    match argon2.verify_password(password.as_bytes(), &parsed_hash) {
        Ok(()) => Ok(true),
        Err(_) => Ok(false),
    }
}

pub fn generate_token(user_id: &str, secret: &str) -> Result<String, jsonwebtoken::errors::Error> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize;

    let exp = now + 3600; // 1 hour expiration

    let claims = Claims {
        sub: user_id.to_string(),
        exp,
        iat: now,
    };

    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret.as_ref()),
    )
}

pub fn validate_token(token: &str, secret: &str) -> Result<String, jsonwebtoken::errors::Error> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.leeway = 60; // 60 seconds leeway

    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_ref()),
        &validation,
    )?;

    Ok(token_data.claims.sub)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_password_generates_valid_hash() {
        let password = "test_password_123";
        let hash = hash_password(password).unwrap();

        // Hash should not be empty
        assert!(!hash.is_empty());
        // Hash should be different from password
        assert_ne!(hash, password);
        // Hash should start with $argon2id$ (Argon2id format)
        assert!(hash.starts_with("$argon2id$"));
    }

    #[test]
    fn test_hash_password_different_passwords_produce_different_hashes() {
        let password1 = "password1";
        let password2 = "password2";

        let hash1 = hash_password(password1).unwrap();
        let hash2 = hash_password(password2).unwrap();

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_hash_password_same_password_produces_different_hashes() {
        let password = "same_password";

        let hash1 = hash_password(password).unwrap();
        let hash2 = hash_password(password).unwrap();

        // Due to random salt, same password should produce different hashes
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_verify_password_correct_password_returns_true() {
        let password = "correct_password";
        let hash = hash_password(password).unwrap();

        let result = verify_password(password, &hash).unwrap();
        assert!(result);
    }

    #[test]
    fn test_verify_password_incorrect_password_returns_false() {
        let password = "correct_password";
        let wrong_password = "wrong_password";
        let hash = hash_password(password).unwrap();

        let result = verify_password(wrong_password, &hash).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_verify_password_invalid_hash_format() {
        let password = "test_password";
        let invalid_hash = "not_a_valid_hash";

        let result = verify_password(password, invalid_hash);
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_token_creates_valid_token() {
        let user_id = "test_user_123";
        let secret = "test_secret_key";

        let token = generate_token(user_id, secret).unwrap();

        // Token should not be empty
        assert!(!token.is_empty());
        // JWT tokens have 3 parts separated by dots
        let parts: Vec<&str> = token.split('.').collect();
        assert_eq!(parts.len(), 3);
    }

    #[test]
    fn test_generate_token_contains_correct_user_id() {
        let user_id = "user_456";
        let secret = "test_secret";

        let token = generate_token(user_id, secret).unwrap();
        let extracted_user_id = validate_token(&token, secret).unwrap();

        assert_eq!(extracted_user_id, user_id);
    }

    #[test]
    fn test_validate_token_validates_correct_token() {
        let user_id = "test_user";
        let secret = "secret_key";

        let token = generate_token(user_id, secret).unwrap();
        let extracted_user_id = validate_token(&token, secret).unwrap();

        assert_eq!(extracted_user_id, user_id);
    }

    #[test]
    fn test_validate_token_rejects_invalid_token() {
        let secret = "secret_key";
        let invalid_token = "invalid.token.here";

        let result = validate_token(invalid_token, secret);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_token_rejects_token_with_wrong_secret() {
        let user_id = "test_user";
        let correct_secret = "correct_secret";
        let wrong_secret = "wrong_secret";

        let token = generate_token(user_id, correct_secret).unwrap();
        let result = validate_token(&token, wrong_secret);

        assert!(result.is_err());
    }

    #[test]
    fn test_token_round_trip() {
        let user_id = "round_trip_user";
        let secret = "round_trip_secret";

        // Generate token
        let token = generate_token(user_id, secret).unwrap();

        // Validate token
        let extracted_user_id = validate_token(&token, secret).unwrap();

        // Should match original
        assert_eq!(extracted_user_id, user_id);
    }

    #[test]
    fn test_generate_token_different_users_produce_different_tokens() {
        let secret = "test_secret";
        let user1 = "user1";
        let user2 = "user2";

        let token1 = generate_token(user1, secret).unwrap();
        let token2 = generate_token(user2, secret).unwrap();

        assert_ne!(token1, token2);
    }

    #[test]
    fn test_verify_password_with_empty_password() {
        let password = "";
        let hash = hash_password(password).unwrap();

        let result = verify_password(password, &hash).unwrap();
        assert!(result);
    }

    #[test]
    fn test_verify_password_with_special_characters() {
        let password = "p@ssw0rd!#$%^&*()";
        let hash = hash_password(password).unwrap();

        let result = verify_password(password, &hash).unwrap();
        assert!(result);
    }

    #[test]
    fn test_verify_password_with_unicode() {
        let password = "пароль123";
        let hash = hash_password(password).unwrap();

        let result = verify_password(password, &hash).unwrap();
        assert!(result);
    }
}
