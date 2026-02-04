use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ApplicationError {
    #[error(transparent)]
    Database(#[from] DatabaseError),
    #[error(transparent)]
    ObjectStore(#[from] ObjectStoreError),
}

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("SQLX Error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("Migrate Error: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),
    #[error("Database Custom: {0}")]
    Custom(String),
}

impl IntoResponse for DatabaseError {
    fn into_response(self) -> Response {
        let (status, reason, trace): (StatusCode, &str, &str) = match self {
            Self::Sqlx(error) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "SQLX Error",
                &error.to_string(),
            ),
            Self::Migrate(error) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Migration Error",
                &error.to_string(),
            ),
            Self::Custom(error) => (StatusCode::BAD_REQUEST, "Error", &error.to_string()),
        };

        let body = Json(RESTErrorResponse {
            timestamp: Utc::now().timestamp() as u64,
            reason: String::from(reason),
            trace: Some(trace.to_string()), // TODO: This should only appear if the trace is requested (the query contains trace=True)
        });
        (status, body).into_response()
    }
}

#[derive(Error, Debug)]
pub enum ObjectStoreError {
    #[error("S3 Error: {0}")]
    S3(String),
}

impl<E, R> From<aws_sdk_s3::error::SdkError<E, R>> for ObjectStoreError
where
    E: std::error::Error + Send + Sync + 'static,
    R: std::fmt::Debug,
{
    fn from(e: aws_sdk_s3::error::SdkError<E, R>) -> Self {
        Self::S3(aws_sdk_s3::error::DisplayErrorContext(e).to_string())
    }
}

impl IntoResponse for ObjectStoreError {
    fn into_response(self) -> Response {
        let (status, reason, trace): (StatusCode, &str, &str) = match self {
            Self::S3(error) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "S3 Service Error",
                &error.to_string(),
            ),
        };

        let body = Json(RESTErrorResponse {
            timestamp: Utc::now().timestamp() as u64,
            reason: String::from(reason),
            trace: Some(trace.to_string()), // TODO: This should only appear if the trace is requested (the query contains trace=True)
        });
        (status, body).into_response()
    }
}

#[derive(Error, Debug)]
pub enum GenerateError {
    #[error("Get Random Error: {0}")]
    GetRandom(#[from] getrandom::Error),
}

impl IntoResponse for GenerateError {
    fn into_response(self) -> Response {
        let (status, reason, trace): (StatusCode, &str, &str) = match self {
            Self::GetRandom(error) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Get Random Value Error",
                &error.to_string(),
            ),
        };

        let body = Json(RESTErrorResponse {
            timestamp: Utc::now().timestamp() as u64,
            reason: String::from(reason),
            trace: Some(trace.to_string()), // TODO: This should only appear if the trace is requested (the query contains trace=True)
        });
        (status, body).into_response()
    }
}

#[derive(Error, Debug)]
pub enum RESTError {
    // Library Errors
    #[error(transparent)]
    Authentication(#[from] AuthenticationError),
    #[error(transparent)]
    Database(#[from] DatabaseError),
    #[error(transparent)]
    Generate(#[from] GenerateError),
    #[error(transparent)]
    ObjectStore(#[from] ObjectStoreError),
    // Crate Errors
    #[error("Multipart Error: {0}")]
    Multipart(#[from] axum::extract::multipart::MultipartError),
    #[error("JSON Error: {0}")]
    Json(#[from] serde_json::Error),
    // Custom Errors
    #[error("Internal Server Error: {0}")]
    InternalServer(String),
    #[error("Bad Request Error: {0}")]
    BadRequest(String),
    #[error("Not Found: {0}")]
    NotFound(String),
}

/// Implemented for easy conversion without mapping error type.
impl From<sqlx::Error> for RESTError {
    fn from(value: sqlx::Error) -> Self {
        RESTError::Database(DatabaseError::Sqlx(value))
    }
}

impl IntoResponse for RESTError {
    fn into_response(self) -> Response {
        let (status, reason, trace): (StatusCode, &str, &str) = match self {
            Self::Authentication(error) => return error.into_response(),
            Self::Database(error) => return error.into_response(),
            Self::Generate(error) => return error.into_response(),
            Self::ObjectStore(error) => return error.into_response(),
            Self::Multipart(error) => return error.into_response(),
            Self::Json(e) => (StatusCode::BAD_REQUEST, "Json Parse Error", &e.to_string()),
            Self::InternalServer(ref e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal Server Error",
                e,
            ),
            Self::BadRequest(ref e) => (StatusCode::BAD_REQUEST, "Bad Request", e),
            Self::NotFound(ref e) => (StatusCode::NOT_FOUND, "Not Found", e),
        };

        let body = Json(RESTErrorResponse {
            timestamp: Utc::now().timestamp() as u64,
            reason: String::from(reason),
            trace: Some(trace.to_string()), // TODO: This should only appear if the trace is requested (the query contains trace=True)
        });
        (status, body).into_response()
    }
}

/// Authentication Errors
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum AuthenticationError {
    #[error("Credentials are invalid and/or missing")]
    MissingCredentials,
    #[error("Invalid Token and/or mismatched paste ID")]
    InvalidCredentials,
}

impl IntoResponse for AuthenticationError {
    fn into_response(self) -> Response {
        let (status, reason): (StatusCode, &str) = match self {
            Self::MissingCredentials => (StatusCode::UNAUTHORIZED, "Missing Credentials"),
            Self::InvalidCredentials => (
                StatusCode::UNAUTHORIZED,
                "Invalid Token and/or mismatched paste ID",
            ),
        };

        let body = Json(RESTErrorResponse {
            timestamp: Utc::now().timestamp() as u64,
            reason: String::from(reason),
            trace: None,
        });

        (status, body).into_response()
    }
}

#[derive(Serialize, Deserialize)]
pub struct RESTErrorResponse {
    /// The reason for the error.
    reason: String,
    /// The trace (more information) of the error.
    trace: Option<String>,
    /// Time since epoch of when the error occurred.
    timestamp: u64,
}
