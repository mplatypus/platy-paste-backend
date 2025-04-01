use std::marker::PhantomData;

use axum::{
    extract::FromRequestParts, http::{request::Parts, HeaderValue}, RequestPartsExt
};
use axum_extra::{
    TypedHeader,
    headers::{
        Authorization,
        authorization::{Bearer, Credentials},
    },
};
use bitflags::bitflags;
use bytes::Bytes;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use time::OffsetDateTime;

use crate::app::{application::App, database::Database};

use super::{
    error::{AppError, AuthError},
    snowflake::Snowflake,
};

// User

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct User {
    pub id: Snowflake,
    pub name: String,
    #[serde(skip_serializing)]
    pub email: String,
    pub permissions: UserPermissions,
}

impl User {
    pub const fn new(id: Snowflake, name: String, email: String, permissions: UserPermissions) -> Self {
        Self {
            id,
            name,
            email,
            permissions,
        }
    }

    pub fn name(&mut self, name: String) {
        self.name = name;
    }

    pub fn permissions(&mut self, permissions: UserPermissions) {
        self.permissions = permissions;
    }

    pub async fn fetch(db: &Database, id: Snowflake) -> Result<Option<Self>, AppError> {
        let id: i64 = id.into();
        Ok(sqlx::query_as!(
            Self,
            "SELECT id, name, email, permissions FROM users WHERE id = $1",
            id
        )
        .fetch_optional(db.pool())
        .await?)
    }

    pub async fn fetch_with_name(db: &Database, name: String) -> Result<Option<Self>, AppError> {
        Ok(sqlx::query_as!(
            Self,
            "SELECT id, name, email, permissions FROM users WHERE name = $1",
            name
        )
        .fetch_optional(db.pool())
        .await?)
    }

    pub async fn update(&self, db: &Database) -> Result<(), AppError> {
        let id: i64 = self.id.into();
        let permissions: i64 = self.permissions.into();

        sqlx::query!(
            "INSERT INTO users(id, name, email, permissions) VALUES ($1, $2, $3, $4) ON CONFLICT (id) DO UPDATE SET name = $2, email = $3, permissions = $4",
            id,
            self.name,
            self.email,
            permissions
        ).execute(db.pool()).await?;

        Ok(())
    }

    pub async fn delete(db: &Database, id: Snowflake) -> Result<(), AppError> {
        let id: i64 = id.into();
        sqlx::query!("DELETE FROM users WHERE id = $1", id,)
            .execute(db.pool())
            .await?;

        Ok(())
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct UserPermissions: u64 {
        /// Can create pastes. FIXME: This permission might need removing...
        const CreatePaste = 1 << 0;
        /// Can edit pastes.
        const EditPaste = 1 << 1;
        /// Can delete pastes.
        const DeletePaste = 1 << 2;
        /// Can edit others pastes.
        /// 
        /// **note:** This is a dangerous permission to enable.
        const EditOtherPaste = 1 << 5;
        /// Can delete others pastes.
        /// 
        /// **note:** This is a dangerous permission to enable.
        const DeleteOtherPaste = 1 << 6;
        /// Can edit account.
        const EditAccount = 1 << 10;
        /// Can delete account.
        const DeleteAccount = 1 << 11;
        /// Can create accounts.
        /// 
        /// **note:** This is a dangerous permission to enable.
        const CreateOtherAccount = 1 << 15;
        /// Can edit others accounts.
        /// 
        /// **note:** This is a dangerous permission to enable.
        const EditOtherAccount = 1 << 16;
        /// Can delete other accounts.
        /// 
        /// **note:** This is a dangerous permission to enable.
        const DeleteOtherAccount = 1 << 17;
        /// Can fetch bots.
        const FetchBot = 1 << 20;
        /// Can create bots.
        const CreateBot = 1 << 21;
        /// Can edit bots.
        const EditBot = 1 << 22;
        /// Can delete bots.
        const DeleteBot = 1 << 23;
        /// Can fetch others bots.
        /// 
        /// **note:** This is a dangerous permission to enable.
        const FetchOtherBot = 1 << 25;
        /// Can edit others bots.
        /// 
        /// **note:** This is a dangerous permission to enable.
        const EditOtherBot = 1 << 26;
        /// Can delete others bots.
        /// 
        /// **note:** This is a dangerous permission to enable.
        const DeleteOtherBot = 1 << 27;
    }
}

impl From<i64> for UserPermissions {
    fn from(value: i64) -> Self {
        Self::from_bits(value as u64).unwrap_or_else(|| {
            panic!("The UserPermissions could not be converted from i64 {value}")
        })
    }
}

impl From<UserPermissions> for i64 {
    fn from(value: UserPermissions) -> Self {
        value.bits() as Self
    }
}

impl Default for UserPermissions {
    fn default() -> Self {
        Self::empty()
    }
}

impl Serialize for UserPermissions {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u64(self.bits())
    }
}

