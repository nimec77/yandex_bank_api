use actix_cors::Cors;
use actix_web::{App, HttpServer, middleware::DefaultHeaders, web};
use std::sync::Arc;
use tracing::{info, instrument};
use yandex_bank_api::application::auth_service::AuthService;
use yandex_bank_api::application::service::BankService;
use yandex_bank_api::data::memory::InMemoryAccountRepository;
use yandex_bank_api::data::user_repository::InMemoryUserRepository;
use yandex_bank_api::infrastructure::logging::init_logging;
use yandex_bank_api::presentation::auth::{get_token, login, register};
use yandex_bank_api::presentation::handlers::{
    AppState, create_account, deposit, get_account, health_check, transfer, withdraw,
};
use yandex_bank_api::presentation::middleware::{
    JwtAuthMiddleware, RequestIdMiddleware, TimingMiddleware,
};

#[tokio::main]
#[instrument]
async fn main() -> std::io::Result<()> {
    // Load environment variables
    dotenv::dotenv().ok();

    // Initialize logging
    info!("Initializing logging subsystem");
    init_logging();
    info!("Logging initialized successfully");

    // Read environment variables
    let jwt_secret =
        std::env::var("JWT_SECRET").expect("JWT_SECRET must be set in environment variables");
    let allowed_origins =
        std::env::var("ALLOWED_ORIGINS").unwrap_or_else(|_| "http://localhost:3000".to_string());
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()
        .expect("PORT must be a valid number");

    info!("Creating in-memory account repository");
    let repository = InMemoryAccountRepository::new();
    info!("Repository created");

    info!("Creating in-memory user repository");
    let user_repository = InMemoryUserRepository::new();
    info!("User repository created");

    info!("Creating bank service");
    let service = BankService::new(Arc::new(repository));
    info!("Bank service created");

    info!("Creating auth service");
    let auth_service = AuthService::new(Arc::new(user_repository), jwt_secret.clone());
    info!("Auth service created");

    info!("Initializing application state");
    let state = web::Data::new(AppState {
        service,
        auth_service: Arc::new(auth_service),
    });
    info!("Application state initialized");

    // Parse allowed origins
    let origins: Vec<String> = allowed_origins
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();
    info!(origins = ?origins, "Configured CORS origins");

    info!("Configuring HTTP server");
    let origins_clone = origins.clone();
    let server = HttpServer::new(move || {
        tracing::trace!("Creating new application instance");

        // Configure CORS
        let mut cors = Cors::default()
            .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
            .allowed_headers(vec![
                actix_web::http::header::CONTENT_TYPE,
                actix_web::http::header::AUTHORIZATION,
            ])
            .max_age(3600)
            .expose_headers(vec![
                actix_web::http::header::HeaderName::from_static("x-total-count"),
                actix_web::http::header::HeaderName::from_static("x-request-id"),
            ]);

        // Set allowed origins
        for origin in &origins_clone {
            cors = cors.allowed_origin(origin.as_str());
        }

        App::new()
            .app_data(state.clone())
            // Middleware order: CORS → Security Headers → JWT → Timing → RequestId
            .wrap(cors)
            .wrap(
                DefaultHeaders::new()
                    .add(("X-Content-Type-Options", "nosniff"))
                    .add(("Referrer-Policy", "no-referrer"))
                    .add(("Permissions-Policy", "geolocation=()"))
                    .add(("Cross-Origin-Opener-Policy", "same-origin")),
            )
            .wrap(JwtAuthMiddleware::new(jwt_secret.clone()))
            .wrap(TimingMiddleware)
            .wrap(RequestIdMiddleware)
            .service(
                web::scope("/api")
                    // Public routes
                    .route("/health", web::get().to(health_check))
                    .route("/auth/register", web::post().to(register))
                    .route("/auth/login", web::post().to(login))
                    .route("/auth/token", web::post().to(get_token))
                    // Protected routes (require JWT)
                    .route("/accounts", web::post().to(create_account))
                    .route("/accounts/{id}", web::get().to(get_account))
                    .route("/accounts/{id}/deposit", web::post().to(deposit))
                    .route("/accounts/{id}/withdraw", web::post().to(withdraw))
                    .route("/transfers", web::post().to(transfer)),
            )
    });

    let bind_addr = format!("127.0.0.1:{}", port);
    info!(address = %bind_addr, "Binding server to address");
    let server = server.bind(("127.0.0.1", port))?;
    info!(address = %bind_addr, "Server bound successfully");

    info!(
        address = %bind_addr,
        routes = %"GET /api/health, POST /api/auth/register, POST /api/auth/login, POST /api/auth/token, POST /api/accounts, GET /api/accounts/{id}, POST /api/accounts/{id}/deposit, POST /api/accounts/{id}/withdraw, POST /api/transfers",
        "Starting HTTP server"
    );
    server.run().await
}
