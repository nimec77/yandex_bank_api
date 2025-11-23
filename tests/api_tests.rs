use actix_web::{App, test, web};
use std::sync::Arc;
use yandex_bank_api::application::service::BankService;
use yandex_bank_api::data::memory::InMemoryAccountRepository;
use yandex_bank_api::domain::models::{
    Account, Amount, CreateAccount, Deposit, Transfer, Withdraw,
};
use yandex_bank_api::presentation::handlers::{
    AppState, create_account, deposit, get_account, transfer, withdraw,
};

#[actix_web::test]
async fn test_create_account() {
    let repository = InMemoryAccountRepository::new();
    let service = BankService::new(Arc::new(repository));
    let state = web::Data::new(AppState { service });

    let app = test::init_service(
        App::new()
            .app_data(state.clone())
            .route("/accounts", web::post().to(create_account)),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/accounts")
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
    let repository = InMemoryAccountRepository::new();
    let service = BankService::new(Arc::new(repository));
    let state = web::Data::new(AppState { service });

    let app = test::init_service(
        App::new()
            .app_data(state.clone())
            .route("/accounts", web::post().to(create_account))
            .route("/accounts/{id}/deposit", web::post().to(deposit))
            .route("/accounts/{id}/withdraw", web::post().to(withdraw)),
    )
    .await;

    // Create account
    let req = test::TestRequest::post()
        .uri("/accounts")
        .set_json(&CreateAccount {
            name: "Bob".to_string(),
        })
        .to_request();
    let account: Account = test::call_and_read_body_json(&app, req).await;

    // Deposit
    let req = test::TestRequest::post()
        .uri(&format!("/accounts/{}/deposit", account.id))
        .set_json(&Deposit {
            amount: Amount::new(100),
        })
        .to_request();
    let updated_account: Account = test::call_and_read_body_json(&app, req).await;
    assert_eq!(updated_account.balance.inner(), 100);

    // Withdraw
    let req = test::TestRequest::post()
        .uri(&format!("/accounts/{}/withdraw", account.id))
        .set_json(&Withdraw {
            amount: Amount::new(50),
        })
        .to_request();
    let final_account: Account = test::call_and_read_body_json(&app, req).await;
    assert_eq!(final_account.balance.inner(), 50);
}

#[actix_web::test]
async fn test_transfer() {
    let repository = InMemoryAccountRepository::new();
    let service = BankService::new(Arc::new(repository));
    let state = web::Data::new(AppState { service });

    let app = test::init_service(
        App::new()
            .app_data(state.clone())
            .route("/accounts", web::post().to(create_account))
            .route("/accounts/{id}/deposit", web::post().to(deposit))
            .route("/transfer", web::post().to(transfer))
            .route("/accounts/{id}", web::get().to(get_account)),
    )
    .await;

    // Create Alice
    let req = test::TestRequest::post()
        .uri("/accounts")
        .set_json(&CreateAccount {
            name: "Alice".to_string(),
        })
        .to_request();
    let alice: Account = test::call_and_read_body_json(&app, req).await;

    // Create Bob
    let req = test::TestRequest::post()
        .uri("/accounts")
        .set_json(&CreateAccount {
            name: "Bob".to_string(),
        })
        .to_request();
    let bob: Account = test::call_and_read_body_json(&app, req).await;

    // Deposit to Alice
    let req = test::TestRequest::post()
        .uri(&format!("/accounts/{}/deposit", alice.id))
        .set_json(&Deposit {
            amount: Amount::new(100),
        })
        .to_request();
    test::call_service(&app, req).await;

    // Transfer
    let req = test::TestRequest::post()
        .uri("/transfer")
        .set_json(&Transfer {
            from_account_id: alice.id,
            to_account_id: bob.id,
            amount: Amount::new(50),
        })
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    // Check Alice balance
    let req = test::TestRequest::get()
        .uri(&format!("/accounts/{}", alice.id))
        .to_request();
    let alice_final: Account = test::call_and_read_body_json(&app, req).await;
    assert_eq!(alice_final.balance.inner(), 50);

    // Check Bob balance
    let req = test::TestRequest::get()
        .uri(&format!("/accounts/{}", bob.id))
        .to_request();
    let bob_final: Account = test::call_and_read_body_json(&app, req).await;
    assert_eq!(bob_final.balance.inner(), 50);
}