impl<'de> Deserialize<'de> for UserPermissions {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let flags = u64::deserialize(deserializer)?;
        Ok(Self::from_bits(flags).unwrap_or_default())
    }
}

#[derive(FromRow, Debug, Clone)]
pub struct UserSecret {
    /// The owner ID of the secret.
    pub id: Snowflake,
    /// The password of the user.
    pub password: SecretString
}

impl UserSecret {
    pub const fn new(id: Snowflake, password: SecretString) -> Self {
        // FIXME: Make sure to encrypt the password.
        Self {
            id,
            password
        }
    }

    pub fn password(&mut self, password: SecretString) {
        // FIXME: Make sure to encrypt the password.
        self.password = password;
    }

    pub async fn fetch(db: &Database, id: Snowflake) -> Result<Option<Self>, AppError> {
        let id: i64 = id.into();
        let query = sqlx::query!(
                "SELECT id, password FROM user_secrets WHERE id = $1",
                id
            )
            .fetch_optional(db.pool())
            .await?;
        
        if let Some(q) = query {
            return Ok(Some(Self::new(
                q.id.into(),
                q.password.into()
            )));
        }

        Ok(None)
    }

    pub async fn update(&self, db: &Database) -> Result<(), AppError> {
        let id: i64 = self.id.into();

        sqlx::query!(
            "INSERT INTO user_secrets(id, password) VALUES ($1, $2) ON CONFLICT (id) DO UPDATE SET password = $2",
            id,
            self.password.expose_secret()
        ).execute(db.pool()).await?;

        Ok(())
    }

    pub async fn delete(db: &Database, id: Snowflake) -> Result<(), AppError> {
        let id: i64 = id.into();
        sqlx::query!("DELETE FROM user_secrets WHERE id = $1", id,)
            .execute(db.pool())
            .await?;

        Ok(())
    }
}

// User Session

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct UserSession {
    /// The token of the session.
    pub token: String,
    /// The owner ID of the session.
    pub id: Snowflake,
    /// Expiry. Must be UTC.
    pub expiry: OffsetDateTime,
}

impl UserSession {
    pub const fn new(token: String, id: Snowflake, expiry: OffsetDateTime) -> Self {
        Self { token, id, expiry }
    }

    pub async fn fetch(db: &Database, token: String) -> Result<Option<Self>, AppError> {
        Ok(sqlx::query_as!(
            Self,
            "SELECT token, id, expiry FROM user_sessions WHERE token = $1",
            token
        )
        .fetch_optional(db.pool())
        .await?)
    }

    pub async fn fetch_all(db: &Database, id: Snowflake) -> Result<Vec<Self>, AppError> {
        let id: i64 = id.into();
        Ok(sqlx::query_as!(
            Self,
            "SELECT token, id, expiry FROM user_sessions WHERE id = $1",
            id
        )
        .fetch_all(db.pool())
        .await?)
    }

    pub async fn fetch_owner(&self, db: &Database) -> Result<Option<User>, AppError> {
        User::fetch(db, self.id).await
    }

    pub async fn update(&self, db: &Database) -> Result<(), AppError> {
        let id: i64 = self.id.into();

        sqlx::query!(
            "INSERT INTO user_sessions(token, id, expiry) VALUES ($1, $2, $3) ON CONFLICT (token) DO NOTHING",
            self.token,
            id,
            self.expiry
        ).execute(db.pool()).await?;

        Ok(())
    }

