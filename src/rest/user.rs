use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, patch, post},
};

use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use time::{Duration, OffsetDateTime};

use crate::{
    app::application::App,
    models::{
        account::{
            Token, User, UserPermissions, UserSecret, UserSession, generate_token,
            validate_unique_name,
        },
        error::{AppError, AuthError},
        snowflake::Snowflake,
    },
};

pub fn generate_router() -> Router<App> {
    Router::new()
        .route("/users/{user_id}", get(get_user))
        .route("/users", get(get_users))
        .route("/users", post(post_user))
        .route("/users/auth", post(post_user_auth))
        .route("/users/{user_id}", patch(patch_user))
        .route("/users/{user_id}", delete(delete_user))
        .route("/users", delete(delete_users))
        .route(
            "/users/{user_id}/sessions/{session_token}",
            get(get_user_session),
        )
        .route("/users/{user_id}/sessions", get(get_user_sessions))
        .route(
            "/users/{user_id}/sessions/{session_token}",
            delete(delete_user_session),
        )
        .route("/users/{user_id}/sessions", delete(delete_user_sessions))
        .route(
            "/users/{user_id}/sessions/all",
            delete(delete_all_user_sessions),
        )
        .layer(DefaultBodyLimit::disable())
}

async fn get_user(
    State(app): State<App>,
    Path(user_id): Path<Snowflake>,
) -> Result<Response, AppError> {
    let user = User::fetch(&app.database, user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found.".to_string()))?;

    Ok((StatusCode::OK, Json(user)).into_response())
}

async fn get_users(
    State(_app): State<App>,
    Json(_body): Json<GetUsersBody>,
) -> Result<Response, AppError> {
    todo!("Implement me!"); // FIXME: Implement this function.
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

async fn post_user_auth(
    State(app): State<App>,
    Json(body): Json<PostUserAuthBody>,
) -> Result<Response, AppError> {
    let user = User::fetch_with_email(&app.database, body.email)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found.".to_string()))?;

    let user_secret = UserSecret::fetch(&app.database, user.id)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found.".to_string()))?;

    if body.password.expose_secret() != user_secret.password.expose_secret() {
        return Err(AppError::Authentication(AuthError::NotFound(
            "User not found.".to_string(),
        ))); // FIXME: This is needs a proper error.
    }

    let expiry = OffsetDateTime::now_utc().saturating_add(Duration::days(28)); // FIXME: This should be settable via the environment.

    let user_session = UserSession::new(generate_token(user_secret.id)?, user_secret.id, expiry);

    user_session.update(&app.database).await?;

    Ok((StatusCode::OK, Json(user_session)).into_response())
}

async fn patch_user(
    State(app): State<App>,
    Path(user_id): Path<Snowflake>,
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

    let mut user = User::fetch(&app.database, user_id)
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
    Path(user_id): Path<Snowflake>,
    _token: Token<UserSession>,
) -> Result<Response, AppError> {
    // TODO: Need to authenticate that the token provided is allowed to delete accounts, or it is the users account.

    User::delete(&app.database, user_id).await?;

    Ok(StatusCode::NO_CONTENT.into_response())
}

async fn delete_users(
    State(_app): State<App>,
    Json(_body): Json<DeleteUsersBody>,
) -> Result<Response, AppError> {
    todo!("Implement me!"); // FIXME: Implement this function.
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

async fn get_user_sessions(
    State(_app): State<App>,
    Query(_query): Query<GetUserSessionQuery>,
) -> Result<Response, AppError> {
    todo!("Implement me!"); // FIXME: Implement this function.
}

async fn delete_user_session(
    State(app): State<App>,
    Path((user_id, session_token)): Path<(Snowflake, String)>,
    token: Token<UserSession>,
) -> Result<Response, AppError> {
    if token.authentication.token != session_token && token.authentication.id != user_id {
        // FIXME: This might not be sound logic.

        return Err(AppError::Authentication(AuthError::MissingCredentials)); // FIXME: This needs a different error.
    }

    UserSession::delete(&app.database, session_token).await?;

    Ok(StatusCode::NO_CONTENT.into_response())
}

async fn delete_user_sessions(
    State(_app): State<App>,
    Json(_body): Json<DeleteUserSessionsBody>,
) -> Result<Response, AppError> {
    todo!("Implement me!"); // FIXME: Implement this function.
}

async fn delete_all_user_sessions(
    State(app): State<App>,
    Path(user_id): Path<Snowflake>,
    token: Token<UserSession>,
) -> Result<Response, AppError> {
    if token.authentication.id == user_id {
        UserSession::delete_all(&app.database, user_id).await?;
    }

    // FIXME: Check that the token has the permission to delete other tokens.

    Ok(StatusCode::NO_CONTENT.into_response())
}

#[derive(Deserialize)]
pub struct GetUsersBody {
    /// The ID for the paste to retrieve.
    #[serde(default)]
    pub ids: Vec<Snowflake>,
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

#[derive(Deserialize)]
pub struct PostUserAuthBody {
    /// The email to give the user.
    pub email: String,
    /// The password ot use for the account.
    pub password: SecretString,
}

#[derive(Deserialize)]
pub struct PatchUserBody {
    /// The name to update the user with.
    #[serde(default)]
    pub name: Option<String>,
    /// The permissions to update the user with.
    #[serde(default)]
    pub permissions: Option<UserPermissions>,
}

#[derive(Deserialize)]
pub struct DeleteUsersBody {
    /// The user ID's to delete.
    pub ids: Vec<Snowflake>,
}

#[derive(Deserialize)]
pub struct GetUserSessionQuery {
    /// The token to get.
    token: String,
}

#[derive(Deserialize)]
pub struct PostUserSessionBody {
    /// The ID of the user.
    pub id: Snowflake,
    /// The password of the user to authenticate with.
    pub password: String,
}

#[derive(Deserialize)]
pub struct DeleteUserSessionsBody {
    /// The token's to delete.
    pub tokens: Vec<String>,
}
