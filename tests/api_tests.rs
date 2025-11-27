use actix_web::{App, test, web};
use std::sync::Arc;
use yandex_bank_api::application::auth_service::AuthService;
use yandex_bank_api::application::service::BankService;
use yandex_bank_api::data::memory::InMemoryAccountRepository;
use yandex_bank_api::data::user_repository::InMemoryUserRepository;
use yandex_bank_api::domain::models::{
    Account, Amount, CreateAccount, Deposit, Transfer, Withdraw,
};
use yandex_bank_api::domain::user::{CreateUser, LoginRequest};
use yandex_bank_api::presentation::handlers::{
    AppState, create_account, deposit, get_account, transfer, withdraw,
};
use yandex_bank_api::presentation::middleware::JwtAuthMiddleware;

macro_rules! setup_test {
    () => {{
        let repository = InMemoryAccountRepository::new();
        let service = BankService::new(Arc::new(repository));
        
        let user_repository = InMemoryUserRepository::new();
        let jwt_secret = "test-secret-key-for-testing-only".to_string();
        let auth_service = AuthService::new(Arc::new(user_repository), jwt_secret.clone());
        
        // Register a test user
        let create_user = CreateUser {
            email: "test@example.com".to_string(),
            password: "test123".to_string(),
        };
        let _user = auth_service.register_user(create_user).await.unwrap();
        
        // Login to get token
        let login_req = LoginRequest {
            email: "test@example.com".to_string(),
            password: "test123".to_string(),
        };
        let token = auth_service.login(login_req).await.unwrap();
        
        let state = web::Data::new(AppState {
            service,
            auth_service: Arc::new(auth_service),
        });

        let app = test::init_service(
            App::new()
                .app_data(state.clone())
                .wrap(JwtAuthMiddleware::new(jwt_secret))
                .route("/accounts", web::post().to(create_account))
                .route("/accounts/{id}", web::get().to(get_account))
                .route("/accounts/{id}/deposit", web::post().to(deposit))
                .route("/accounts/{id}/withdraw", web::post().to(withdraw))
                .route("/transfers", web::post().to(transfer)),
        )
        .await;

        (app, token)
    }};
}

#[actix_web::test]
async fn test_create_account() {
    let (app, token) = setup_test!();

    let req = test::TestRequest::post()
        .uri("/accounts")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(&CreateAccount {
            name: "Alice".to_string(),
        })
        .to_request();

    let resp: Account = test::call_and_read_body_json(&app, req).await;
    assert_eq!(resp.name, "Alice");
    assert_eq!(resp.balance.inner(), 0);
}

#[actix_web::test]
async fn test_deposit_and_withdraw() {
    let (app, token) = setup_test!();

    // Create account
    let req = test::TestRequest::post()
        .uri("/accounts")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(&CreateAccount {
            name: "Bob".to_string(),
        })
        .to_request();
    let account: Account = test::call_and_read_body_json(&app, req).await;

    // Deposit
    let req = test::TestRequest::post()
        .uri(&format!("/accounts/{}/deposit", account.id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(&Deposit {
            amount: Amount::new(100),
        })
        .to_request();
    let updated_account: Account = test::call_and_read_body_json(&app, req).await;
    assert_eq!(updated_account.balance.inner(), 100);

    // Withdraw
    let req = test::TestRequest::post()
        .uri(&format!("/accounts/{}/withdraw", account.id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(&Withdraw {
            amount: Amount::new(50),
        })
        .to_request();
    let final_account: Account = test::call_and_read_body_json(&app, req).await;
    assert_eq!(final_account.balance.inner(), 50);
}

#[actix_web::test]
async fn test_transfer() {
    let (app, token) = setup_test!();

    // Create Alice
    let req = test::TestRequest::post()
        .uri("/accounts")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(&CreateAccount {
            name: "Alice".to_string(),
        })
        .to_request();
    let alice: Account = test::call_and_read_body_json(&app, req).await;

    // Create Bob
    let req = test::TestRequest::post()
        .uri("/accounts")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(&CreateAccount {
            name: "Bob".to_string(),
        })
        .to_request();
    let bob: Account = test::call_and_read_body_json(&app, req).await;

    // Deposit to Alice
    let req = test::TestRequest::post()
        .uri(&format!("/accounts/{}/deposit", alice.id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(&Deposit {
            amount: Amount::new(100),
        })
        .to_request();
    test::call_service(&app, req).await;

    // Transfer
    let req = test::TestRequest::post()
        .uri("/transfers")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(&Transfer {
            from_account_id: alice.id,
            to_account_id: bob.id,
            amount: Amount::new(50),
        })
        .to_request();
    let resp: actix_web::dev::ServiceResponse = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    // Check Alice balance
    let req = test::TestRequest::get()
        .uri(&format!("/accounts/{}", alice.id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let alice_final: Account = test::call_and_read_body_json(&app, req).await;
    assert_eq!(alice_final.balance.inner(), 50);

    // Check Bob balance
    let req = test::TestRequest::get()
        .uri(&format!("/accounts/{}", bob.id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let bob_final: Account = test::call_and_read_body_json(&app, req).await;
    assert_eq!(bob_final.balance.inner(), 50);
}

#[actix_web::test]
async fn test_unauthorized_access() {
    let (app, _token) = setup_test!();

    // Try to access protected route without token
    let req = test::TestRequest::post()
        .uri("/accounts")
        .set_json(&CreateAccount {
            name: "Alice".to_string(),
        })
        .to_request();

    let resp = test::try_call_service(&app, req).await;
    match resp {
        Ok(service_resp) => {
            assert_eq!(service_resp.status(), actix_web::http::StatusCode::UNAUTHORIZED);
        }
        Err(err) => {
            // The error should be an Unauthorized error
            assert!(err.to_string().contains("missing bearer") || err.to_string().contains("Unauthorized"));
        }
    }
}