    pub async fn delete(db: &Database, token: String) -> Result<(), AppError> {
        sqlx::query!("DELETE FROM user_sessions WHERE token = $1", token,)
            .execute(db.pool())
            .await?;

        Ok(())
    }

    pub async fn delete_all(db: &Database, id: Snowflake) -> Result<(), AppError> {
        let id: i64 = id.into();
        sqlx::query!("DELETE FROM user_sessions WHERE id = $1", id,)
            .execute(db.pool())
            .await?;

        Ok(())
    }
}

// Bot

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Bot {
    pub id: Snowflake,
    pub name: String,
    pub owner_id: Snowflake,
    #[serde(skip_serializing)] // FIXME: This needs changing for when the user creates a new account, either a custom payload, or this does get serialized.
    pub token: String,
    pub permissions: BotPermissions,
}

impl Bot {
    pub const fn new(
        id: Snowflake,
        name: String,
        owner_id: Snowflake,
        token: String,
        permissions: BotPermissions,
    ) -> Self {
        Self {
            id,
            name,
            owner_id,
            token,
            permissions,
        }
    }

    pub fn name(&mut self, name: String) {
        self.name = name;
    }

    pub fn token(&mut self, token: String) {
        self.token = token;
    }

    pub fn permissions(&mut self, permissions: BotPermissions) {
        self.permissions = permissions;
    }

    pub async fn fetch(db: &Database, id: Snowflake) -> Result<Option<Self>, AppError> {
        let id: i64 = id.into();
        Ok(sqlx::query_as!(
            Self,
            "SELECT id, name, owner_id, token, permissions FROM bots WHERE id = $1",
            id
        )
        .fetch_optional(db.pool())
        .await?)
    }

    pub async fn fetch_with_name(db: &Database, name: String) -> Result<Option<Self>, AppError> {
        Ok(sqlx::query_as!(
            Self,
            "SELECT id, name, owner_id, token, permissions FROM bots WHERE name = $1",
            name
        )
        .fetch_optional(db.pool())
        .await?)
    }

    pub async fn fetch_with_token(db: &Database, token: String) -> Result<Option<Self>, AppError> {
        Ok(sqlx::query_as!(
            Self,
            "SELECT id, name, owner_id, token, permissions FROM bots WHERE token = $1",
            token
        )
        .fetch_optional(db.pool())
        .await?)
    }

    pub async fn fetch_all(db: &Database, owner_id: Snowflake) -> Result<Vec<Self>, AppError> {
        let owner_id: i64 = owner_id.into();
        Ok(sqlx::query_as!(
            Self,
            "SELECT id, name, owner_id, token, permissions FROM bots WHERE owner_id = $1",
            owner_id
        )
        .fetch_all(db.pool())
        .await?)
    }

    pub async fn update(&self, db: &Database) -> Result<(), AppError> {
        let id: i64 = self.id.into();
        let owner_id: i64 = self.owner_id.into();
        let permissions: i64 = self.permissions.into();

        sqlx::query!(
            "INSERT INTO bots(id, name, owner_id, token, permissions) VALUES ($1, $2, $3, $4, $5) ON CONFLICT (token) DO UPDATE SET name = $2, token = $4, permissions = $5",
            id,
            self.name,
            owner_id,
            self.token,
            permissions
        ).execute(db.pool()).await?;

        Ok(())
    }

    pub async fn delete(db: &Database, id: Snowflake) -> Result<(), AppError> {
        let id: i64 = id.into();
        sqlx::query!("DELETE FROM bots WHERE id = $1", id,)
            .execute(db.pool())
            .await?;

        Ok(())
    }

    pub async fn delete_with_owner(
        db: &Database,
        id: Snowflake,
        owner_id: Snowflake,
    ) -> Result<(), AppError> {
        let id: i64 = id.into();
        let owner_id: i64 = owner_id.into();
        sqlx::query!(
            "DELETE FROM bots WHERE id = $1 AND owner_id = $2",
            id,
            owner_id,
        )
        .execute(db.pool())
        .await?;

        Ok(())
    }

