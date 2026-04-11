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

use crate::models::DtUtc;

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
    /// ## Handler
    ///
    /// Errors from [`HandlerError`].
    #[error(transparent)]
    Handler(#[from] HandlerError),
}

/// ## Handler Error
///
/// Errors related to the handler.
#[derive(Error, Debug)]
pub enum HandlerError {
    /// ## Database
    ///
    /// Errors from [`DatabaseError`].
    #[error("Database: {0}")]
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
    /// ## MPSC
    ///
    /// Errors from [`tokio::sync::mpsc::error::SendError<T>`].
    #[error("MPSC Error: {0}")]
    Mpsc(String),
    /// ## Oneshot
    ///
    /// Errors from [`tokio::sync::oneshot::error::RecvError`].
    #[error("Oneshot Error: {0}")]
    Oneshot(String),
    /// The handler actor has already been started.
    #[error("The handler actor has already been started")]
    AlreadyStarted,
    /// The handler actor has not been started.
    #[error("The handler actor has not been started.")]
    NotStarted,
    /// The handler actor has been closed.
    #[error("The handler actor has been closed.")]
    Closed,
    /// The handler actor timed out.
    #[error("The handler actor timed out.")]
    Timeout,
}

/// Implemented for easy conversion without mapping error type.
impl<T> From<tokio::sync::mpsc::error::SendError<T>> for HandlerError {
    fn from(value: tokio::sync::mpsc::error::SendError<T>) -> Self {
        Self::Mpsc(value.to_string())
    }
}

/// Implemented for easy conversion without mapping error type.
impl From<tokio::sync::oneshot::error::RecvError> for HandlerError {
    fn from(value: tokio::sync::oneshot::error::RecvError) -> Self {
        Self::Oneshot(value.to_string())
    }
}

/// Implemented for easy conversion without mapping error type.
impl From<tokio::time::error::Elapsed> for HandlerError {
    fn from(_value: tokio::time::error::Elapsed) -> Self {
        Self::Timeout
    }
}

impl IntoResponse for HandlerError {
    fn into_response(self) -> Response {
        match self {
            Self::Database(error) => error.into_response(),
            Self::ObjectStore(error) => error.into_response(),
            Self::Generate(error) => error.into_response(),
            Self::Mpsc(error) => RESTErrorResponse::new_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "MPSC Error",
                error,
            ),
            Self::Oneshot(error) => RESTErrorResponse::new_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Oneshot Error",
                error,
            ),
            Self::AlreadyStarted => RESTErrorResponse::new_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Handler Error",
                "This handler has already been started.",
            ),
            Self::NotStarted => RESTErrorResponse::new_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Handler Error",
                "This handler has not yet been started.",
            ),
            Self::Closed => RESTErrorResponse::new_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Handler Error",
                "This handler has been closed.",
            ),
            Self::Timeout => RESTErrorResponse::new_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Handler Error",
                "A request to the handler timed out.",
            ),
        }
    }
}

