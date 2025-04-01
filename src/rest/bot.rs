use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, patch, post},
};
use serde::{Deserialize, Serialize};

use crate::{
    app::application::App,
    models::{
        account::{Bot, BotPermissions, Token, UserSession, generate_token, validate_unique_name},
        error::AppError,
        snowflake::Snowflake,
    },
};

pub fn generate_router() -> Router<App> {
    Router::new()
        .route("/bots/{bot_id}", get(get_bot))
        .route("/bots", get(get_bots))
        .route("/bots/all", get(get_all_bots))
        .route("/bots", post(post_bot))
        .route("/bots/{bot_id}/token/reset", post(post_bot_reset_token))
        .route("/bots/{bot_id}", patch(patch_bot))
        .route("/bots/{bot_id}", delete(delete_bot))
        .route("/bots", delete(delete_bots))
        .route("/bots/all", delete(delete_all_bots))
        .layer(DefaultBodyLimit::disable())
}

async fn get_bot(
    State(app): State<App>,
    Path(bot_id): Path<Snowflake>,
    token: Token<UserSession>,
) -> Result<Response, AppError> {
    let bot = Bot::fetch(&app.database, bot_id)
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
    Path(bot_id): Path<Snowflake>,
    token: Token<UserSession>,
) -> Result<Response, AppError> {
    let mut bot = Bot::fetch(&app.database, bot_id)
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
    Path(bot_id): Path<Snowflake>,
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

    let mut bot = Bot::fetch(&app.database, bot_id)
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
    Path(bot_id): Path<Snowflake>,
    token: Token<UserSession>,
) -> Result<Response, AppError> {
    Bot::delete_with_owner(&app.database, bot_id, token.authentication.id).await?;

    Ok(StatusCode::NO_CONTENT.into_response())
}

async fn delete_bots(
    State(app): State<App>,
    token: Token<UserSession>,
    Json(body): Json<DeleteBotsBody>,
) -> Result<Response, AppError> {
    for id in body.ids {
        Bot::delete_with_owner(&app.database, id, token.authentication.id).await?;
    }

    Ok(StatusCode::NOT_IMPLEMENTED.into_response())
}

async fn delete_all_bots(
    State(app): State<App>,
    token: Token<UserSession>,
) -> Result<Response, AppError> {
    Bot::delete_all(&app.database, token.authentication.id).await?;

    Ok(StatusCode::NOT_IMPLEMENTED.into_response())
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
pub struct PatchBotBody {
    /// The name to update the bot with.
    #[serde(default)]
    pub name: Option<String>,
    /// The permissions to update the bot with.
    #[serde(default)]
    pub permissions: Option<BotPermissions>,
}

#[derive(Deserialize, Serialize)]
pub struct DeleteBotsBody {
    /// The ids of the bots to delete.
    ids: Vec<Snowflake>,
}
