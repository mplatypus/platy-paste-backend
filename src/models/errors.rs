//! ## Errors
//!
//! Errors related to the application, and objects.

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// ## Application Error
///
/// Errors related to the applictions creation and lifetime.
#[derive(Error, Debug)]
pub enum ApplicationError {
    /// ## Database
    ///
    /// Errors from [`DatabaseError`].
    #[error(transparent)]
    Database(#[from] DatabaseError),
    /// ## Object Store
    ///
    /// Errors from [`ObjectStoreError`].
    #[error(transparent)]
    ObjectStore(#[from] ObjectStoreError),
}

/// ## Databse Error
///
/// Errors related to the database.
#[derive(Error, Debug)]
pub enum DatabaseError {
    /// ## SQLX
    ///
    /// Errors from [`sqlx::Error`].
    #[error("SQLX Error: {0}")]
    Sqlx(#[from] sqlx::Error),
    /// ## Migrate
    ///
    /// Errors from [`sqlx::migrate::MigrateError`].
    #[error("Migrate Error: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),
    /// ## Custom
    ///
    /// Custom database errors.
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

/// ## Object Store Error
///
/// Errors related to the object storage used.
#[derive(Error, Debug)]
pub enum ObjectStoreError {
    /// ## S3
    ///
    /// Errors from [`aws_sdk_s3::error::SdkError<E, R>`].
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

/// Implemented for easy conversion without mapping error type.
impl From<sqlx::Error> for RESTError {
    fn from(value: sqlx::Error) -> Self {
        Self::Database(DatabaseError::Sqlx(value))
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

/// ## Generate Error
///
/// Errors related to the generation of custom values.
#[derive(Error, Debug)]
pub enum GenerateError {
    /// ## Get Random
    ///
    /// Errors from [`getrandom::Error`].
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

/// ## Parse Error
///
/// Errors related to the parsing of different values to certain types.
#[derive(Error, Debug)]
pub enum ParseError {
    /// ## JSON
    ///
    /// Errors from [`serde_json::Error`].
    #[error("JSON Error: {0}")]
    Json(#[from] serde_json::Error),
    /// ## Regex
    ///
    /// Errors from [`regex::Error`].
    #[error("Regex Error: {0}")]
    Regex(#[from] regex::Error),
    /// ## Header To String
    ///
    /// Errors from [`http::header::ToStrError`].
    #[error("Header To String Error: {0}")]
    HeaderToStr(#[from] http::header::ToStrError),
    /// ## Mime From String
    ///
    /// Errors from [`mime::FromStrError`].
    #[error("Mime From String Error: {0}")]
    MimeFromStr(#[from] mime::FromStrError),
    /// ## From UTF-8
    ///
    /// Errors from [`std::string::FromUtf8Error`].
    #[error("From UTF-8 Error: {0}")]
    FromUtf8(#[from] std::string::FromUtf8Error),
    /// ## Parse Integer
    ///
    /// Errors from [`std::num::ParseIntError`].
    #[error("Parse Integer Error: {0}")]
    ParseInt(#[from] std::num::ParseIntError),
    /// ## Parse Snowflake
    ///
    /// Used to validate that a snowflake is not just a random number.
    #[error("Parse Snowflake Error: {0}")]
    ParseSnowflake(String),
}

/// Implemented for easy conversion without mapping error type.
impl From<serde_json::Error> for RESTError {
    fn from(value: serde_json::Error) -> Self {
        Self::Parse(ParseError::Json(value))
    }
}

/// Implemented for easy conversion without mapping error type.
impl From<regex::Error> for RESTError {
    fn from(value: regex::Error) -> Self {
        Self::Parse(ParseError::Regex(value))
    }
}

/// Implemented for easy conversion without mapping error type.
impl From<http::header::ToStrError> for RESTError {
    fn from(value: http::header::ToStrError) -> Self {
        Self::Parse(ParseError::HeaderToStr(value))
    }
}

/// Implemented for easy conversion without mapping error type.
impl From<mime::FromStrError> for RESTError {
    fn from(value: mime::FromStrError) -> Self {
        Self::Parse(ParseError::MimeFromStr(value))
    }
}

/// Implemented for easy conversion without mapping error type.
impl From<std::string::FromUtf8Error> for RESTError {
    fn from(value: std::string::FromUtf8Error) -> Self {
        Self::Parse(ParseError::FromUtf8(value))
    }
}

/// Implemented for easy conversion without mapping error type.
impl From<std::num::ParseIntError> for RESTError {
    fn from(value: std::num::ParseIntError) -> Self {
        Self::Parse(ParseError::ParseInt(value))
    }
}

impl IntoResponse for ParseError {
    fn into_response(self) -> Response {
        let (status, reason, trace): (StatusCode, &str, &str) = match self {
            Self::Json(e) => (StatusCode::BAD_REQUEST, "Json Parse Error", &e.to_string()),
            Self::Regex(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Regex Error",
                &e.to_string(),
            ),
            Self::HeaderToStr(e) => (
                StatusCode::BAD_REQUEST,
                "Header To String Error",
                &e.to_string(),
            ),
            Self::MimeFromStr(e) => (
                StatusCode::BAD_REQUEST,
                "Mime From String Error",
                &e.to_string(),
            ),
            Self::FromUtf8(e) => (StatusCode::BAD_REQUEST, "From UTF-8 Error", &e.to_string()),
            Self::ParseInt(e) => (
                StatusCode::BAD_REQUEST,
                "Parse Integer Error",
                &e.to_string(),
            ),
            Self::ParseSnowflake(e) => (
                StatusCode::BAD_REQUEST,
                "Parse Snowflake Error",
                &e.to_string(),
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

/// ## Rejection Error
///
/// Errors related to the parsing of different axum body types.
#[derive(Error, Debug)]
pub enum RejectionError {
    /// ## Multipart
    ///
    /// Errors from [`axum::extract::multipart::MultipartRejection`].
    #[error("Multipart Rejection Error: {0}")]
    Multipart(#[from] axum::extract::multipart::MultipartRejection),
    /// ## Bytes
    ///
    /// Errors from [`axum::extract::rejection::BytesRejection`].
    #[error("Bytes Rejection Error: {0}")]
    Bytes(#[from] axum::extract::rejection::BytesRejection),
}

/// Implemented for easy conversion without mapping error type.
impl From<axum::extract::multipart::MultipartRejection> for RESTError {
    fn from(value: axum::extract::multipart::MultipartRejection) -> Self {
        Self::Rejection(RejectionError::Multipart(value))
    }
}

/// Implemented for easy conversion without mapping error type.
impl From<axum::extract::rejection::BytesRejection> for RESTError {
    fn from(value: axum::extract::rejection::BytesRejection) -> Self {
        Self::Rejection(RejectionError::Bytes(value))
    }
}

impl IntoResponse for RejectionError {
    fn into_response(self) -> Response {
        match self {
            Self::Multipart(error) => error.into_response(),
            Self::Bytes(error) => error.into_response(),
        }
    }
}

/// ## REST Error
///
/// All error types that can be returned when making a REST request.
#[derive(Error, Debug)]
pub enum RESTError {
    // Library Errors
    /// ## Authentication
    ///
    /// Errors from [`AuthenticationError`].
    #[error(transparent)]
    Authentication(#[from] AuthenticationError),
    /// ## Database
    ///
    /// Errors from [`DatabaseError`].
    #[error(transparent)]
    Database(#[from] DatabaseError),
    /// ## Object Store
    ///
    /// Errors from [`ObjectStoreError`].
    #[error(transparent)]
    ObjectStore(#[from] ObjectStoreError),
    /// ## Generate
    ///
    /// Errors from [`GenerateError`].
    #[error(transparent)]
    Generate(#[from] GenerateError),
    /// ## Parse
    ///
    /// Errors from [`ParseError`].
    #[error(transparent)]
    Parse(#[from] ParseError),
    /// ## Rejection
    ///
    /// Errors from [`RejectionError`].
    #[error(transparent)]
    Rejection(#[from] RejectionError),
    // Crate Errors
    /// ## Multipart
    ///
    /// Errors from [`axum::extract::multipart::MultipartError`].
    #[error(transparent)]
    Multipart(#[from] axum::extract::multipart::MultipartError),
    // Custom Errors
    /// ## Internal Server
    ///
    /// Custom errors related to internal server issues (500).
    #[error("Internal Server Error: {0}")]
    InternalServer(String),
    /// ## Bad Request
    ///
    /// Custom errors related to bad requests (400).
    #[error("Bad Request Error: {0}")]
    BadRequest(String),
    /// ## Not Found
    ///
    /// Custom errors related to unfound items or endpoints (404).
    #[error("Not Found: {0}")]
    NotFound(String),
}

impl IntoResponse for RESTError {
    fn into_response(self) -> Response {
        let (status, reason, trace): (StatusCode, &str, &str) = match self {
            Self::Authentication(error) => return error.into_response(),
            Self::Database(error) => return error.into_response(),
            Self::ObjectStore(error) => return error.into_response(),
            Self::Generate(error) => return error.into_response(),
            Self::Parse(error) => return error.into_response(),
            Self::Rejection(error) => return error.into_response(),
            Self::Multipart(error) => return error.into_response(),
            Self::InternalServer(ref e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal Server Error",
                e,
            ),
            Self::BadRequest(ref e) => (StatusCode::BAD_REQUEST, "Bad Request", e),
            Self::NotFound(ref e) => (StatusCode::NOT_FOUND, "Not Found", e),
        };

        let body = Json(RESTErrorResponse::new(
            reason,
            Some(trace.to_string()), // TODO: This should only appear if the trace is requested (the query contains trace=True)
        ));

        (status, body).into_response()
    }
}

/// ## Authentication Errors
///
/// Errors related to authenticating to the server.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum AuthenticationError {
    /// ## Missing Credentials
    ///
    /// The user did not provide credentials.
    #[error("Credentials are invalid and/or missing")]
    MissingCredentials,
    /// ## Invalid Credentials
    ///
    /// The credentials that have been provided are invalid.
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

        let body = Json(RESTErrorResponse::new(reason, None));

        (status, body).into_response()
    }
}

/// ## REST Error Response
///
/// The JSON response sent when an error occurs.
#[derive(Serialize, Deserialize)]
pub struct RESTErrorResponse {
    /// The reason for the error.
    reason: String,
    /// The trace (more information) of the error.
    trace: Option<String>,
    /// Time since epoch of when the error occurred.
    timestamp: u64,
}

impl RESTErrorResponse {
    /// ## New
    ///
    /// Create a new [`RESTErrorResponse`] object.
    pub fn new(reason: impl ToString, trace: Option<String>) -> Self {
        Self {
            reason: reason.to_string(),
            trace: trace,
            timestamp: Utc::now().timestamp() as u64,
        }
    }
}

#[cfg(test)]
impl RESTErrorResponse {
    // Testing item, docs not needed.
    #[expect(missing_docs)]
    pub fn reason(&self) -> &str {
        &self.reason
    }

    // Testing item, docs not needed.
    #[expect(missing_docs)]
    pub fn trace(&self) -> Option<&str> {
        self.trace.as_deref()
    }

    // Testing item, docs not needed.
    #[expect(missing_docs)]
    pub fn timestamp(&self) -> u64 {
        self.timestamp
    }
}
