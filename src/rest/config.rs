use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, State},
    response::{IntoResponse, Response},
    routing::get,
};
use http::StatusCode;

use crate::{
    app::{application::App, config::Config},
    models::{error::AppError, payload::ResponseConfig},
};

pub fn generate_router(config: &Config) -> Router<App> {
    Router::new()
        .route("/config", get(get_config))
        .layer(DefaultBodyLimit::max(
            config.size_limits().maximum_total_document_size(),
        ))
}

async fn get_config(State(app): State<App>) -> Result<Response, AppError> {
    let response_config = ResponseConfig::from_config(&app.config);

    Ok((StatusCode::OK, Json(response_config)).into_response())
}
