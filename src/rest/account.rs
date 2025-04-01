use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, patch, post},
};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};

use crate::{
    app::{application::App, database::Database},
    models::{
        account::{Bot, BotPermissions, Token, User, UserPermissions, UserSecret, UserSession},
        error::{AppError, AuthError},
        snowflake::Snowflake,
    },
};

pub fn generate_router() -> Router<App> {
    Router::new()
        .route("/user", get(get_user))
        .route("/user", post(post_user))
        .route("/user", patch(patch_user))
        .route("/user", delete(delete_user))
        .route("/user/session", get(get_user_session))
        .route("/user/session", post(post_user_session))
        .route("/user/session", delete(delete_user_session))
        .route("/user/session/all", delete(delete_all_user_sessions))
        .route("/user/bot", get(get_bot))
        .route("/user/bots", get(get_bots))
        .route("/user/bot/all", get(get_all_bots))
        .route("/user/bot", post(post_bot))
        .route("/user/bot/token/reset", post(post_bot_reset_token))
        .route("/user/bot", patch(patch_bot))
        .route("/user/bot", delete(delete_bot))
        .route("/user/bot/all", delete(delete_all_bots))
        .layer(DefaultBodyLimit::disable())
}

/// Validate unique name.
///
/// validate that a name (provided) is unique.
async fn validate_unique_name(db: &Database, name: String) -> Result<(), AppError> {
    let user = User::fetch_with_name(db, name.clone()).await?;

    if user.is_some() {
        return Err(AppError::ConflictError(format!(
            "Name '{name}' already exists."
        )));
    }

    // Split into two separate functions to reduce DB calls.
    let bot = Bot::fetch_with_name(db, name.clone()).await?;

    if bot.is_some() {
        return Err(AppError::ConflictError(format!(
            "Name '{name}' already exists."
        )));
    }

    Ok(())
}

async fn get_user(
    State(app): State<App>,
    Query(query): Query<GetUserQuery>,
) -> Result<Response, AppError> {
    if query.user_id.is_none() && query.user_name.is_none() {
        return Err(AppError::BadRequest(
            "`user_id` or `user_name` must be provided.".to_string(),
        ));
    }

    if query.user_id.is_some() && query.user_name.is_some() {
        return Err(AppError::BadRequest(
            "both `user_id` and `user_name` must not be provided in the same request.".to_string(),
        ));
    }

    let user = {
        if let Some(user_id) = query.user_id {
            User::fetch(&app.database, user_id)
                .await?
                .ok_or_else(|| AppError::NotFound("User not found.".to_string()))?
        } else if let Some(user_name) = query.user_name {
            User::fetch_with_name(&app.database, user_name)
                .await?
                .ok_or_else(|| AppError::NotFound("User not found.".to_string()))?
        } else {
            panic!("Both user_id and user_name were missing, when one was validated."); // FIXME: This could be done better.
        }
    };

    Ok((StatusCode::OK, Json(user)).into_response())
}

async fn post_user(
    State(app): State<App>,
    Json(body): Json<PostUserBody>,
) -> Result<Response, AppError> {
    // TODO: Need to authenticate that the token provided is allowed to make accounts.
    // TODO: Validate that the user created, is allowed to give the permissions to the below user.
    validate_unique_name(&app.database, body.name.clone()).await?;

    #[allow(clippy::option_if_let_else)] // FIXME: This needs to be looked at.
    let permissions = if let Some(p) = body.permissions {
        p // This needs to restrict to a certain set of permissions, either ignoring the permissions, or erroring out.
    } else {
        UserPermissions::CreatePaste
            | UserPermissions::EditPaste
            | UserPermissions::DeletePaste
            | UserPermissions::EditAccount
            | UserPermissions::DeleteAccount
            | UserPermissions::FetchBot
            | UserPermissions::CreateBot
            | UserPermissions::EditBot
            | UserPermissions::DeleteBot
    };

    let user = User::new(Snowflake::generate()?, body.name, body.email, permissions);

    user.update(&app.database).await?;

    let user_secret = UserSecret::new(user.id, body.password);

    user_secret.update(&app.database).await?;

    Ok((StatusCode::OK, Json(user)).into_response())
}

