use crate::application::service::BankService;
use crate::data::memory::InMemoryAccountRepository;
use crate::domain::models::{CreateAccount, Deposit, DomainError, Transfer, Withdraw};
use actix_web::{HttpResponse, ResponseError, web};
use thiserror::Error;

// AppState holding the service
pub struct AppState {
    pub service: BankService<InMemoryAccountRepository>,
}

// Map DomainError to HTTP Response
#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Bad Request: {0}")]
    BadRequest(String),
    #[error("Not Found: {0}")]
    NotFound(String),
    #[error("Internal Server Error: {0}")]
    InternalServerError(String),
}

impl ResponseError for ApiError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match self {
            ApiError::BadRequest(_) => actix_web::http::StatusCode::BAD_REQUEST,
            ApiError::NotFound(_) => actix_web::http::StatusCode::NOT_FOUND,
            ApiError::InternalServerError(_) => actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        match err.downcast_ref::<DomainError>() {
            Some(DomainError::InsufficientFunds) => {
                ApiError::BadRequest("Insufficient funds".to_string())
            }
            Some(DomainError::AccountNotFound) => {
                ApiError::NotFound("Account not found".to_string())
            }
            Some(DomainError::InvalidAmount) => ApiError::BadRequest("Invalid amount".to_string()),
            None => ApiError::InternalServerError(err.to_string()),
        }
    }
}

// Handlers

pub async fn create_account(
    state: web::Data<AppState>,
    req: web::Json<CreateAccount>,
) -> Result<HttpResponse, ApiError> {
    let account = state.service.create_account(req.into_inner()).await?;
    Ok(HttpResponse::Created().json(account))
}

pub async fn get_account(
    state: web::Data<AppState>,
    path: web::Path<u32>,
) -> Result<HttpResponse, ApiError> {
    let account = state.service.get_account(path.into_inner()).await?;
    Ok(HttpResponse::Ok().json(account))
}

pub async fn deposit(
    state: web::Data<AppState>,
    path: web::Path<u32>,
    req: web::Json<Deposit>,
) -> Result<HttpResponse, ApiError> {
    let account = state
        .service
        .deposit(path.into_inner(), req.into_inner().amount)
        .await?;
    Ok(HttpResponse::Ok().json(account))
}

pub async fn withdraw(
    state: web::Data<AppState>,
    path: web::Path<u32>,
    req: web::Json<Withdraw>,
) -> Result<HttpResponse, ApiError> {
    let account = state
        .service
        .withdraw(path.into_inner(), req.into_inner().amount)
        .await?;
    Ok(HttpResponse::Ok().json(account))
}

pub async fn transfer(
    state: web::Data<AppState>,
    req: web::Json<Transfer>,
) -> Result<HttpResponse, ApiError> {
    state.service.transfer(req.into_inner()).await?;
    Ok(HttpResponse::Ok().finish())
}
