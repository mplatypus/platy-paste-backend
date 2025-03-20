use axum::{extract::DefaultBodyLimit, Router};

use crate::app::app::App;

pub fn generate_router() -> Router<App> {
    Router::new()
        .layer(DefaultBodyLimit::disable())
}