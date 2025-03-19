use axum::{extract::{DefaultBodyLimit, State}, response::Response, routing::get, Router};

use crate::{app::app::App, models::{error::AppError, token::Token}};


pub fn generate_router() -> Router<App> {
    Router::new()
        .route("/supported-types", get(get_supported_types))
        .route("/generate", get(get_generate))
        .route("/callback", get(get_callback))
        .layer(DefaultBodyLimit::disable())
}

async fn get_supported_types(State(_app): State<App>, _: Token) -> Result<Response, AppError> {
    todo!("Implement me!")
}

async fn get_generate(State(_app): State<App>, _: Token) -> Result<Response, AppError> {
    todo!("Implement me!")
}

async fn get_callback(State(_app): State<App>, _: Token) -> Result<Response, AppError> {
    todo!("Implement me!")
}