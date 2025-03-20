use axum::{extract::FromRequestParts, http::request::Parts, RequestPartsExt};
use axum_extra::{headers::{authorization::Bearer, Authorization}, TypedHeader};
use serde::{Deserialize, Serialize};
use bitflags::bitflags;

use crate::app::{app::App, database::Database};

use super::error::{AppError, AuthError};


#[derive(Deserialize, Serialize, Debug)]
pub struct Token {
    pub token: String,
    pub owner: i64,
    pub permissions: TokenPermissions,
}

impl Token {
    pub fn new(token: String, owner: i64, permissions: TokenPermissions) -> Token {
        Token { token, owner, permissions }
    }

    pub fn permissions(&mut self, permissions: TokenPermissions) {
        self.permissions = permissions;
    }

    pub async fn fetch(db: &Database, token: String) -> Result<Option<Token>, AppError> {
        Ok(sqlx::query_as!(
            Token,
            "SELECT token, owner, permissions FROM tokens WHERE token = $1",
            token
        ).fetch_optional(db.pool()).await?)
    }

    pub async fn fetch_all(db: &Database, owner: i64) -> Result<Vec<Token>, AppError> {
        Ok(sqlx::query_as!(
            Token,
            "SELECT token, owner, permissions FROM tokens WHERE owner = $1",
            owner
        ).fetch_all(db.pool()).await?)
    }

    pub async fn update(&self, db: &Database) -> Result<(), AppError> {
        let perms: i64 = self.permissions.into();
        sqlx::query_as!(
            Token,
            "INSERT INTO tokens(token, owner, permissions) VALUES ($1, $2, $3) ON CONFLICT (token) DO UPDATE SET permissions = $3",
            self.token,
            self.owner,
            perms
        ).execute(db.pool()).await?;

        Ok(())
    }

    pub async fn delete(db: &Database, token: String) -> Result<(), AppError> {
        sqlx::query!(
            "DELETE FROM tokens WHERE token = $1",
            token,
        ).execute(db.pool()).await?;

        Ok(())
    }

    pub async fn delete_all(db: &Database, owner: i64) -> Result<(), AppError> {
        sqlx::query!(
            "DELETE FROM tokens WHERE owner = $1",
            owner,
        ).execute(db.pool()).await?;

        Ok(())
    }
}

#[async_trait::async_trait]
impl FromRequestParts<App> for Token {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &App) -> Result<Self, Self::Rejection> {
        let TypedHeader(Authorization(bearer)) = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await
            .map_err(|_| AuthError::MissingCredentials)?;

        let token_info = Token::fetch(&state.database, bearer.token().to_string()).await?;
        
        if let Some(token) = token_info {
            return Ok(token);
        } else {
            return Err(AppError::Authentication(AuthError::InvalidToken))
        }
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy)]
    /// Token Permissions.
    ///
    /// The permissions of the token.
    pub struct TokenPermissions: u32 {
        /// Create posts.
        const CreatePost = 1 << 0; // 1
        /// Edit posts.
        /// 
        /// > These must be owned by the token that created them.
        const EditPost = 1 << 1; // 2
        /// Delete posts.
        /// 
        /// > These must be owned by the token that created them.
        const DeletePost = 1 << 2; // 4
        /// Edit all posts.
        /// 
        /// > These can be owned by you, or others.
        const EditAllPosts = 1 << 3; // 8
        /// Delete all posts.
        /// 
        /// > These can be owned by you, or others.
        const DeleteAllPosts = 1 << 4; // 16
    }
}

impl From<i64> for TokenPermissions {
    fn from(value: i64) -> Self {
        TokenPermissions::from_bits(value as u32).unwrap_or_else(|| {
            panic!(
                "The TokenPermissions could not be converted from i64 {}",
                value
            )
        })
    }
}

impl From<TokenPermissions> for i64 {
    fn from(value: TokenPermissions) -> Self {
        value.bits() as i64
    }
}

impl Default for TokenPermissions {
    fn default() -> Self {
        Self::empty()
    }
}

impl Serialize for TokenPermissions {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u32(self.bits())
    }
}

impl<'de> Deserialize<'de> for TokenPermissions {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let flags = u32::deserialize(deserializer)?;
        Ok(Self::from_bits(flags).unwrap_or_default())
    }
}

/// Generate Token.
/// 
/// Generates a token of a certain length.
/// 
/// * `length` - The length of the token to generate.
pub fn generate_token(length: usize) -> Result<String, getrandom::Error> {
    let mut buffer: Vec<u8> = vec![0; length];
    //let mut buffer: [u8; length] = [0; length];
    
    getrandom::fill(&mut buffer)?;

    let ascii = String::from("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ");

    Ok(buffer.iter() // This does something?
        .map(|x| ascii.as_bytes()[(*x as usize) % ascii.len()] as char) // This maps the ascii table to the buffer
        .collect::<String>()) // This picks up the shit and slaps it in a string
}