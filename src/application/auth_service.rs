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
        if self
            .user_repository
            .find_user_by_email(&req.email)
            .await?
            .is_some()
        {
            warn!(email = %req.email, "User already exists");
            return Err(
                DomainError::Validation("User with this email already exists".to_string()).into(),
            );
        }

        // Hash password
        let password_hash = hash_password(&req.password).map_err(|e| {
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

        let user = self
            .user_repository
            .find_user_by_email(&req.email)
            .await?
            .ok_or_else(|| {
                warn!(email = %req.email, "User not found during login");
                DomainError::Unauthorized("Invalid email or password".to_string())
            })?;

        // Verify password
        let is_valid = verify_password(&req.password, &user.password_hash).map_err(|e| {
            error!(error = %e, "Failed to verify password");
            DomainError::Internal(format!("Failed to verify password: {}", e))
        })?;

        if !is_valid {
            warn!(user_id = %user.id, email = %user.email, "Invalid password during login");
            return Err(DomainError::Unauthorized("Invalid email or password".to_string()).into());
        }

        // Generate JWT token
        let token = generate_token(&user.id, &self.jwt_secret).map_err(|e| {
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
        let user = self
            .user_repository
            .find_user_by_id(user_id)
            .await?
            .ok_or_else(|| {
                warn!(user_id = user_id, "User not found during token generation");
                DomainError::NotFound(format!("User not found: {}", user_id))
            })?;

        // Generate JWT token
        let token = generate_token(&user.id, &self.jwt_secret).map_err(|e| {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::user_repository::InMemoryUserRepository;
    use crate::domain::user::{CreateUser, LoginRequest};

    #[tokio::test]
    async fn test_register_user_registers_new_user_successfully() {
        let repo = Arc::new(InMemoryUserRepository::new());
        let service = AuthService::new(repo, "test_secret".to_string());

        let req = CreateUser {
            email: "newuser@example.com".to_string(),
            password: "password123".to_string(),
        };

        let user = service.register_user(req).await.unwrap();

        assert_eq!(user.email, "newuser@example.com");
        assert!(!user.id.is_empty());
        assert!(!user.password_hash.is_empty());
        // Password should be hashed, not stored plain
        assert_ne!(user.password_hash, "password123");
    }

    #[tokio::test]
    async fn test_register_user_returns_error_for_duplicate_email() {
        let repo = Arc::new(InMemoryUserRepository::new());
        let service = AuthService::new(repo, "test_secret".to_string());

        let req1 = CreateUser {
            email: "duplicate@example.com".to_string(),
            password: "password1".to_string(),
        };
        let req2 = CreateUser {
            email: "duplicate@example.com".to_string(),
            password: "password2".to_string(),
        };

        service.register_user(req1).await.unwrap();
        let result = service.register_user(req2).await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        if let Ok(domain_error) = error.downcast::<DomainError>() {
            match domain_error {
                DomainError::Validation(msg) => {
                    assert!(msg.contains("already exists"));
                }
                _ => panic!("Expected Validation error"),
            }
        }
    }

    #[tokio::test]
    async fn test_register_user_hashes_password() {
        let repo = Arc::new(InMemoryUserRepository::new());
        let service = AuthService::new(repo, "test_secret".to_string());

        let req = CreateUser {
            email: "hashtest@example.com".to_string(),
            password: "plain_password".to_string(),
        };

        let user = service.register_user(req).await.unwrap();

        // Password hash should not be the plain password
        assert_ne!(user.password_hash, "plain_password");
        // Hash should be verifiable
        let is_valid = verify_password("plain_password", &user.password_hash).unwrap();
        assert!(is_valid);
    }

    #[tokio::test]
    async fn test_login_logs_in_with_correct_credentials() {
        let repo = Arc::new(InMemoryUserRepository::new());
        let service = AuthService::new(repo, "test_secret".to_string());

        // Register user first
        let register_req = CreateUser {
            email: "login@example.com".to_string(),
            password: "correct_password".to_string(),
        };
        service.register_user(register_req).await.unwrap();

        // Login
        let login_req = LoginRequest {
            email: "login@example.com".to_string(),
            password: "correct_password".to_string(),
        };

        let token = service.login(login_req).await.unwrap();
        assert!(!token.is_empty());
    }

    #[tokio::test]
    async fn test_login_returns_error_for_wrong_password() {
        let repo = Arc::new(InMemoryUserRepository::new());
        let service = AuthService::new(repo, "test_secret".to_string());

        // Register user
        let register_req = CreateUser {
            email: "wrongpass@example.com".to_string(),
            password: "correct_password".to_string(),
        };
        service.register_user(register_req).await.unwrap();

        // Try to login with wrong password
        let login_req = LoginRequest {
            email: "wrongpass@example.com".to_string(),
            password: "wrong_password".to_string(),
        };

        let result = service.login(login_req).await;
        assert!(result.is_err());

        let error = result.unwrap_err();
        if let Ok(domain_error) = error.downcast::<DomainError>() {
            match domain_error {
                DomainError::Unauthorized(msg) => {
                    assert!(msg.contains("Invalid email or password"));
                }
                _ => panic!("Expected Unauthorized error"),
            }
        }
    }

    #[tokio::test]
    async fn test_login_returns_error_for_nonexistent_user() {
        let repo = Arc::new(InMemoryUserRepository::new());
        let service = AuthService::new(repo, "test_secret".to_string());

        let login_req = LoginRequest {
            email: "nonexistent@example.com".to_string(),
            password: "password".to_string(),
        };

        let result = service.login(login_req).await;
        assert!(result.is_err());

        let error = result.unwrap_err();
        if let Ok(domain_error) = error.downcast::<DomainError>() {
            match domain_error {
                DomainError::Unauthorized(msg) => {
                    assert!(msg.contains("Invalid email or password"));
                }
                _ => panic!("Expected Unauthorized error"),
            }
        }
    }

    #[tokio::test]
    async fn test_login_returns_valid_jwt_token() {
        let repo = Arc::new(InMemoryUserRepository::new());
        let jwt_secret = "test_secret_key".to_string();
        let service = AuthService::new(repo, jwt_secret.clone());

        // Register user
        let register_req = CreateUser {
            email: "token@example.com".to_string(),
            password: "password".to_string(),
        };
        let user = service.register_user(register_req).await.unwrap();

        // Login
        let login_req = LoginRequest {
            email: "token@example.com".to_string(),
            password: "password".to_string(),
        };
        let token = service.login(login_req).await.unwrap();

        // Validate token
        let extracted_user_id =
            crate::infrastructure::security::validate_token(&token, &jwt_secret).unwrap();
        assert_eq!(extracted_user_id, user.id);
    }

    #[tokio::test]
    async fn test_get_token_generates_token_for_existing_user() {
        let repo = Arc::new(InMemoryUserRepository::new());
        let jwt_secret = "test_secret".to_string();
        let service = AuthService::new(repo, jwt_secret.clone());

        // Register user
        let register_req = CreateUser {
            email: "gettoken@example.com".to_string(),
            password: "password".to_string(),
        };
        let user = service.register_user(register_req).await.unwrap();

        // Get token
        let token = service.get_token(&user.id).await.unwrap();
        assert!(!token.is_empty());

        // Validate token
        let extracted_user_id =
            crate::infrastructure::security::validate_token(&token, &jwt_secret).unwrap();
        assert_eq!(extracted_user_id, user.id);
    }

    #[tokio::test]
    async fn test_get_token_returns_error_for_nonexistent_user() {
        let repo = Arc::new(InMemoryUserRepository::new());
        let service = AuthService::new(repo, "test_secret".to_string());

        let result = service.get_token("nonexistent-user-id").await;
        assert!(result.is_err());

        let error = result.unwrap_err();
        if let Ok(domain_error) = error.downcast::<DomainError>() {
            match domain_error {
                DomainError::NotFound(msg) => {
                    assert!(msg.contains("User not found"));
                }
                _ => panic!("Expected NotFound error"),
            }
        }
    }

    #[tokio::test]
    async fn test_multiple_users_can_register() {
        let repo = Arc::new(InMemoryUserRepository::new());
        let service = AuthService::new(repo, "test_secret".to_string());

        let req1 = CreateUser {
            email: "user1@example.com".to_string(),
            password: "pass1".to_string(),
        };
        let req2 = CreateUser {
            email: "user2@example.com".to_string(),
            password: "pass2".to_string(),
        };

        let user1 = service.register_user(req1).await.unwrap();
        let user2 = service.register_user(req2).await.unwrap();

        assert_ne!(user1.id, user2.id);
        assert_ne!(user1.email, user2.email);
    }

    #[tokio::test]
    async fn test_login_with_different_passwords() {
        let repo = Arc::new(InMemoryUserRepository::new());
        let service = AuthService::new(repo, "test_secret".to_string());

        // Register with password1
        let register_req = CreateUser {
            email: "multipass@example.com".to_string(),
            password: "password1".to_string(),
        };
        service.register_user(register_req).await.unwrap();

        // Login with correct password should work
        let login_req1 = LoginRequest {
            email: "multipass@example.com".to_string(),
            password: "password1".to_string(),
        };
        assert!(service.login(login_req1).await.is_ok());

        // Login with wrong password should fail
        let login_req2 = LoginRequest {
            email: "multipass@example.com".to_string(),
            password: "password2".to_string(),
        };
        assert!(service.login(login_req2).await.is_err());
    }
}
