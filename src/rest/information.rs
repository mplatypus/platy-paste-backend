//! Information related endpoints and router generator.

use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, State},
    response::{IntoResponse as _, Response},
    routing::get,
};
use http::StatusCode;

use crate::{
    app::{application::App, config::Config},
    models::{
        errors::RESTError,
        payload::information::{ResponseConfig, ResponseInformation, ResponseStatus},
    },
};

/// ## Generate Router
///
/// Generates the router for document related endpoints.
///
/// ## Returns
/// The router with all the document related endpoints attached.
pub fn generate_router(config: &Config) -> Router<App> {
    Router::new()
        .route("/information", get(get_information))
        .route("/information/status", get(get_status))
        .route("/information/configuration", get(get_configuration))
        .layer(DefaultBodyLimit::max(
            config.size_limits().maximum_total_document_size(),
        ))
}

/// Get Status.
///
/// Get the servers current status.
///
/// ## Errors
/// Returns an error if the request failed.
///
/// ## Returns
///
/// - `200` - The [`ResponseStatus`] object.
pub async fn get_status() -> Result<Response, RESTError> {
    let response_config = ResponseStatus::new("ok".to_string());

    Ok((StatusCode::OK, Json(response_config)).into_response())
}

/// Get Information.
///
/// Get information about the server.
///
/// ## Errors
/// Returns an error if the request failed.
///
/// ## Returns
///
/// - `200` - The [`ResponseInformation`] object.
/// - `500` - An internal server error occurred.
pub async fn get_information(State(_app): State<App>) -> Result<Response, RESTError> {
    let response_config = ResponseInformation::from_env()
        .map_err(|err| RESTError::InternalServer(format!("Errors: {}", err.join(", "))))?;

    Ok((StatusCode::OK, Json(response_config)).into_response())
}

/// Get Configuration.
///
/// Get the servers current configuration information.
///
/// ## Errors
/// Returns an error if the request failed.
///
/// ## Returns
///
/// - `200` - The [`ResponseConfig`] object.
pub async fn get_configuration(State(app): State<App>) -> Result<Response, RESTError> {
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

    use crate::models::payload::information::{
        ResponseConfig, ResponseInformation, ResponseStatus,
    };
    use crate::rest::generate_router as main_generate_router;

    mod v1 {
        use super::*;

        mod get_status {

            use super::*;

            #[sqlx::test]
            async fn test_successful(pool: PgPool) {
                let config = Config::test_builder()
                    .build()
                    .expect("Failed to build config.");
                let object_store = TestObjectStore::new();
                let state = ApplicationState::new_tests(config.clone(), pool, object_store.clone())
                    .await
                    .expect("Failed to build application state.");

                let app = main_generate_router(state);
                let server = TestServer::new(app);

                let response = server.get("/v1/information/status").await;

                response.assert_status(StatusCode::OK);

                response.assert_header("Content-Type", "application/json");

                let body = response.as_bytes();

                let expected_body = serde_json::to_vec(&ResponseStatus::new("ok".to_string()))
                    .expect("Failed to build expected body.");
                assert_eq!(body.to_vec(), expected_body, "Body does not match.");
            }
        }

        mod get_information {

            use super::*;

            #[sqlx::test]
            async fn test_successful(pool: PgPool) {
                let config = Config::test_builder()
                    .build()
                    .expect("Failed to build config.");
                let object_store = TestObjectStore::new();
                let state = ApplicationState::new_tests(config.clone(), pool, object_store.clone())
                    .await
                    .expect("Failed to build application state.");

                let app = main_generate_router(state);
                let server = TestServer::new(app);

                let response = server.get("/v1/information").await;

                response.assert_status(StatusCode::OK);

                response.assert_header("Content-Type", "application/json");

                let body = response.as_bytes();

                let expected_body = serde_json::to_vec(
                    &ResponseInformation::from_env()
                        .expect("Failed to build response information payload."),
                )
                .expect("Failed to build expected body.");

                assert_eq!(&body.to_vec(), &expected_body, "Body does not match.");
            }
        }

        mod get_config {

            use super::*;

            #[sqlx::test]
            async fn test_successful(pool: PgPool) {
                let config = Config::test_builder()
                    .build()
                    .expect("Failed to build config.");
                let object_store = TestObjectStore::new();
                let state = ApplicationState::new_tests(config.clone(), pool, object_store.clone())
                    .await
                    .expect("Failed to build application state.");

                let app = main_generate_router(state);
                let server = TestServer::new(app);

                let response = server.get("/v1/information/configuration").await;

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
