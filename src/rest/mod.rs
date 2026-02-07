pub mod config;
pub mod document;
pub mod paste;

use std::time::Duration;

use axum::Router;
use http::{HeaderValue, Method, StatusCode, header};
use tower_http::{cors::CorsLayer, timeout::TimeoutLayer, trace::TraceLayer};

use crate::{app::application::App, models::errors::RESTError};

pub fn generate_router(state: App) -> Router<()> {
    let config = state.config().clone();
    let cors = CorsLayer::new()
        .allow_origin(
            config
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

    Router::new()
        .nest("/v1", paste::generate_router(&config))
        .nest("/v1", document::generate_router(&config))
        .nest("/v1", config::generate_router(&config))
        .layer(TraceLayer::new_for_http())
        .layer(TimeoutLayer::with_status_code(
            StatusCode::GATEWAY_TIMEOUT,
            Duration::from_secs(10),
        )) // TODO: Not sure if gateway timeout makes sense for this.
        .layer(cors)
        .fallback(fallback)
        .with_state(state)
}

async fn fallback() -> RESTError {
    RESTError::NotFound("This endpoint does not exist.".to_string())
}