    pub async fn delete_all(db: &Database, owner_id: Snowflake) -> Result<(), AppError> {
        let owner_id: i64 = owner_id.into();
        sqlx::query!("DELETE FROM bots WHERE owner_id = $1", owner_id,)
            .execute(db.pool())
            .await?;

        Ok(())
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct BotPermissions: u64 {
        /// Can fetch pastes.
        const FetchPaste = 1 << 0;
        /// Can create pastes.
        const CreatePaste = 1 << 1;
        /// Can edit pastes.
        const EditPaste = 1 << 2;
        /// Can delete pastes.
        const DeletePaste = 1 << 3;
        /// Can create pastes on owners behalf.
        /// 
        /// **note:** This is a dangerous permission to enable.
        const CreateOwnerPaste = 1 << 10; // FIXME: I am not sure I want to keep this. Maybe if I do, add a boolean to pastes of whether a bot made/edited them or not.
        /// Can edit pastes on owners behalf.
        /// 
        /// **note:** This is a dangerous permission to enable.
        const EditOwnerPaste = 1 << 11; // FIXME: I am not sure I want to keep this.
        /// Can delete pastes on owners behalf.
        /// 
        /// **note:** This is a dangerous permission to enable.
        const DeleteOwnerPaste = 1 << 12; // FIXME: I am not sure I want to keep this.
    }
}

impl From<i64> for BotPermissions {
    fn from(value: i64) -> Self {
        Self::from_bits(value as u64)
            .unwrap_or_else(|| panic!("The BotPermissions could not be converted from i64 {value}"))
    }
}

impl From<BotPermissions> for i64 {
    fn from(value: BotPermissions) -> Self {
        value.bits() as Self
    }
}

impl Default for BotPermissions {
    fn default() -> Self {
        Self::empty()
    }
}

impl Serialize for BotPermissions {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u64(self.bits())
    }
}

impl<'de> Deserialize<'de> for BotPermissions {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let flags = u64::deserialize(deserializer)?;
        Ok(Self::from_bits(flags).unwrap_or_default())
    }
}

#[derive(Debug, Clone)]
pub enum Required {
    UserSession(UserSession),
    Bot(Bot),
}

/// Any Required.
///
/// Any of the token options, but one is required.
pub struct AnyRequired(Required);

impl AnyRequired {
    pub const fn new(required: Required) -> Self {
        Self(required)
    }

    pub const fn required(&self) -> &Required {
        &self.0
    }
}

/// Any Optional.
///
/// Any of the token options, or no token is required.
pub struct AnyOptional(Option<Required>);

impl AnyOptional {
    pub const fn new(optional: Option<Required>) -> Self {
        Self(optional)
    }

    pub const fn required(&self) -> &Option<Required> {
        &self.0
    }
}

pub trait AuthMode {
    async fn validate(parts: &mut Parts, state: &App) -> Result<Option<Self>, AppError>
    where
        Self: std::marker::Sized;
}

impl AuthMode for UserSession {
    async fn validate(parts: &mut Parts, state: &App) -> Result<Option<Self>, AppError> {
        #[allow(clippy::manual_let_else)]
        let TypedHeader(Authorization(bearer)) =
            match parts.extract::<TypedHeader<Authorization<Bearer>>>().await {
                // TODO: This might not be the best method of handling this.
                Ok(bearer) => bearer,
                Err(_) => return Ok(None),
            };

        let user_session = Self::fetch(&state.database, bearer.token().to_string()).await?;

        if let Some(session) = user_session {
            let current_time = OffsetDateTime::now_utc();

            if session.expiry <= current_time {
                Self::delete(&state.database, bearer.token().to_string()).await?;
                return Err(AppError::Authentication(AuthError::ExpiredToken));
            }

            return Ok(Some(session));
        }

        Err(AppError::Authentication(AuthError::InvalidToken))
    }
}