async fn patch_user(
    State(app): State<App>,
    Query(query): Query<PatchUserQuery>,
    _token: Token<UserSession>,
    Json(body): Json<PatchUserBody>,
) -> Result<Response, AppError> {
    // TODO: Need to authenticate that the token provided is allowed to make accounts.
    // TODO: Validate that the user created, is allowed to give the permissions to the below user.
    if body.name.is_none() && body.permissions.is_none() {
        return Err(AppError::BadRequest(
            "One of `name` or `permissions` is required.".to_string(),
        ));
    }

    let mut user = User::fetch(&app.database, query.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found.".to_string()))?;

    if let Some(name) = body.name {
        validate_unique_name(&app.database, name.clone()).await?;
        user.name(name);
    }

    if let Some(permissions) = body.permissions {
        user.permissions(permissions);
    }

    user.update(&app.database).await?;

    Ok((StatusCode::OK, Json(user)).into_response())
}

async fn delete_user(
    State(app): State<App>,
    Query(query): Query<DeleteUserQuery>,
    _token: Token<UserSession>,
) -> Result<Response, AppError> {
    // TODO: Need to authenticate that the token provided is allowed to delete accounts, or it is the users account.

    User::delete(&app.database, query.user_id).await?;

    Ok(StatusCode::NO_CONTENT.into_response())
}

async fn get_user_session(
    State(app): State<App>,
    Query(query): Query<GetUserSessionQuery>,
) -> Result<Response, AppError> {
    let user_session = UserSession::fetch(&app.database, query.token)
        .await?
        .ok_or_else(|| AppError::NotFound("User Session not found.".to_string()))?;

    let current_time = OffsetDateTime::now_utc();

    if user_session.expiry <= current_time {
        UserSession::delete(&app.database, user_session.token).await?;
        return Err(AppError::Authentication(AuthError::ExpiredToken));
    }

    Ok((StatusCode::OK, Json(user_session)).into_response())
}

async fn post_user_session(
    State(app): State<App>,
    Json(body): Json<PostUserSessionBody>,
) -> Result<Response, AppError> {
    // FIXME: This should use a secure password.
    // FIXME: This whole function might need rebuilding.
    let user_secret = UserSecret::fetch(&app.database, body.id)
        .await?
        .ok_or_else(|| AppError::NotFound("User authentication not found.".to_string()))?;

    if body.password != user_secret.password.expose_secret() {
        return Err(AppError::Authentication(AuthError::NotFound(
            "Authentication Invalid.".to_string(),
        ))); // FIXME: This is needs a proper error.
    }

    let expiry = OffsetDateTime::now_utc().saturating_add(Duration::days(28)); // FIXME: This should be settable via the environment.

    let user_session = UserSession::new(generate_token(user_secret.id)?, user_secret.id, expiry);

    user_session.update(&app.database).await?;

    Ok((StatusCode::OK, Json(user_session)).into_response())
}

async fn delete_user_session(
    State(app): State<App>,
    token: Token<UserSession>,
    Json(body): Json<DeleteUserSessionBody>,
) -> Result<Response, AppError> {
    if token.authentication.token == body.token {
        UserSession::delete(&app.database, body.token).await?;
        return Ok(StatusCode::NO_CONTENT.into_response());
    }

    let user_session = UserSession::fetch(&app.database, body.token)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found.".to_string()))?;

    if token.authentication.id != user_session.id {
        return Err(AppError::Authentication(AuthError::MissingPermissions));
    }

    UserSession::delete(&app.database, user_session.token).await?;

    Ok(StatusCode::NO_CONTENT.into_response())
}

async fn delete_all_user_sessions(
    State(app): State<App>,
    token: Token<UserSession>,
    Json(body): Json<DeleteAllUserSessionsBody>,
) -> Result<Response, AppError> {
    if token.authentication.id == body.user_id {
        UserSession::delete_all(&app.database, body.user_id).await?;
    }

    // FIXME: Check that the token has the permission to delete other tokens.

    Ok(StatusCode::NO_CONTENT.into_response())
}

async fn get_bot(
    State(app): State<App>,
    Query(query): Query<GetBotQuery>,
    token: Token<UserSession>,
) -> Result<Response, AppError> {
    let bot = Bot::fetch(&app.database, query.bot_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Bot not found.".to_string()))?;

    if token.authentication.id != bot.owner_id {
        // TODO: This error is shown instead of an Unauthorized error,
        // so that there is no way to find this endpoint to find valid bot tokens.
        // Maybe there is a better way to fix this.
        return Err(AppError::NotFound("Bot not found.".to_string()));
    }

    Ok((StatusCode::OK, Json(bot)).into_response())
}

async fn get_bots(
    State(app): State<App>,
    token: Token<UserSession>,
    Json(body): Json<GetBotsBody>,
) -> Result<Response, AppError> {
    let mut bots = Vec::new();
    for bot_id in body.bot_ids {
        let bot = Bot::fetch(&app.database, bot_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Bot not found.".to_string()))?;

        if token.authentication.id != bot.owner_id {
            // TODO: This error is shown instead of an Unauthorized error,
            // so that there is no way to find this endpoint to find valid bot tokens.
            // Maybe there is a better way to fix this.
            return Err(AppError::NotFound("Bot not found.".to_string()));
        }

        bots.push(bot);
    }

    Ok((StatusCode::OK, Json(bots)).into_response())
}

async fn get_all_bots(
    State(app): State<App>,
    token: Token<UserSession>,
) -> Result<Response, AppError> {
    let bots = Bot::fetch_all(&app.database, token.authentication.id).await?;

    Ok((StatusCode::OK, Json(bots)).into_response())
}

async fn post_bot(
    State(app): State<App>,
    token: Token<UserSession>,
    Json(body): Json<PostBotBody>,
) -> Result<Response, AppError> {
    // TODO: Need to authenticate that the token provided is allowed to make bots.
    // TODO: Validate that the bot created, is allowed to give the permissions to the bot.

    let owner_id = token.authentication.id;

    let bot = Bot::new(
        Snowflake::generate()?,
        body.name,
        owner_id,
        generate_token(owner_id)?,
        body.permissions,
    );

    bot.update(&app.database).await?;

    Ok((StatusCode::OK, Json(bot)).into_response())
}

async fn post_bot_reset_token(
    State(app): State<App>,
    Query(query): Query<PostBotResetTokenQuery>,
    token: Token<UserSession>,
) -> Result<Response, AppError> {
    let mut bot = Bot::fetch(&app.database, query.bot_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Bot not found.".to_string()))?;

    if token.authentication.id != bot.owner_id {
        return Err(AppError::NotFound("Bot not found.".to_string())); // FIXME: This might need changing.
    }

    bot.token(generate_token(bot.owner_id)?);

    bot.update(&app.database).await?;

    Ok((
        StatusCode::OK,
        Json(PostBotResetTokenResponse::from_bot(bot)),
    )
        .into_response())
}

async fn patch_bot(
    State(app): State<App>,
    Query(query): Query<PatchBotQuery>,
    token: Token<UserSession>,
    Json(body): Json<PatchBotBody>,
) -> Result<Response, AppError> {
    // TODO: Need to authenticate that the token provided is allowed to make bots.
    // TODO: Validate that the bot created, is allowed to give the permissions to the bot.

    if body.name.is_none() && body.permissions.is_none() {
        return Err(AppError::BadRequest(
            "One of `name` or `permissions` is required.".to_string(),
        ));
    }

    let mut bot = Bot::fetch(&app.database, query.bot_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Bot not found.".to_string()))?;

    if token.authentication.id != bot.owner_id {
        // TODO: This error is shown instead of an Unauthorized error,
        // so that there is no way to find this endpoint to find valid bot tokens.
        // Maybe there is a better way to fix this.
        return Err(AppError::NotFound("Bot not found.".to_string()));
    }

    if let Some(name) = body.name {
        validate_unique_name(&app.database, name.clone()).await?;
        bot.name(name);
    }

    if let Some(permissions) = body.permissions {
        bot.permissions(permissions);
    }

    bot.update(&app.database).await?;

    Ok((StatusCode::OK, Json(bot)).into_response())
}

async fn delete_bot(
    State(app): State<App>,
    Query(query): Query<DeleteBotQuery>,
    token: Token<UserSession>,
) -> Result<Response, AppError> {
    Bot::delete_with_owner(&app.database, query.bot_id, token.authentication.id).await?;

    Ok(StatusCode::NO_CONTENT.into_response())
}

async fn delete_all_bots(
    State(app): State<App>,
    token: Token<UserSession>,
) -> Result<Response, AppError> {
    Bot::delete_all(&app.database, token.authentication.id).await?;

    Ok(StatusCode::NOT_IMPLEMENTED.into_response())
}

#[derive(Deserialize)]
pub struct GetUserQuery {
    /// The ID for the paste to retrieve.
    #[serde(default)]
    user_id: Option<Snowflake>,
    /// The users name to search for.
    #[serde(default)]
    user_name: Option<String>,
}

#[derive(Deserialize)]
pub struct PostUserBody {
    /// The name to give the user.
    pub name: String,
    /// The email to give the user.
    pub email: String,
    /// The password ot use for the account.
    pub password: SecretString,
    /// The permissions to give to the user.
    #[serde(default)]
    pub permissions: Option<UserPermissions>,
}

#[derive(Serialize)]
pub struct GetOAuthUserResponse {
    /// The URL to load.
    pub url: String,
}

impl GetOAuthUserResponse {
    pub const fn new(url: String) -> Self {
        Self { url }
    }
}

#[derive(Deserialize)]
pub struct PatchUserQuery {
    /// The user ID to update.
    user_id: Snowflake,
}

#[derive(Deserialize, Serialize)]
pub struct PatchUserBody {
    /// The name to update the user with.
    #[serde(default)]
    pub name: Option<String>,
    /// The permissions to update the user with.
    #[serde(default)]
    pub permissions: Option<UserPermissions>,
}

#[derive(Deserialize, Serialize)]
pub struct DeleteUserQuery {
    /// The user ID to update.
    user_id: Snowflake,
}

#[derive(Deserialize, Serialize)]
pub struct GetUserSessionQuery {
    /// The token to fetch.
    token: String,
}

#[derive(Deserialize)]
pub struct PostUserSessionBody {
    /// The ID of the user.
    pub id: Snowflake,
    /// The password of the user to authenticate with.
    pub password: String,
}

#[derive(Deserialize, Serialize)]
pub struct DeleteUserSessionBody {
    /// The token to fetch.
    token: String,
}

#[derive(Deserialize, Serialize)]
pub struct DeleteAllUserSessionsBody {
    /// The user ID to delete all tokens from.
    user_id: Snowflake,
}

#[derive(Deserialize, Serialize)]
pub struct GetBotQuery {
    /// The id of the bot to get.
    bot_id: Snowflake,
}

#[derive(Deserialize, Serialize)]
pub struct GetBotsBody {
    /// The ids of the bots to get.
    bot_ids: Vec<Snowflake>,
}

#[derive(Deserialize, Serialize)]
pub struct PostBotBody {
    /// The bots name.
    name: String,
    /// The bots permissions.
    permissions: BotPermissions,
}

#[derive(Deserialize)]
pub struct PostBotResetTokenQuery {
    /// The bot ID to reset the token for.
    pub bot_id: Snowflake,
}

#[derive(Serialize)]
pub struct PostBotResetTokenResponse {
    pub id: Snowflake,
    pub name: String,
    pub owner_id: Snowflake,
    pub token: String,
    pub permissions: BotPermissions,
}

impl PostBotResetTokenResponse {
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

    pub fn from_bot(bot: Bot) -> Self {
        Self::new(bot.id, bot.name, bot.owner_id, bot.token, bot.permissions)
    }
}

#[derive(Deserialize, Serialize)]
pub struct PatchBotQuery {
    /// The token to update.
    bot_id: Snowflake,
}

#[derive(Deserialize, Serialize)]
pub struct PatchBotBody {
    /// The name to update the bot with.
    #[serde(default)]
    pub name: Option<String>,
    /// The permissions to update the bot with.
    #[serde(default)]
    pub permissions: Option<BotPermissions>,
}

#[derive(Deserialize, Serialize)]
pub struct DeleteBotQuery {
    /// The id of the bot to delete.
    bot_id: Snowflake,
}

pub fn generate_token(_owner_id: Snowflake) -> Result<String, AppError> {
    // FIXME: The owner ID should be added in base64, and then separated via a "." (owner ID, unique token.)
    // maybe also encrypt the time it was created into it as well?
    const TOKEN_LENGTH: usize = 25;

    let mut buffer: Vec<u8> = vec![0; TOKEN_LENGTH];

    getrandom::fill(&mut buffer).map_err(|e| AppError::NotFound(e.to_string()))?;

    let ascii = String::from("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ-");

    Ok(buffer
        .iter() // Convert to an iterator.
        .map(|x| ascii.as_bytes()[(*x as usize) % ascii.len()] as char) // This maps the ascii table to the buffer
        .collect::<String>()) // Collect the items into a string.
}
