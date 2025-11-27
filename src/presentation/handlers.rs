use crate::application::auth_service::AuthService;
use crate::application::service::BankService;
use crate::data::memory::InMemoryAccountRepository;
use crate::data::user_repository::InMemoryUserRepository;
use crate::domain::error::DomainError;
use crate::domain::models::{CreateAccount, Deposit, Transfer, Withdraw};
use crate::presentation::middleware::AuthenticatedUser;
use actix_web::{FromRequest, HttpMessage, HttpResponse, ResponseError, web};
use chrono::Utc;
use serde::Serialize;
use std::pin::Pin;
use std::sync::Arc;
use thiserror::Error;
use tracing::{error, info, instrument, warn};

// AppState holding the service
pub struct AppState {
    pub service: BankService<InMemoryAccountRepository>,
    pub auth_service: Arc<AuthService<InMemoryUserRepository>>,
}

// Uniform error response format
#[derive(Serialize)]
struct ErrorResponse {
    error: String,
    details: serde_json::Value,
}

// Bank API Error Types
#[derive(Error, Debug)]
pub enum BankError {
    #[error("Validation error: {0}")]
    Validation(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Insufficient funds")]
    InsufficientFunds,
    #[error("Unauthorized: {0}")]
    Unauthorized(String),
    #[error("Database error: {0}")]
    Database(String),
    #[error("Internal error: {0}")]
    Internal(String),
}

impl ResponseError for BankError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match self {
            BankError::Validation(_) => actix_web::http::StatusCode::BAD_REQUEST,
            BankError::NotFound(_) => actix_web::http::StatusCode::NOT_FOUND,
            BankError::InsufficientFunds => actix_web::http::StatusCode::BAD_REQUEST,
            BankError::Unauthorized(_) => actix_web::http::StatusCode::UNAUTHORIZED,
            BankError::Database(_) => actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
            BankError::Internal(_) => actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        let status = self.status_code();
        let error_msg = self.to_string();

        let details = match self {
            BankError::Validation(msg) => serde_json::json!({ "message": msg }),
            BankError::NotFound(msg) => serde_json::json!({ "message": msg }),
            BankError::InsufficientFunds => serde_json::json!({ "message": "Insufficient funds" }),
            BankError::Unauthorized(msg) => serde_json::json!({ "message": msg }),
            BankError::Database(msg) => serde_json::json!({ "message": msg }),
            BankError::Internal(msg) => serde_json::json!({ "message": msg }),
        };

        // Log error based on severity
        match self {
            BankError::Validation(_) => {
                warn!(error = %error_msg, status = %status, "Validation error")
            }
            BankError::NotFound(_) => {
                warn!(error = %error_msg, status = %status, "Resource not found")
            }
            BankError::InsufficientFunds => {
                warn!(error = %error_msg, status = %status, "Insufficient funds")
            }
            BankError::Unauthorized(_) => {
                warn!(error = %error_msg, status = %status, "Unauthorized")
            }
            BankError::Database(_) => {
                error!(error = %error_msg, status = %status, "Database error")
            }
            BankError::Internal(_) => {
                error!(error = %error_msg, status = %status, "Internal error")
            }
        }

        let error_response = ErrorResponse {
            error: error_msg,
            details,
        };

        HttpResponse::build(status).json(error_response)
    }
}

impl From<anyhow::Error> for BankError {
    fn from(err: anyhow::Error) -> Self {
        match err.downcast_ref::<DomainError>() {
            Some(DomainError::InsufficientFunds) => BankError::InsufficientFunds,
            Some(DomainError::AccountNotFound) => {
                BankError::NotFound("Account not found".to_string())
            }
            Some(DomainError::InvalidAmount) => BankError::Validation("Invalid amount".to_string()),
            Some(DomainError::Validation(msg)) => BankError::Validation(msg.clone()),
            Some(DomainError::NotFound(msg)) => BankError::NotFound(msg.clone()),
            Some(DomainError::Unauthorized(msg)) => BankError::Unauthorized(msg.clone()),
            Some(DomainError::Internal(msg)) => BankError::Internal(msg.clone()),
            None => BankError::Database(err.to_string()),
        }
    }
}

// AuthenticatedUser extractor
impl FromRequest for AuthenticatedUser {
    type Error = BankError;
    type Future = Pin<Box<dyn std::future::Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(
        req: &actix_web::HttpRequest,
        _payload: &mut actix_web::dev::Payload,
    ) -> Self::Future {
        let user = req.extensions().get::<AuthenticatedUser>().cloned();
        Box::pin(async move {
            user.ok_or_else(|| BankError::Unauthorized("User not authenticated".to_string()))
        })
    }
}

