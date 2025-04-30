pub mod app;
pub mod models;
pub mod rest;

use axum::{
    Router,
    http::HeaderValue,
    response::{IntoResponse, Response},
};
use http::{Method, header};
use models::{
    error::AppError,
    paste::{ExpiryTaskMessage, expiry_tasks},
};
use time::{UtcOffset, format_description};
use tokio::sync::mpsc;
use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder};
use tower_http::{cors::CorsLayer, timeout::TimeoutLayer, trace::TraceLayer};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt::time::OffsetTime, layer::SubscriberExt};

use std::{net::SocketAddr, sync::Arc, time::Duration};

#[tokio::main]
async fn main() {
    let offset = UtcOffset::current_local_offset().expect("should get local offset!");
    let timer = OffsetTime::new(
        offset,
        format_description::parse(
            "[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:3]",
        )
        .expect("Could not format time."),
    );

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

    let state: Arc<app::application::ApplicationState> =
        match app::application::ApplicationState::new().await {
            Ok(s) => s,
            Err(err) => panic!("Failed to build state: {err}"),
        };

    let cors = CorsLayer::new()
        .allow_origin(
            state
                .config
                .domain()
                .parse::<HeaderValue>()
                .expect("Failed to parse CORS domain."),
        )
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PATCH,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([header::ACCEPT, header::CONTENT_TYPE, header::AUTHORIZATION]);

    let limiter = GovernorLayer {
        config: Arc::new(
            GovernorConfigBuilder::default()
                .per_second(60)
                .burst_size(state.config.rate_limits().global())
                .period(Duration::from_secs(5))
                .use_headers()
                .finish()
                .expect("Failed to build global paste limiter."),
        ),
    };

    let app = Router::new()
        .nest("/v1", rest::paste::generate_router(&state.config))
        .nest("/v1", rest::document::generate_router(&state.config))
        .layer(TraceLayer::new_for_http())
        .layer(TimeoutLayer::new(Duration::from_secs(10)))
        .layer(cors)
        .layer(limiter)
        .fallback(fallback)
        .with_state(state.clone());

    let host = state.config.host();
    let port = state.config.port();

    let version = env!("CARGO_PKG_VERSION");

    tracing::info!(
        "Running Platy Paste Backend ({}) on {}:{}",
        version,
        host,
        port
    );

    let (expire_sender, expire_receiver) = mpsc::channel::<ExpiryTaskMessage>(10);

    let expiry_task = tokio::task::spawn(expiry_tasks(state, expire_receiver));

    let listener = tokio::net::TcpListener::bind(format!("{host}:{port}"))
        .await
        .expect("Failed to bind to address");

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .expect("Failed creating server");

    match expire_sender.blocking_send(ExpiryTaskMessage::Cancel) {
        Ok(()) => {
            expiry_task
                .await
                .expect("Failed to cleanly shut down expiry task.");
        }
        Err(e) => {
            tracing::error!("Failed to cleanly shutdown message task! Reason: {e}");
        }
    };
}

async fn fallback() -> Response {
    AppError::NotFound("This endpoint does not exist.".to_string()).into_response()
}
