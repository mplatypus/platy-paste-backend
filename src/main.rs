pub mod app;
pub mod models;
pub mod rest;

use chrono::Local;
use models::paste::expiry_tasks;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt::time::FormatTime, layer::SubscriberExt};

use std::net::SocketAddr;

use crate::rest::generate_router;

#[tokio::main]
async fn main() {
    #[derive(Clone)]
    struct LocalTimer;

    impl FormatTime for LocalTimer {
        fn format_time(
            &self,
            w: &mut tracing_subscriber::fmt::format::Writer<'_>,
        ) -> std::fmt::Result {
            write!(w, "{}", Local::now().format("%Y-%m-%d %H:%M:%S%.3f"))
        }
    }

    let timer = LocalTimer {};

    let file_appender = RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .max_log_files(25)
        .filename_prefix("platy-paste")
        .filename_suffix("log")
        .build("./logs/")
        .expect("Rolling File Appender Failed to build.");

    let (file_non_blocking, _file_guard) = tracing_appender::non_blocking(file_appender);

    let file_subscriber = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_writer(file_non_blocking)
        .with_timer(timer.clone());

    let (console_non_blocking, _console_guard) = tracing_appender::non_blocking(std::io::stdout());
    let console_subscriber = tracing_subscriber::fmt::layer()
        .with_writer(console_non_blocking)
        .with_timer(timer.clone());

    let subscriber = tracing_subscriber::registry()
        .with(file_subscriber)
        .with(console_subscriber);

    tracing::subscriber::set_global_default(subscriber).expect("Failed to set subscriber");

    let state: app::application::App = match app::application::ApplicationState::new().await {
        Ok(s) => s,
        Err(err) => panic!("Failed to build state: {err}"),
    };

    let expiry_state = state.clone();

    let config = state.config().clone();

    let app = generate_router(state);

    let host = config.host();
    let port = config.port();

    let version = env!("CARGO_PKG_VERSION");

    tracing::info!(
        "Running Platy Paste Backend ({}) on {}:{}",
        version,
        host,
        port
    );

    let expiry_task = tokio::task::spawn(expiry_tasks(expiry_state));

    let listener = tokio::net::TcpListener::bind(format!("{host}:{port}"))
        .await
        .expect("Failed to bind to address");

    let shutdown_signal = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to listen for shutdown signal");
    };

    let server = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    );

    tokio::select! {
        _ = server.with_graceful_shutdown(shutdown_signal) => {
            expiry_task.abort();
            tracing::info!("Successfully shutdown expiry task and server.");
        },
    }
}
