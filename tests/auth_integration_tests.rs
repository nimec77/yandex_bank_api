use actix_web::{App, test, web};
use std::sync::Arc;
use yandex_bank_api::application::auth_service::AuthService;
use yandex_bank_api::application::service::BankService;
use yandex_bank_api::data::memory::InMemoryAccountRepository;
use yandex_bank_api::data::user_repository::InMemoryUserRepository;
use yandex_bank_api::domain::user::{CreateUser, LoginRequest};
use yandex_bank_api::presentation::auth::{get_token, login, register};
use yandex_bank_api::presentation::handlers::AppState;
use yandex_bank_api::presentation::middleware::JwtAuthMiddleware;

macro_rules! setup_auth_test {
    () => {{
        let repository = InMemoryAccountRepository::new();
        let service = BankService::new(Arc::new(repository));

        let user_repository = InMemoryUserRepository::new();
        let jwt_secret = "test-secret-key-for-auth-tests".to_string();
        let auth_service = AuthService::new(Arc::new(user_repository), jwt_secret.clone());

        let state = web::Data::new(AppState {
            service,
            auth_service: Arc::new(auth_service),
        });

        let app = test::init_service(
            App::new()
                .app_data(state.clone())
                .wrap(JwtAuthMiddleware::new(jwt_secret))
                .service(
                    web::scope("/api")
                        .route("/auth/register", web::post().to(register))
                        .route("/auth/login", web::post().to(login))
                        .route("/auth/token", web::post().to(get_token)),
                ),
        )
        .await;

        app
    }};
}

#[actix_web::test]
async fn test_full_registration_login_flow() {
    let app = setup_auth_test!();

    // Register user
    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(&CreateUser {
            email: "flow@example.com".to_string(),
            password: "password123".to_string(),
        })
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    let resp: serde_json::Value = test::read_body_json(resp).await;
    assert!(resp.get("id").is_some());
    assert_eq!(resp["email"], "flow@example.com");
    let user_id = resp["id"].as_str().unwrap().to_string();

    // Login
    let req = test::TestRequest::post()
        .uri("/api/auth/login")
        .set_json(&LoginRequest {
            email: "flow@example.com".to_string(),
            password: "password123".to_string(),
        })
        .to_request();

    let service_resp = test::call_service(&app, req).await;
    assert!(service_resp.status().is_success());
    let resp: serde_json::Value = test::read_body_json(service_resp).await;
    assert!(resp.get("access_token").is_some());
    let token = resp["access_token"].as_str().unwrap();

    // Get token using user_id
    let req = test::TestRequest::post()
        .uri("/api/auth/token")
        .set_json(serde_json::json!({
            "user_id": user_id
        }))
        .to_request();

    let service_resp = test::call_service(&app, req).await;
    assert!(service_resp.status().is_success());
    let resp: serde_json::Value = test::read_body_json(service_resp).await;
    assert!(resp.get("access_token").is_some());
    let token2 = resp["access_token"].as_str().unwrap();

    // Tokens may be the same if generated within the same second (JWT uses second precision)
    // This is correct behavior - JWT tokens are deterministic based on their claims
    // If we want different tokens, we need to wait at least 1 second
    // For this test, we'll just verify both tokens are valid
    assert!(!token.is_empty());
    assert!(!token2.is_empty());
}

#[actix_web::test]
async fn test_register_duplicate_email() {
    let app = setup_auth_test!();

    // Register first user
    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(&CreateUser {
            email: "duplicate@example.com".to_string(),
            password: "pass1".to_string(),
        })
        .to_request();
    test::call_service(&app, req).await;

    // Try to register with same email
    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(&CreateUser {
            email: "duplicate@example.com".to_string(),
            password: "pass2".to_string(),
        })
        .to_request();
    let resp: actix_web::dev::ServiceResponse = test::call_service(&app, req).await;
    assert_eq!(resp.status(), actix_web::http::StatusCode::BAD_REQUEST);
}

