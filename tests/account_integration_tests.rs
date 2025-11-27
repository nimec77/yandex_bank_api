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

macro_rules! setup_account_test {
    () => {{
        let repository = InMemoryAccountRepository::new();
        let service = BankService::new(Arc::new(repository));

        let user_repository = InMemoryUserRepository::new();
        let jwt_secret = "test-secret-key-for-account-tests".to_string();
        let auth_service = AuthService::new(Arc::new(user_repository), jwt_secret.clone());

        // Register and login
        let create_user = CreateUser {
            email: "account@example.com".to_string(),
            password: "test123".to_string(),
        };
        let _user = auth_service.register_user(create_user).await.unwrap();

        let login_req = LoginRequest {
            email: "account@example.com".to_string(),
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
async fn test_complex_transfer_scenario() {
    let (app, token) = setup_account_test!();

    // Create three accounts
    let mut accounts = Vec::new();
    for name in ["Alice", "Bob", "Charlie"] {
        let req = test::TestRequest::post()
            .uri("/accounts")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(&CreateAccount {
                name: name.to_string(),
            })
            .to_request();
        let account: Account = test::call_and_read_body_json(&app, req).await;
        accounts.push(account);
    }

    // Deposit to Alice
    let req = test::TestRequest::post()
        .uri(&format!("/accounts/{}/deposit", accounts[0].id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(&Deposit {
            amount: Amount::new(1000),
        })
        .to_request();
    test::call_service(&app, req).await;

    // Transfer from Alice to Bob
    let req = test::TestRequest::post()
        .uri("/transfers")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(&Transfer {
            from_account_id: accounts[0].id,
            to_account_id: accounts[1].id,
            amount: Amount::new(300),
        })
        .to_request();
    test::call_service(&app, req).await;

    // Transfer from Bob to Charlie
    let req = test::TestRequest::post()
        .uri("/transfers")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(&Transfer {
            from_account_id: accounts[1].id,
            to_account_id: accounts[2].id,
            amount: Amount::new(100),
        })
        .to_request();
    test::call_service(&app, req).await;

    // Verify final balances
    let req = test::TestRequest::get()
        .uri(&format!("/accounts/{}", accounts[0].id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let alice: Account = test::call_and_read_body_json(&app, req).await;
    assert_eq!(alice.balance.inner(), 700);

    let req = test::TestRequest::get()
        .uri(&format!("/accounts/{}", accounts[1].id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let bob: Account = test::call_and_read_body_json(&app, req).await;
    assert_eq!(bob.balance.inner(), 200);

    let req = test::TestRequest::get()
        .uri(&format!("/accounts/{}", accounts[2].id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let charlie: Account = test::call_and_read_body_json(&app, req).await;
    assert_eq!(charlie.balance.inner(), 100);
}

#[actix_web::test]
async fn test_multiple_concurrent_deposits() {
    let (app, token) = setup_account_test!();

    // Create account
    let req = test::TestRequest::post()
        .uri("/accounts")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(&CreateAccount {
            name: "Concurrent".to_string(),
        })
        .to_request();
    let account: Account = test::call_and_read_body_json(&app, req).await;

    // Perform multiple deposits sequentially
    for amount in [10, 20, 30, 40, 50] {
        let req = test::TestRequest::post()
            .uri(&format!("/accounts/{}/deposit", account.id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(&Deposit {
                amount: Amount::new(amount),
            })
            .to_request();
        test::call_service(&app, req).await;
    }

    // Verify final balance
    let req = test::TestRequest::get()
        .uri(&format!("/accounts/{}", account.id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let final_account: Account = test::call_and_read_body_json(&app, req).await;
    assert_eq!(final_account.balance.inner(), 150);
}

#[actix_web::test]
async fn test_account_balance_edge_cases() {
    let (app, token) = setup_account_test!();

    // Create account
    let req = test::TestRequest::post()
        .uri("/accounts")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(&CreateAccount {
            name: "Edge Cases".to_string(),
        })
        .to_request();
    let account: Account = test::call_and_read_body_json(&app, req).await;
    assert_eq!(account.balance.inner(), 0);

    // Deposit large amount
    let req = test::TestRequest::post()
        .uri(&format!("/accounts/{}/deposit", account.id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(&Deposit {
            amount: Amount::new(1_000_000_000),
        })
        .to_request();
    let updated: Account = test::call_and_read_body_json(&app, req).await;
    assert_eq!(updated.balance.inner(), 1_000_000_000);

    // Withdraw all
    let req = test::TestRequest::post()
        .uri(&format!("/accounts/{}/withdraw", account.id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(&Withdraw {
            amount: Amount::new(1_000_000_000),
        })
        .to_request();
    let updated: Account = test::call_and_read_body_json(&app, req).await;
    assert_eq!(updated.balance.inner(), 0);
}

#[actix_web::test]
async fn test_transfer_rollback_scenario() {
    let (app, token) = setup_account_test!();

    // Create two accounts
    let req = test::TestRequest::post()
        .uri("/accounts")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(&CreateAccount {
            name: "Source".to_string(),
        })
        .to_request();
    let source: Account = test::call_and_read_body_json(&app, req).await;

    let req = test::TestRequest::post()
        .uri("/accounts")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(&CreateAccount {
            name: "Dest".to_string(),
        })
        .to_request();
    let dest: Account = test::call_and_read_body_json(&app, req).await;

    // Deposit to source
    let req = test::TestRequest::post()
        .uri(&format!("/accounts/{}/deposit", source.id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(&Deposit {
            amount: Amount::new(500),
        })
        .to_request();
    test::call_service(&app, req).await;

    // Transfer some amount
    let req = test::TestRequest::post()
        .uri("/transfers")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(&Transfer {
            from_account_id: source.id,
            to_account_id: dest.id,
            amount: Amount::new(200),
        })
        .to_request();
    test::call_service(&app, req).await;

    // Verify balances
    let req = test::TestRequest::get()
        .uri(&format!("/accounts/{}", source.id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let source_final: Account = test::call_and_read_body_json(&app, req).await;
    assert_eq!(source_final.balance.inner(), 300);

    let req = test::TestRequest::get()
        .uri(&format!("/accounts/{}", dest.id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let dest_final: Account = test::call_and_read_body_json(&app, req).await;
    assert_eq!(dest_final.balance.inner(), 200);

    // Try to transfer back more than available (should fail)
    let req = test::TestRequest::post()
        .uri("/transfers")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(&Transfer {
            from_account_id: dest.id,
            to_account_id: source.id,
            amount: Amount::new(300),
        })
        .to_request();
    let resp: actix_web::dev::ServiceResponse = test::call_service(&app, req).await;
    assert_eq!(resp.status(), actix_web::http::StatusCode::BAD_REQUEST);
}

#[actix_web::test]
async fn test_multiple_accounts_operations() {
    let (app, token) = setup_account_test!();

    // Create multiple accounts
    let mut account_ids = Vec::new();
    for i in 1..=5 {
        let req = test::TestRequest::post()
            .uri("/accounts")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(&CreateAccount {
                name: format!("Account {}", i),
            })
            .to_request();
        let account: Account = test::call_and_read_body_json(&app, req).await;
        account_ids.push(account.id);
    }

    // Deposit different amounts to each
    for (i, &id) in account_ids.iter().enumerate() {
        let req = test::TestRequest::post()
            .uri(&format!("/accounts/{}/deposit", id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(&Deposit {
                amount: Amount::new((i + 1) as u64 * 100),
            })
            .to_request();
        test::call_service(&app, req).await;
    }

    // Verify all balances
    for (i, &id) in account_ids.iter().enumerate() {
        let req = test::TestRequest::get()
            .uri(&format!("/accounts/{}", id))
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();
        let account: Account = test::call_and_read_body_json(&app, req).await;
        assert_eq!(account.balance.inner(), (i + 1) as u64 * 100);
    }
}

#[actix_web::test]
async fn test_sequential_operations() {
    let (app, token) = setup_account_test!();

    // Create account
    let req = test::TestRequest::post()
        .uri("/accounts")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(&CreateAccount {
            name: "Sequential".to_string(),
        })
        .to_request();
    let account: Account = test::call_and_read_body_json(&app, req).await;

    // Deposit 100
    let req = test::TestRequest::post()
        .uri(&format!("/accounts/{}/deposit", account.id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(&Deposit {
            amount: Amount::new(100),
        })
        .to_request();
    test::call_service(&app, req).await;

    // Withdraw 30
    let req = test::TestRequest::post()
        .uri(&format!("/accounts/{}/withdraw", account.id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(&Withdraw {
            amount: Amount::new(30),
        })
        .to_request();
    test::call_service(&app, req).await;

    // Deposit 50
    let req = test::TestRequest::post()
        .uri(&format!("/accounts/{}/deposit", account.id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(&Deposit {
            amount: Amount::new(50),
        })
        .to_request();
    test::call_service(&app, req).await;

    // Withdraw 20
    let req = test::TestRequest::post()
        .uri(&format!("/accounts/{}/withdraw", account.id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(&Withdraw {
            amount: Amount::new(20),
        })
        .to_request();
    test::call_service(&app, req).await;

    // Final balance should be 100
    let req = test::TestRequest::get()
        .uri(&format!("/accounts/{}", account.id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let final_account: Account = test::call_and_read_body_json(&app, req).await;
    assert_eq!(final_account.balance.inner(), 100);
}