// Handlers

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    timestamp: String,
}

#[instrument]
pub async fn health_check() -> HttpResponse {
    info!("Health check requested");
    let response = HealthResponse {
        status: "ok".to_string(),
        timestamp: Utc::now().to_rfc3339(),
    };
    HttpResponse::Ok().json(response)
}

#[instrument(skip(state), fields(account_id))]
pub async fn create_account(
    state: web::Data<AppState>,
    req: web::Json<CreateAccount>,
) -> Result<HttpResponse, BankError> {
    info!(name = %req.name, "Creating new account");
    let account = state
        .service
        .create_account(req.into_inner())
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to create account");
            e
        })?;
    tracing::Span::current().record("account_id", account.id);
    info!(
        account_id = account.id,
        balance = account.balance.inner(),
        "Account created successfully"
    );
    Ok(HttpResponse::Created().json(account))
}

#[instrument(skip(state), fields(account_id = %*path))]
pub async fn get_account(
    state: web::Data<AppState>,
    path: web::Path<u32>,
) -> Result<HttpResponse, BankError> {
    let account_id = path.into_inner();
    info!(account_id = account_id, "Getting account balance");
    let account = state.service.get_account(account_id).await.map_err(|e| {
        error!(account_id = account_id, error = %e, "Failed to get account");
        e
    })?;
    info!(
        account_id = account.id,
        balance = account.balance.inner(),
        "Account retrieved successfully"
    );
    Ok(HttpResponse::Ok().json(account))
}

#[instrument(skip(state), fields(account_id = %*path, amount))]
pub async fn deposit(
    state: web::Data<AppState>,
    path: web::Path<u32>,
    req: web::Json<Deposit>,
) -> Result<HttpResponse, BankError> {
    let account_id = path.into_inner();
    let amount = req.amount.inner();
    tracing::Span::current().record("amount", amount);
    info!(
        account_id = account_id,
        amount = amount,
        "Processing deposit"
    );
    let account = state
        .service
        .deposit(account_id, req.into_inner().amount)
        .await
        .map_err(|e| {
            error!(account_id = account_id, amount = amount, error = %e, "Failed to deposit");
            e
        })?;
    info!(
        account_id = account.id,
        balance = account.balance.inner(),
        "Deposit completed successfully"
    );
    Ok(HttpResponse::Ok().json(account))
}

#[instrument(skip(state), fields(account_id = %*path, amount))]
pub async fn withdraw(
    state: web::Data<AppState>,
    path: web::Path<u32>,
    req: web::Json<Withdraw>,
) -> Result<HttpResponse, BankError> {
    let account_id = path.into_inner();
    let amount = req.amount.inner();
    tracing::Span::current().record("amount", amount);
    info!(
        account_id = account_id,
        amount = amount,
        "Processing withdrawal"
    );
    let account = state
        .service
        .withdraw(account_id, req.into_inner().amount)
        .await
        .map_err(|e| {
            error!(account_id = account_id, amount = amount, error = %e, "Failed to withdraw");
            e
        })?;
    info!(
        account_id = account.id,
        balance = account.balance.inner(),
        "Withdrawal completed successfully"
    );
    Ok(HttpResponse::Ok().json(account))
}

#[instrument(skip(state), fields(from_account_id, to_account_id, amount))]
pub async fn transfer(
    state: web::Data<AppState>,
    req: web::Json<Transfer>,
) -> Result<HttpResponse, BankError> {
    let transfer_req = req.into_inner();
    let from_id = transfer_req.from_account_id;
    let to_id = transfer_req.to_account_id;
    let amount = transfer_req.amount.inner();
    tracing::Span::current()
        .record("from_account_id", from_id)
        .record("to_account_id", to_id)
        .record("amount", amount);
    info!(
        from_account_id = from_id,
        to_account_id = to_id,
        amount = amount,
        "Processing transfer"
    );
    state.service.transfer(transfer_req).await.map_err(|e| {
        error!(
            from_account_id = from_id,
            to_account_id = to_id,
            amount = amount,
            error = %e,
            "Failed to transfer"
        );
        e
    })?;
    info!(
        from_account_id = from_id,
        to_account_id = to_id,
        amount = amount,
        "Transfer completed successfully"
    );
    Ok(HttpResponse::Ok().finish())
}
