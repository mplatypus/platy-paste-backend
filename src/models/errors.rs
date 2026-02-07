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
pub enum ParseError {
    #[error("JSON Error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Regex Error: {0}")]
    Regex(#[from] regex::Error),
    #[error("Header To String Error: {0}")]
    HeaderToStr(#[from] http::header::ToStrError),
    #[error("Mime From String Error: {0}")]
    MimeFromStr(#[from] mime::FromStrError),
    #[error("From UTF-8 Error: {0}")]
    FromUtf8(#[from] std::string::FromUtf8Error),
    #[error("Parse Integer Error: {0}")]
    ParseInt(#[from] std::num::ParseIntError),
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

#[derive(Error, Debug)]
pub enum RejectionError {
    #[error("Multipart Rejection Error: {0}")]
    Multipart(#[from] axum::extract::multipart::MultipartRejection),
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

#[derive(Error, Debug)]
pub enum RESTError {
    // Library Errors
    #[error(transparent)]
    Authentication(#[from] AuthenticationError),
    #[error(transparent)]
    Database(#[from] DatabaseError),
    #[error(transparent)]
    ObjectStore(#[from] ObjectStoreError),
    #[error(transparent)]
    Generate(#[from] GenerateError),
    #[error(transparent)]
    Parse(#[from] ParseError),
    #[error(transparent)]
    Rejection(#[from] RejectionError),
    // Crate Errors
    #[error(transparent)]
    Multipart(#[from] axum::extract::multipart::MultipartError),
    // Custom Errors
    #[error("Internal Server Error: {0}")]
    InternalServer(String),
    #[error("Bad Request Error: {0}")]
    BadRequest(String),
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

        let body = Json(RESTErrorResponse::new(reason, None));

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

impl RESTErrorResponse {
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
    pub fn reason(&self) -> &str {
        &self.reason
    }

    pub fn trace(&self) -> Option<&str> {
        self.trace.as_deref()
    }

    pub fn timestamp(&self) -> u64 {
        self.timestamp
    }
}
