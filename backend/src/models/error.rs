use aws_sdk_s3::error::{DisplayErrorContext, SdkError};
use axum::{
    Json,
    extract::multipart::{self, MultipartError},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use std::{
    num::ParseIntError,
    time::{SystemTime, UNIX_EPOCH},
};
use thiserror::Error;

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum AppError {
    #[error("Database transaction failed: {0}")]
    Database(#[from] sqlx::Error),
    #[error("S3 Client failed: {0}")]
    S3Client(String),
    #[error("Authentication error: {0}")]
    Authentication(#[from] AuthError),
    #[error("Multipart error: {0}")]
    Multipart(#[from] MultipartError),
    #[error("JSON Error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Reqwest Error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("Parse Int Error: {0}")]
    ParseIntError(#[from] ParseIntError),
    #[error("Not Found: {0}")]
    NotFound(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, reason): (StatusCode, &str) = match self {
            Self::Database(_) => (StatusCode::BAD_REQUEST, "Database Error."),
            Self::Authentication(e) => return e.into_response(),
            Self::Multipart(e) => return e.into_response(),
            Self::Json(_) => (StatusCode::BAD_REQUEST, ""),
            Self::Reqwest(_) => (StatusCode::BAD_REQUEST, ""),
            Self::S3Client(_) => (StatusCode::INTERNAL_SERVER_ERROR, ""),
            Self::ParseIntError(_) => (StatusCode::BAD_REQUEST, "Failed to parse integer."),
            Self::NotFound(ref e) => (StatusCode::NOT_FOUND, &e.clone()),
        };
        if status == StatusCode::INTERNAL_SERVER_ERROR {
            tracing::error!(error = %self);
        }
        let body = Json(ErrorResponse {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Could not fetch current time.")
                .as_secs(),
            reason: String::from(reason),
            trace: None,
        });
        (status, body).into_response()
    }
}

impl<E, R> From<SdkError<E, R>> for AppError
where
    E: std::error::Error + Send + Sync + 'static,
    R: std::fmt::Debug,
{
    fn from(e: SdkError<E, R>) -> Self {
        Self::S3Client(DisplayErrorContext(e).to_string())
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Error {
    reason: String,
    trace: Option<String>,
    timestamp: i64,
}

/// Authentication Errors
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum AuthError {
    #[error("Credentials are invalid and/or missing")]
    MissingCredentials,
    #[error("Permissions are invalid and/or missing")]
    MissingPermissions,
    #[error("The token provided does not exist")]
    InvalidToken,
    #[error("Not Found: {0}")]
    NotFound(String),
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, reason): (StatusCode, &str) = match self {
            Self::MissingCredentials => (StatusCode::UNAUTHORIZED, "Missing Credentials."),
            Self::MissingPermissions => (StatusCode::UNAUTHORIZED, "Missing Permissions."),
            Self::InvalidToken => (StatusCode::UNAUTHORIZED, "Invalid Token."),
            Self::NotFound(_) => (StatusCode::NOT_FOUND, "Not Found"),
        };

        let body = Json(ErrorResponse {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Could not fetch current time.")
                .as_secs(),
            reason: String::from(reason),
            trace: None,
        });
        (status, body).into_response()
    }
}

#[derive(Serialize, Deserialize)]
pub struct ErrorResponse {
    /// The reason for the error.
    reason: String,
    /// The trace (more information) of the error.
    trace: Option<String>,
    /// Time since epoch of when the error occurred.
    timestamp: u64,
}
