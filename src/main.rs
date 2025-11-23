use actix_web::{App, HttpServer, web};
use std::sync::Arc;
use yandex_bank_api::application::service::BankService;
use yandex_bank_api::data::memory::InMemoryAccountRepository;
use yandex_bank_api::presentation::handlers::{
    AppState, create_account, deposit, get_account, transfer, withdraw,
};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let repository = InMemoryAccountRepository::new();
    let service = BankService::new(Arc::new(repository));
    let state = web::Data::new(AppState { service });

    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .route("/accounts", web::post().to(create_account))
            .route("/accounts/{id}", web::get().to(get_account))
            .route("/accounts/{id}/deposit", web::post().to(deposit))
            .route("/accounts/{id}/withdraw", web::post().to(withdraw))
            .route("/transfer", web::post().to(transfer))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