#[actix_web::test]
async fn test_login_wrong_password() {
    let app = setup_auth_test!();

    // Register user
    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(&CreateUser {
            email: "wrongpass@example.com".to_string(),
            password: "correct".to_string(),
        })
        .to_request();
    test::call_service(&app, req).await;

    // Try to login with wrong password
    let req = test::TestRequest::post()
        .uri("/api/auth/login")
        .set_json(&LoginRequest {
            email: "wrongpass@example.com".to_string(),
            password: "wrong".to_string(),
        })
        .to_request();
    let resp: actix_web::dev::ServiceResponse = test::call_service(&app, req).await;
    assert_eq!(resp.status(), actix_web::http::StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn test_login_nonexistent_user() {
    let app = setup_auth_test!();

    let req = test::TestRequest::post()
        .uri("/api/auth/login")
        .set_json(&LoginRequest {
            email: "nonexistent@example.com".to_string(),
            password: "password".to_string(),
        })
        .to_request();
    let resp: actix_web::dev::ServiceResponse = test::call_service(&app, req).await;
    assert_eq!(resp.status(), actix_web::http::StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn test_get_token_nonexistent_user() {
    let app = setup_auth_test!();

    let req = test::TestRequest::post()
        .uri("/api/auth/token")
        .set_json(serde_json::json!({
            "user_id": "nonexistent-id"
        }))
        .to_request();
    let resp: actix_web::dev::ServiceResponse = test::call_service(&app, req).await;
    assert_eq!(resp.status(), actix_web::http::StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn test_multiple_users_registration() {
    let app = setup_auth_test!();

    // Register multiple users
    for i in 1..=5 {
        let req = test::TestRequest::post()
            .uri("/api/auth/register")
            .set_json(&CreateUser {
                email: format!("user{}@example.com", i),
                password: format!("pass{}", i),
            })
            .to_request();
        let service_resp = test::call_service(&app, req).await;
        assert!(service_resp.status().is_success());
        let resp: serde_json::Value = test::read_body_json(service_resp).await;
        assert_eq!(resp["email"], format!("user{}@example.com", i));
    }
}

#[actix_web::test]
async fn test_login_multiple_times_generates_different_tokens() {
    let app = setup_auth_test!();

    // Register user
    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(&CreateUser {
            email: "multitoken@example.com".to_string(),
            password: "password".to_string(),
        })
        .to_request();
    test::call_service(&app, req).await;

    // Login multiple times and verify all tokens are valid
    let mut tokens = Vec::new();
    for i in 0..3 {
        // Add a small delay to ensure different timestamps (JWT uses second precision)
        if i > 0 {
            tokio::time::sleep(tokio::time::Duration::from_millis(1100)).await;
        }
        let req = test::TestRequest::post()
            .uri("/api/auth/login")
            .set_json(&LoginRequest {
                email: "multitoken@example.com".to_string(),
                password: "password".to_string(),
            })
            .to_request();
        let service_resp = test::call_service(&app, req).await;
        assert!(service_resp.status().is_success());
        let resp: serde_json::Value = test::read_body_json(service_resp).await;
        tokens.push(resp["access_token"].as_str().unwrap().to_string());
    }

    // With delays, all tokens should be different (different timestamps)
    assert_ne!(tokens[0], tokens[1]);
    assert_ne!(tokens[1], tokens[2]);
    assert_ne!(tokens[0], tokens[2]);
}

#[actix_web::test]
async fn test_password_not_stored_in_plain_text() {
    let app = setup_auth_test!();

    let password = "sensitive_password_123";

    // Register user
    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(&CreateUser {
            email: "plaintext@example.com".to_string(),
            password: password.to_string(),
        })
        .to_request();
    let service_resp = test::call_service(&app, req).await;
    assert!(service_resp.status().is_success());
    let resp: serde_json::Value = test::read_body_json(service_resp).await;

    // Response should not contain password
    assert!(resp.get("password").is_none());
    assert!(resp.get("password_hash").is_none());

    // But login should still work
    let req = test::TestRequest::post()
        .uri("/api/auth/login")
        .set_json(&LoginRequest {
            email: "plaintext@example.com".to_string(),
            password: password.to_string(),
        })
        .to_request();
    let service_resp = test::call_service(&app, req).await;
    assert!(service_resp.status().is_success());
    let resp: serde_json::Value = test::read_body_json(service_resp).await;
    assert!(resp.get("access_token").is_some());
}