/// ## Database Error
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
        match self {
            Self::Sqlx(error) => RESTErrorResponse::new_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "SQLX Error",
                error,
            ),
            Self::Migrate(error) => RESTErrorResponse::new_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Migration Error",
                error,
            ),
            Self::Custom(error) => {
                RESTErrorResponse::new_response(StatusCode::BAD_REQUEST, "Custom Error", error)
            }
        }
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
    fn from(error: aws_sdk_s3::error::SdkError<E, R>) -> Self {
        Self::S3(aws_sdk_s3::error::DisplayErrorContext(error).to_string())
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
        match self {
            Self::S3(error) => RESTErrorResponse::new_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "S3 Service Error",
                error,
            ),
        }
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
        match self {
            Self::GetRandom(error) => RESTErrorResponse::new_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Get Random Value Error",
                error,
            ),
        }
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
        match self {
            Self::Json(error) => {
                RESTErrorResponse::new_response(StatusCode::BAD_REQUEST, "Json Parse Error", error)
            }
            Self::Regex(error) => RESTErrorResponse::new_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Regex Error",
                error,
            ),
            Self::HeaderToStr(error) => RESTErrorResponse::new_response(
                StatusCode::BAD_REQUEST,
                "Header To String Error",
                error,
            ),
            Self::MimeFromStr(error) => RESTErrorResponse::new_response(
                StatusCode::BAD_REQUEST,
                "Mime From String Error",
                error,
            ),
            Self::FromUtf8(error) => {
                RESTErrorResponse::new_response(StatusCode::BAD_REQUEST, "From UTF-8 Error", error)
            }
            Self::ParseInt(error) => RESTErrorResponse::new_response(
                StatusCode::BAD_REQUEST,
                "Parse Integer Error",
                error,
            ),
            Self::ParseSnowflake(error) => RESTErrorResponse::new_response(
                StatusCode::BAD_REQUEST,
                "Parse Snowflake Error",
                error,
            ),
        }
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
        match self {
            Self::MissingCredentials => RESTErrorResponse::new_response(
                StatusCode::UNAUTHORIZED,
                "Missing Credentials",
                "Missing Token",
            ),
            Self::InvalidCredentials => RESTErrorResponse::new_response(
                StatusCode::UNAUTHORIZED,
                "Invalid Credentials",
                "Invalid Token and/or mismatched paste ID",
            ),
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
    /// ## Handler
    ///
    /// Errors from [`HandlerError`].
    #[error(transparent)]
    Handler(#[from] HandlerError),
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

impl RESTError {
    /// The easier method of using [`Self::InternalServer`] that takes any value that can be displayed.
    pub fn internal_server<T>(e: T) -> Self
    where
        T: std::fmt::Display,
    {
        Self::InternalServer(e.to_string())
    }

    /// The easier method of using [`Self::BadRequest`] that takes any value that can be displayed.
    pub fn bad_request<T>(e: T) -> Self
    where
        T: std::fmt::Display,
    {
        Self::BadRequest(e.to_string())
    }

    /// The easier method of using [`Self::NotFound`] that takes any value that can be displayed.
    pub fn not_found<T>(e: T) -> Self
    where
        T: std::fmt::Display,
    {
        Self::NotFound(e.to_string())
    }
}

impl IntoResponse for RESTError {
    fn into_response(self) -> Response {
        match self {
            Self::Authentication(error) => error.into_response(),
            Self::Database(error) => error.into_response(),
            Self::ObjectStore(error) => error.into_response(),
            Self::Handler(error) => error.into_response(),
            Self::Generate(error) => error.into_response(),
            Self::Parse(error) => error.into_response(),
            Self::Rejection(error) => error.into_response(),
            Self::Multipart(error) => error.into_response(),
            Self::InternalServer(ref e) => RESTErrorResponse::new_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal Server Error",
                e,
            ),
            Self::BadRequest(ref e) => {
                RESTErrorResponse::new_response(StatusCode::BAD_REQUEST, "Bad Request", e)
            }
            Self::NotFound(ref e) => {
                RESTErrorResponse::new_response(StatusCode::NOT_FOUND, "Not Found", e)
            }
        }
    }
}

/// ## REST Error Response
///
/// The JSON response sent when an error occurs.
#[derive(Serialize, Deserialize)]
pub struct RESTErrorResponse {
    /// The reason for the error.
    reason: String,
    /// The message about the error.
    message: String,
    /// Time since epoch of when the error occurred.
    timestamp: u64,
}

impl RESTErrorResponse {
    /// ## New
    ///
    /// Create a new [`RESTErrorResponse`] object.
    pub const fn new(reason: String, message: String, timestamp: DtUtc) -> Self {
        Self {
            reason,
            message,
            timestamp: timestamp.timestamp() as u64,
        }
    }

    /// ## New Response
    ///
    /// Creates a new [`Response`] object where the body is a [`RESTErrorResponse`] as JSON.
    ///
    /// ## Parameters
    /// - `status_code` - The status code to set the response to.
    /// - `reason` - The reason this error occurred.
    /// - `message` - The full error message.
    pub fn new_response<R: std::fmt::Display, M: std::fmt::Display>(
        status_code: StatusCode,
        reason: R,
        message: M,
    ) -> Response {
        (
            status_code,
            Json(Self {
                reason: reason.to_string(),
                message: message.to_string(),
                timestamp: Utc::now().timestamp() as u64,
            }),
        )
            .into_response()
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
    pub fn message(&self) -> &str {
        &self.message
    }

    // Testing item, docs not needed.
    #[expect(missing_docs)]
    pub fn timestamp(&self) -> u64 {
        self.timestamp
    }
}
