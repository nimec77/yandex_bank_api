use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use rand_core::OsRng;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation, Algorithm};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

// Argon2 parameters for 50-150ms target latency
const ARGON2_M_COST: u32 = 19456; // 19 MB
const ARGON2_T_COST: u32 = 2;     // 2 iterations
const ARGON2_P_COST: u32 = 1;      // 1 parallelism

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
            .map_err(|e| argon2::password_hash::Error::from(e))?,
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
            .map_err(|e| argon2::password_hash::Error::from(e))?,
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

