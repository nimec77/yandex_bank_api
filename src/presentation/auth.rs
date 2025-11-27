use crate::application::auth_service::AuthService;
use crate::data::user_repository::InMemoryUserRepository;
use crate::domain::user::{CreateUser, LoginRequest};
use crate::presentation::handlers::BankError;
use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info, instrument};

#[derive(Serialize)]
pub struct RegisterResponse {
    pub id: String,
    pub email: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub access_token: String,
}

#[derive(Serialize)]
pub struct TokenResponse {
    pub access_token: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetTokenRequest {
    pub user_id: String,
}

#[instrument(skip(auth_service))]
pub async fn register(
    auth_service: web::Data<Arc<AuthService<InMemoryUserRepository>>>,
    req: web::Json<CreateUser>,
) -> Result<HttpResponse, BankError> {
    info!(email = %req.email, "Registration request received");
    
    let user = auth_service.register_user(req.into_inner()).await
        .map_err(|e| {
            error!(error = %e, "Failed to register user");
            BankError::from(e)
        })?;

    let response = RegisterResponse {
        id: user.id,
        email: user.email,
    };

    info!(user_id = %response.id, email = %response.email, "User registered successfully");
    Ok(HttpResponse::Created().json(response))
}

#[instrument(skip(auth_service))]
pub async fn login(
    auth_service: web::Data<Arc<AuthService<InMemoryUserRepository>>>,
    req: web::Json<LoginRequest>,
) -> Result<HttpResponse, BankError> {
    info!(email = %req.email, "Login request received");
    
    let token = auth_service.login(req.into_inner()).await
        .map_err(|e| {
            error!(error = %e, "Failed to login");
            BankError::from(e)
        })?;

    let response = LoginResponse {
        access_token: token,
    };

    info!("Login successful");
    Ok(HttpResponse::Ok().json(response))
}

#[instrument(skip(auth_service))]
pub async fn get_token(
    auth_service: web::Data<Arc<AuthService<InMemoryUserRepository>>>,
    req: web::Json<GetTokenRequest>,
) -> Result<HttpResponse, BankError> {
    info!(user_id = %req.user_id, "Token request received");
    
    let token = auth_service.get_token(&req.user_id).await
        .map_err(|e| {
            error!(error = %e, "Failed to generate token");
            BankError::from(e)
        })?;

    let response = TokenResponse {
        access_token: token,
    };

    info!("Token generated successfully");
    Ok(HttpResponse::Ok().json(response))
}

