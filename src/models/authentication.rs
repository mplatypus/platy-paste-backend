use crate::app::{application::App, database::Database};
use axum::{RequestPartsExt, extract::FromRequestParts, http::request::Parts};
use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Bearer},
};
use base64::{Engine, prelude::BASE64_URL_SAFE};
use secrecy::{ExposeSecret, SecretString};
use sqlx::PgTransaction;

use super::{
    error::{AppError, AuthError},
    snowflake::Snowflake,
};

#[derive(Clone, Debug)]
pub struct Token {
    /// The paste ID the token is attached to.
    paste_id: Snowflake,
    /// The token for the paste.
    token: SecretString,
}

impl Token {
    /// New.
    ///
    /// Create a new [`Token`] object.
    pub const fn new(paste_id: Snowflake, token: SecretString) -> Self {
        Self { paste_id, token }
    }

    /// The owning paste ID.
    pub const fn paste_id(&self) -> Snowflake {
        self.paste_id
    }

    /// The authentication token.
    pub fn token(&self) -> SecretString {
        self.token.clone()
    }

    /// Fetch.
    ///
    /// Fetch a paste ID from its token.
    ///
    /// ## Arguments
    ///
    /// - `db` - The database to make the request to.
    /// - `token` - The token of the paste.
    ///
    /// ## Errors
    ///
    /// - [`AppError`] - The database had an error.
    ///
    /// ## Returns
    ///
    /// - [`Option::Some`] - The [`Token`] object.
    /// - [`Option::None`] - No token was found.
    pub async fn fetch(db: &Database, token: String) -> Result<Option<Self>, AppError> {
        Ok(sqlx::query_as!(
            Self,
            "SELECT paste_id, token FROM paste_tokens WHERE token = $1",
            token,
        )
        .fetch_optional(db.pool())
        .await?)
    }

    /// Insert.
    ///
    /// Insert (create) a paste token.
    ///
    /// ## Arguments
    ///
    /// - `transaction` The transaction to use.
    ///
    /// ## Errors
    ///
    /// - [`AppError`] - The database had an error, or the snowflake exists already.
    pub async fn insert(&self, transaction: &mut PgTransaction<'_>) -> Result<(), AppError> {
        let paste_id: i64 = self.paste_id.into();
        sqlx::query!(
            "INSERT INTO paste_tokens(paste_id, token) VALUES ($1, $2)",
            paste_id,
            self.token.expose_secret()
        )
        .execute(transaction.as_mut())
        .await?;

        Ok(())
    }

    /// Delete.
    ///
    /// Delete a token.
    ///
    /// ## Arguments
    ///
    /// - `db` - The database to make the request to.
    /// - `token` - The token of the paste.
    ///
    /// ## Errors
    ///
    /// - [`AppError`] - The database had an error.
    pub async fn delete(db: &Database, token: String) -> Result<(), AppError> {
        sqlx::query!("DELETE FROM paste_tokens WHERE token = $1", token,)
            .execute(db.pool())
            .await?;

        Ok(())
    }
}

impl FromRequestParts<App> for Token {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &App) -> Result<Self, Self::Rejection> {
        #[allow(clippy::manual_let_else)]
        let TypedHeader(Authorization(bearer)) = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await
            .map_err(|_| AuthError::MissingCredentials)?;

        let bot = Self::fetch(&state.database, bearer.token().to_string())
            .await?
            .ok_or(AuthError::InvalidToken)?;

        Ok(bot)
    }
}

/// Generate Token.
///
/// ## Parameters
///
/// - `paste_id` - The paste attached to the token.
///
/// ## Errors
///
/// - [`AppError`] - Raise when it fails to fill random integers.
///
/// ## Returns
///
/// The [`SecretString`] (token) generated.
pub fn generate_token(paste_id: Snowflake) -> Result<SecretString, AppError> {
    const TOKEN_LENGTH: usize = 25;

    let mut buffer: Vec<u8> = vec![0; TOKEN_LENGTH];

    getrandom::fill(&mut buffer).map_err(|e| {
        AppError::InternalServer(format!("Failed to obtain a random integers: {e}"))
    })?;

    let ascii = String::from("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ-");

    let unique_token = buffer
        .iter() // Convert to an iterator.
        .map(|x| ascii.as_bytes()[(*x as usize) % ascii.len()] as char) // This maps the ascii table to the buffer
        .collect::<String>(); // Collect the items into a string.

    let paste_id_encrypted = BASE64_URL_SAFE.encode(paste_id.to_string());

    Ok(SecretString::new(
        format!("{paste_id_encrypted}.{unique_token}").into(),
    ))
}
