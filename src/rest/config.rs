use std::{sync::Arc, time::Duration};

use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, State},
    response::{IntoResponse, Response},
    routing::get,
};
use http::StatusCode;
use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder};

use crate::{
    app::{application::App, config::Config},
    models::{error::AppError, payload::ResponseConfig},
};

pub fn generate_router(config: &Config) -> Router<App> {
    let global_limiter = GovernorLayer {
        config: Arc::new(
            GovernorConfigBuilder::default()
                .per_second(60)
                .burst_size(config.rate_limits().global_config())
                .period(Duration::from_secs(5))
                .use_headers()
                .finish()
                .expect("Failed to build global document limiter."),
        ),
    };

    let get_config_limiter = GovernorLayer {
        config: Arc::new(
            GovernorConfigBuilder::default()
                .per_second(60)
                .burst_size(config.rate_limits().get_config())
                .period(Duration::from_secs(5))
                .use_headers()
                .finish()
                .expect("Failed to build get document limiter."),
        ),
    };

    Router::new()
        .route("/config", get(get_config).layer(get_config_limiter))
        .layer(global_limiter)
        .layer(DefaultBodyLimit::max(
            config.size_limits().maximum_total_document_size(),
        ))
}

async fn get_config(State(app): State<App>) -> Result<Response, AppError> {
    let response_config = ResponseConfig::from_config(&app.config);

    Ok((StatusCode::OK, Json(response_config)).into_response())
}
