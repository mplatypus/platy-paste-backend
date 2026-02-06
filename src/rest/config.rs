use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, State},
    response::{IntoResponse, Response},
    routing::get,
};
use http::StatusCode;

use crate::{
    app::{application::App, config::Config},
    models::{errors::RESTError, payload::ResponseConfig},
};

pub fn generate_router(config: &Config) -> Router<App> {
    Router::new()
        .route("/config", get(get_config))
        .layer(DefaultBodyLimit::max(
            config.size_limits().maximum_total_document_size(),
        ))
}

pub async fn get_config(State(app): State<App>) -> Result<Response, RESTError> {
    let response_config = ResponseConfig::from_config(app.config());

    Ok((StatusCode::OK, Json(response_config)).into_response())
}

#[cfg(test)]
mod tests {
    use axum_test::TestServer;
    use http::StatusCode;
    use sqlx::PgPool;

    use crate::app::{
        application::ApplicationState, config::Config, object_store::TestObjectStore,
    };

    use crate::models::payload::ResponseConfig;
    use crate::rest::generate_router as main_generate_router;

    mod v1 {
        use super::*;

        mod get_config {
            use super::*;

            #[sqlx::test]
            async fn test_working(pool: PgPool) {
                let config = Config::test_builder()
                    .build()
                    .expect("Failed to build config.");
                let object_store = TestObjectStore::new();
                let state = ApplicationState::new_tests(config.clone(), pool, object_store.clone())
                    .await
                    .expect("Failed to build application state.");

                let app = main_generate_router(state);
                let server = TestServer::new(app).expect("Failed to build server.");

                let response = server.get("/v1/config").await;

                response.assert_status(StatusCode::OK);

                response.assert_header("Content-Type", "application/json");

                let body = response.as_bytes();

                let expected_body = serde_json::to_vec(&ResponseConfig::from_config(&config))
                    .expect("Failed to build expected body.");
                assert_eq!(body.to_vec(), expected_body, "Body does not match.");
            }
        }
    }
}
