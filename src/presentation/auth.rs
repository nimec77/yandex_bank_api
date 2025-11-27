use crate::domain::user::{CreateUser, LoginRequest};
use crate::presentation::handlers::{AppState, BankError};
use actix_web::{HttpResponse, web};
use serde::{Deserialize, Serialize};
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

#[instrument(skip(state))]
pub async fn register(
    state: web::Data<AppState>,
    req: web::Json<CreateUser>,
) -> Result<HttpResponse, BankError> {
    info!(email = %req.email, "Registration request received");

    let user = state
        .auth_service
        .register_user(req.into_inner())
        .await
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

#[instrument(skip(state))]
pub async fn login(
    state: web::Data<AppState>,
    req: web::Json<LoginRequest>,
) -> Result<HttpResponse, BankError> {
    info!(email = %req.email, "Login request received");

    let token = state
        .auth_service
        .login(req.into_inner())
        .await
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

#[instrument(skip(state))]
pub async fn get_token(
    state: web::Data<AppState>,
    req: web::Json<GetTokenRequest>,
) -> Result<HttpResponse, BankError> {
    info!(user_id = %req.user_id, "Token request received");

    let token = state
        .auth_service
        .get_token(&req.user_id)
        .await
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
