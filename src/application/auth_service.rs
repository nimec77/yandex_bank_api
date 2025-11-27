use crate::domain::error::DomainError;
use crate::domain::repository::UserRepository;
use crate::domain::user::{CreateUser, LoginRequest, User};
use crate::infrastructure::security::{generate_token, hash_password, verify_password};
use anyhow::Result;
use std::sync::Arc;
use tracing::{debug, error, info, instrument, trace, warn};
use uuid::Uuid;

pub struct AuthService<R: UserRepository> {
    user_repository: Arc<R>,
    jwt_secret: String,
}

impl<R: UserRepository> AuthService<R> {
    pub fn new(user_repository: Arc<R>, jwt_secret: String) -> Self {
        Self {
            user_repository,
            jwt_secret,
        }
    }

    #[instrument(skip(self), fields(email = %req.email))]
    pub async fn register_user(&self, req: CreateUser) -> Result<User> {
        trace!("Starting user registration");
        
        // Check if user already exists
        if self.user_repository.find_user_by_email(&req.email).await?.is_some() {
            warn!(email = %req.email, "User already exists");
            return Err(DomainError::Validation("User with this email already exists".to_string()).into());
        }

        // Hash password
        let password_hash = hash_password(&req.password)
            .map_err(|e| {
                error!(error = %e, "Failed to hash password");
                DomainError::Internal(format!("Failed to hash password: {}", e))
            })?;

        // Create user
        let user = User {
            id: Uuid::new_v4().to_string(),
            email: req.email,
            password_hash,
        };

        debug!(user_id = %user.id, email = %user.email, "Saving user to repository");
        self.user_repository.save_user(user.clone()).await?;

        info!(
            user_id = %user.id,
            email = %user.email,
            "User registered successfully"
        );

        Ok(user)
    }

    #[instrument(skip(self), fields(email = %req.email))]
    pub async fn login(&self, req: LoginRequest) -> Result<String> {
        trace!("Starting login");
        
        let user = self.user_repository
            .find_user_by_email(&req.email)
            .await?
            .ok_or_else(|| {
                warn!(email = %req.email, "User not found during login");
                DomainError::Unauthorized("Invalid email or password".to_string())
            })?;

        // Verify password
        let is_valid = verify_password(&req.password, &user.password_hash)
            .map_err(|e| {
                error!(error = %e, "Failed to verify password");
                DomainError::Internal(format!("Failed to verify password: {}", e))
            })?;

        if !is_valid {
            warn!(user_id = %user.id, email = %user.email, "Invalid password during login");
            return Err(DomainError::Unauthorized("Invalid email or password".to_string()).into());
        }

        // Generate JWT token
        let token = generate_token(&user.id, &self.jwt_secret)
            .map_err(|e| {
                error!(error = %e, "Failed to generate token");
                DomainError::Internal(format!("Failed to generate token: {}", e))
            })?;

        info!(
            user_id = %user.id,
            email = %user.email,
            "Login successful"
        );

        Ok(token)
    }

    #[instrument(skip(self), fields(user_id = user_id))]
    pub async fn get_token(&self, user_id: &str) -> Result<String> {
        trace!("Generating token for user");
        
        // Verify user exists
        let user = self.user_repository
            .find_user_by_id(user_id)
            .await?
            .ok_or_else(|| {
                warn!(user_id = user_id, "User not found during token generation");
                DomainError::NotFound(format!("User not found: {}", user_id))
            })?;

        // Generate JWT token
        let token = generate_token(&user.id, &self.jwt_secret)
            .map_err(|e| {
                error!(error = %e, "Failed to generate token");
                DomainError::Internal(format!("Failed to generate token: {}", e))
            })?;

        info!(
            user_id = %user.id,
            email = %user.email,
            "Token generated successfully"
        );

        Ok(token)
    }
}

