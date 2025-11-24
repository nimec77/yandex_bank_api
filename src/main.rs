use actix_web::{App, HttpServer, web};
use std::sync::Arc;
use tracing::{info, instrument};
use yandex_bank_api::application::service::BankService;
use yandex_bank_api::data::memory::InMemoryAccountRepository;
use yandex_bank_api::infrastructure::logging::init_logging;
use yandex_bank_api::presentation::handlers::{
    AppState, create_account, deposit, get_account, health_check, transfer, withdraw,
};
use yandex_bank_api::presentation::middleware::{RequestIdMiddleware, TimingMiddleware};

#[tokio::main]
#[instrument]
async fn main() -> std::io::Result<()> {
    // Initialize logging
    info!("Initializing logging subsystem");
    init_logging();
    info!("Logging initialized successfully");

    info!("Creating in-memory account repository");
    let repository = InMemoryAccountRepository::new();
    info!("Repository created");

    info!("Creating bank service");
    let service = BankService::new(Arc::new(repository));
    info!("Bank service created");

    info!("Initializing application state");
    let state = web::Data::new(AppState { service });
    info!("Application state initialized");

    info!("Configuring HTTP server");
    let server = HttpServer::new(move || {
        tracing::trace!("Creating new application instance");
        App::new()
            .app_data(state.clone())
            .wrap(TimingMiddleware)
            .wrap(RequestIdMiddleware)
            .service(
                web::scope("/api")
                    .route("/health", web::get().to(health_check))
                    .route("/accounts", web::post().to(create_account))
                    .route("/accounts/{id}", web::get().to(get_account))
                    .route("/accounts/{id}/deposit", web::post().to(deposit))
                    .route("/accounts/{id}/withdraw", web::post().to(withdraw))
                    .route("/transfers", web::post().to(transfer)),
            )
    });

    let bind_addr = "127.0.0.1:8080";
    info!(address = %bind_addr, "Binding server to address");
    let server = server.bind(("127.0.0.1", 8080))?;
    info!(address = %bind_addr, "Server bound successfully");

    info!(
        address = %bind_addr,
        routes = %"GET /api/health, POST /api/accounts, GET /api/accounts/{id}, POST /api/accounts/{id}/deposit, POST /api/accounts/{id}/withdraw, POST /api/transfers",
        "Starting HTTP server"
    );
    server.run().await
}
