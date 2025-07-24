use aws_sdk_s3::error::{DisplayErrorContext, SdkError};
use axum::{
    Json,
    extract::multipart::MultipartError,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use std::num::ParseIntError;
use thiserror::Error;
use time::OffsetDateTime;

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum AppError {
    #[error("Authentication error: {0}")]
    Authentication(#[from] AuthError),
    #[error("Database transaction failed: {0}")]
    Database(#[from] sqlx::Error),
    #[error("S3 Client failed: {0}")]
    S3Client(String),
    #[error("Multipart error: {0}")]
    Multipart(#[from] MultipartError),
    #[error("JSON Error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Parse Int Error: {0}")]
    ParseInt(#[from] ParseIntError),
    #[error("Internal Server Error: {0}")]
    InternalServer(String),
    #[error("Bad Request Error: {0}")]
    BadRequest(String),
    #[error("Not Found: {0}")]
    NotFound(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, reason, trace): (StatusCode, &str, &str) = match self {
            Self::Authentication(e) => return e.into_response(),
            Self::Database(e) => (StatusCode::BAD_REQUEST, "Database Error", &e.to_string()),
            Self::Multipart(e) => return e.into_response(),
            Self::Json(e) => (StatusCode::BAD_REQUEST, "Json Error", &e.to_string()),
            Self::S3Client(ref e) => (StatusCode::INTERNAL_SERVER_ERROR, "S3 Error", e),
            Self::ParseInt(e) => (
                StatusCode::BAD_REQUEST,
                "Failed to parse integer",
                &e.to_string(),
            ),
            Self::InternalServer(ref e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal Server Error",
                e,
            ),
            Self::BadRequest(ref e) => (StatusCode::BAD_REQUEST, "Bad Request", e),
            Self::NotFound(ref e) => (StatusCode::NOT_FOUND, "Not Found", e),
        };

        let body = Json(ErrorResponse {
            timestamp: OffsetDateTime::now_utc().unix_timestamp() as u64,
            reason: String::from(reason),
            trace: Some(trace.to_string()), // This should only appear if the trace is requested (the query contains trace=True)
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

#[derive(Serialize, Debug)]
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
    #[error("Invalid Token and/or mismatched paste ID")]
    InvalidCredentials,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, reason): (StatusCode, &str) = match self {
            Self::MissingCredentials => (StatusCode::UNAUTHORIZED, "Missing Credentials"),
            Self::InvalidCredentials => (
                StatusCode::UNAUTHORIZED,
                "Invalid Token and/or mismatched paste ID",
            ),
        };

        let body = Json(ErrorResponse {
            timestamp: OffsetDateTime::now_utc().unix_timestamp() as u64,
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
