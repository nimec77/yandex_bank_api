use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

pub fn init_logging() {
    // Set default log level to "info" if RUST_LOG is not set
    // RUST_LOG format: "module::path=level,other_module=level"
    // Example: "RUST_LOG=actix_web=info,yandex_bank_api=debug"
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(filter)
        .with(
            fmt::layer()
                .with_target(true) // Show module/target in logs
                .with_file(true) // Show file name
                .with_line_number(true), // Show line number
        )
        .init();
}