impl AuthMode for Bot {
    async fn validate(parts: &mut Parts, state: &App) -> Result<Option<Self>, AppError> {
        #[allow(clippy::manual_let_else)]
        let TypedHeader(Authorization(bearer)) = match parts
            .extract::<TypedHeader<Authorization<BotBearer>>>()
            .await
        {
            // TODO: This might not be the best method of handling this.
            Ok(bearer) => bearer,
            Err(_) => return Ok(None),
        };

        let bot = Self::fetch_with_token(&state.database, bearer.token().to_string()).await?;

        if let Some(b) = bot {
            return Ok(Some(b));
        }

        Err(AppError::Authentication(AuthError::InvalidToken))
    }
}

impl AuthMode for AnyRequired {
    async fn validate(parts: &mut Parts, state: &App) -> Result<Option<Self>, AppError> {
        let user_session = UserSession::validate(parts, state).await?;

        if let Some(session) = user_session {
            return Ok(Some(Self::new(Required::UserSession(session))));
        }

        let bot = Bot::validate(parts, state).await?;

        if let Some(b) = bot {
            return Ok(Some(Self::new(Required::Bot(b))));
        }

        Err(AppError::Authentication(AuthError::MissingCredentials))
    }
}

impl AuthMode for AnyOptional {
    async fn validate(parts: &mut Parts, state: &App) -> Result<Option<Self>, AppError> {
        let user_session = UserSession::validate(parts, state).await?;

        if let Some(session) = user_session {
            return Ok(Some(Self::new(Some(Required::UserSession(session)))));
        }

        let bot = Bot::validate(parts, state).await?;

        if let Some(b) = bot {
            return Ok(Some(Self::new(Some(Required::Bot(b)))));
        }

        Ok(Some(Self::new(None)))
    }
}

pub struct Token<A: AuthMode> {
    pub authentication: A,
    _marker: PhantomData<A>,
}

impl<A: AuthMode> Token<A> {
    pub const fn new(authentication: A) -> Self {
        Self {
            authentication,
            _marker: PhantomData,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct BotBearer(pub String);

impl BotBearer {
    pub fn token(&self) -> &str {
        self.0.as_str()["Bot ".len()..].trim_start()
    }
}

impl Credentials for BotBearer {
    const SCHEME: &'static str = "Bot";

    fn decode(value: &HeaderValue) -> Option<Self> {
        debug_assert!(
            value.as_bytes()[..Self::SCHEME.len()].eq_ignore_ascii_case(Self::SCHEME.as_bytes()),
            "HeaderValue to decode should start with \"Bot ..\", received = {value:?}",
        );

        let token = String::from_utf8(value.as_bytes().to_vec()).ok()?;
        println!("Token: {token:?}");

        Some(Self(token))
    }

    fn encode(&self) -> HeaderValue {
        let mut encoded = String::from(Self::SCHEME);
        encoded.push_str(&self.0);

        let bytes = Bytes::from(encoded);
        HeaderValue::from_maybe_shared(bytes)
            .expect("Encoding should always result in a valid HeaderValue")
    }
}

impl FromRequestParts<App> for Token<UserSession> {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &App) -> Result<Self, Self::Rejection> {
        Ok(Self::new(
            UserSession::validate(parts, state)
                .await?
                .ok_or(AuthError::InvalidToken)?,
        ))
    }
}

impl FromRequestParts<App> for Token<Bot> {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &App) -> Result<Self, Self::Rejection> {
        Ok(Self::new(
            Bot::validate(parts, state)
                .await?
                .ok_or(AuthError::InvalidToken)?,
        ))
    }
}

impl FromRequestParts<App> for Token<AnyRequired> {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &App) -> Result<Self, Self::Rejection> {
        Ok(Self::new(
            AnyRequired::validate(parts, state)
                .await?
                .ok_or(AuthError::InvalidToken)?,
        ))
    }
}

impl FromRequestParts<App> for Token<AnyOptional> {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &App) -> Result<Self, Self::Rejection> {
        Ok(Self::new(
            AnyOptional::validate(parts, state)
                .await?
                .ok_or(AuthError::InvalidToken)?,
        ))
    }
}
