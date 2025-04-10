use std::{sync::Arc, time::Duration};

use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Multipart, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, patch, post},
};
use regex::Regex;
use time::OffsetDateTime;
use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder};

use crate::{
    app::{application::App, config::Config},
    models::{
        authentication::{Token, generate_token},
        document::Document,
        error::{AppError, AuthError},
        paste::Paste,
        payload::{
            GetPasteQuery, PostPasteBody, PostPasteQuery, ResponseDocument,
            ResponsePaste,
        },
        snowflake::Snowflake,
    },
};

/* FIXME: Unsure if this is actually needed.
/// Supported mimes are the ones that will be supported by the website.
const SUPPORTED_MIMES: &[&str] = &[
    // Text mimes
    "text/x-asm",
    "text/x-c",
    "text/plain",
    "text/markdown",
    "text/css",
    "text/csv",
    "text/html",
    "text/x-java-source",
    "text/javascript",
    "text/x-pascal",
    "text/x-python",
    // Application mimes
    "application/json"
];
*/

/// Unsupported mimes, are ones that will be declined.
const UNSUPPORTED_MIMES: &[&str] = &["image/*", "video/*", "audio/*", "font/*", "application/pdf"];

const DEFAULT_MIME: &str = "text/plain";

pub fn generate_router(config: &Config) -> Router<App> {
    let global_limiter = GovernorLayer {
        config: Arc::new(
            GovernorConfigBuilder::default()
                .per_second(60)
                .burst_size(config.global_paste_rate_limiter())
                .period(Duration::from_secs(5))
                .use_headers()
                .finish()
                .expect("Failed to build global paste limiter."),
        ),
    };

    let get_pastes_limiter = GovernorLayer {
        config: Arc::new(
            GovernorConfigBuilder::default()
                .per_second(60)
                .burst_size(config.get_pastes_rate_limiter())
                .period(Duration::from_secs(5))
                .use_headers()
                .finish()
                .expect("Failed to build get pastes limiter."),
        ),
    };

    let get_paste_limiter = GovernorLayer {
        config: Arc::new(
            GovernorConfigBuilder::default()
                .per_second(60)
                .burst_size(config.get_paste_rate_limiter())
                .period(Duration::from_secs(5))
                .use_headers()
                .finish()
                .expect("Failed to build get paste limiter."),
        ),
    };

    let post_paste_limiter = GovernorLayer {
        config: Arc::new(
            GovernorConfigBuilder::default()
                .per_second(60)
                .burst_size(config.post_paste_rate_limiter())
                .period(Duration::from_secs(5))
                .use_headers()
                .finish()
                .expect("Failed to build post paste limiter."),
        ),
    };

    let patch_paste_limiter = GovernorLayer {
        config: Arc::new(
            GovernorConfigBuilder::default()
                .per_second(60)
                .burst_size(config.patch_paste_rate_limiter())
                .period(Duration::from_secs(5))
                .use_headers()
                .finish()
                .expect("Failed to build patch paste limiter."),
        ),
    };

    let delete_paste_limiter = GovernorLayer {
        config: Arc::new(
            GovernorConfigBuilder::default()
                .per_second(60)
                .burst_size(config.delete_paste_rate_limiter())
                .period(Duration::from_secs(5))
                .use_headers()
                .finish()
                .expect("Failed to build delete paste limiter."),
        ),
    };

    Router::new()
        .route("/pastes", get(get_pastes).layer(get_pastes_limiter))
        .route(
            "/pastes/{paste_id}",
            get(get_paste).layer(get_paste_limiter),
        )
        .route("/pastes", post(post_paste).layer(post_paste_limiter))
        .route(
            "/pastes/{paste_id}",
            patch(patch_paste).layer(patch_paste_limiter),
        )
        .route(
            "/pastes/{paste_id}",
            delete(delete_paste).layer(delete_paste_limiter),
        )
        .layer(global_limiter)
        .layer(DefaultBodyLimit::max(
            config.global_paste_total_document_size_limit() * 1024 * 1024,
        ))
}

/// Get Paste.
///
/// Get an existing paste.
///
/// ## Path
///
/// - `paste_id` - The pastes ID.
///
/// ## Query
///
/// References: [`GetPasteQuery`]
///
/// - `content` - Whether to include the content or not.
///
/// ## Returns
///
/// - `404` - The paste was not found.
/// - `200` - The [`ResponsePaste`] object.
async fn get_paste(
    State(app): State<App>,
    Path(paste_id): Path<Snowflake>,
    Query(query): Query<GetPasteQuery>,
) -> Result<Response, AppError> {
    let paste = Paste::fetch(&app.database, paste_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Paste not found.".to_string()))?;

    if let Some(expiry) = paste.expiry {
        if expiry < OffsetDateTime::now_utc() {
            Paste::delete_with_id(&app.database, paste.id).await?;
            return Err(AppError::NotFound("Paste not found.".to_string()));
        }
    }

    let documents = Document::fetch_all(&app.database, paste.id).await?;

    let mut response_documents = Vec::new();
    for document in documents {
        let content = {
            if query.include_content {
                let data = app.s3.fetch_document(document.generate_path()).await?;
                let d: &str = &String::from_utf8_lossy(&data);
                Some(d.to_string())
            } else {
                None
            }
        };

        let response_document = ResponseDocument::from_document(document, content);

        response_documents.push(response_document);
    }

    let paste_response = ResponsePaste::from_paste(&paste, None, response_documents);

    Ok((StatusCode::OK, Json(paste_response)).into_response())
}

/// Get Pastes.
///
/// Get a list of existing pastes.
///
/// ## Body
/// 
/// An array of [`Snowflake`]'s.
///
/// ## Returns
///
/// - `200` - A list of [`ResponsePaste`] objects.
async fn get_pastes(
    State(app): State<App>,
    Json(body): Json<Vec<Snowflake>>,
) -> Result<Response, AppError> {
    let mut response_pastes: Vec<ResponsePaste> = Vec::new();

    for paste_id in body {
        let paste = Paste::fetch(&app.database, paste_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Paste not found.".to_string()))?;

        if let Some(expiry) = paste.expiry {
            if expiry < OffsetDateTime::now_utc() {
                Paste::delete_with_id(&app.database, paste.id).await?;
                return Err(AppError::NotFound("Paste not found.".to_string()));
            }
        }

        let documents = Document::fetch_all(&app.database, paste.id).await?;

        let mut response_documents = Vec::new();
        for document in documents {
            let response_document = ResponseDocument::from_document(document, None);

            response_documents.push(response_document);
        }

        let response_paste = ResponsePaste::from_paste(&paste, None, response_documents);

        response_pastes.push(response_paste);
    }

    Ok((StatusCode::OK, Json(response_pastes)).into_response())
}

/// Post Paste.
///
/// Create a new paste.
///
/// The first object in the multipart must be the body object.
///
/// The following items will be the documents.
///
/// ## Query
///
/// References: [`PostPasteQuery`]
///
/// - `content` - Whether to include the content or not.
///
/// ## Body
///
/// References: [`PostPasteBody`]
///
/// - `expiry` - The expiry of the paste.
///
/// ## Returns
///
/// - `400` - The body and/or documents are invalid.
/// - `200` - The [`ResponsePaste`] object.
#[allow(clippy::too_many_lines)]
async fn post_paste(
    State(app): State<App>,
    Query(query): Query<PostPasteQuery>,
    mut multipart: Multipart,
) -> Result<Response, AppError> {
    let body: PostPasteBody = {
        if let Some(field) = multipart.next_field().await? {
            if field
                .content_type()
                .is_none_or(|content_type| content_type != "application/json")
            {
                return Err(AppError::BadRequest(
                    "Payload must be of the type application/json.".to_string(),
                ));
            }

            let bytes = field.bytes().await?;

            serde_json::from_slice(&bytes)?
        } else {
            return Err(AppError::BadRequest("Payload missing.".to_string()));
        }
    };

    let expiry = {
        if body.expiry.is_none()
            && app.config.default_expiry_hours().is_none()
            && app.config.maximum_expiry_hours().is_some()
        {
            return Err(AppError::BadRequest(
                "A expiry time is required.".to_string(),
            ));
        } else if let Some(expiry) = body.expiry {
            let time = OffsetDateTime::from_unix_timestamp(expiry as i64)
                .map_err(|e| AppError::BadRequest(format!("Failed to build timestamp: {e}")))?;
            let now = OffsetDateTime::now_utc();
            let difference = (time - now).whole_seconds();

            if difference.is_negative() {
                return Err(AppError::BadRequest(
                    "The timestamp provided is invalid.".to_string(),
                ));
            }

            if let Some(maximum_expiry_hours) = app.config.maximum_expiry_hours() {
                if difference as usize > maximum_expiry_hours * 3600 {
                    return Err(AppError::BadRequest(
                        "The timestamp provided is above the maximum.".to_string(),
                    ));
                }
            }

            Some(time)
        } else {
            app.config
                .default_expiry_hours()
                .map(|default_expiry_time| {
                    OffsetDateTime::now_utc()
                        .saturating_add(time::Duration::hours(default_expiry_time as i64))
                })
        }
    };

    let mut transaction = app.database.pool().begin().await?;

    let paste = Paste::new(Snowflake::generate()?, false, expiry);

    paste.insert(&mut transaction).await?;

    let mut documents: Vec<(Document, String)> = Vec::new();
    while let Some(field) = multipart.next_field().await? {
        let document_type = {
            match field.content_type() {
                Some(content_type) => {
                    if contains_mime(UNSUPPORTED_MIMES, content_type) {
                        return Err(AppError::BadRequest(format!(
                            "Invalid mime type received: {content_type}"
                        )));
                    }

                    content_type.to_string()
                }
                None => DEFAULT_MIME.to_string(),
            }
        };
        let name = field.name().unwrap_or("unknown").to_string();
        let data = field.bytes().await?;

        if data.len() > (app.config.global_paste_document_size_limit() * 1024 * 1024) {
            return Err(AppError::NotFound("Document too large.".to_string()));
        }

        let document = Document::new(Snowflake::generate()?, paste.id, document_type, name);

        documents.push((document, String::from_utf8_lossy(&data).to_string()));
    }

    let final_documents: Vec<ResponseDocument> = documents
        .iter()
        .map(|(d, c)| {
            let content = {
                if query.include_content {
                    Some(c.clone())
                } else {
                    None
                }
            };

            ResponseDocument::from_document(d.clone(), content)
        })
        .collect();

    if documents.len() > app.config.global_paste_total_document_count() {
        return Err(AppError::BadRequest(
            "Too many documents provided.".to_string(),
        ));
    }

    if documents.is_empty() {
        return Err(AppError::BadRequest("No documents provided.".to_string()));
    }

    for (document, content) in documents {
        app.s3.create_document(&document, content.into()).await?;

        document.insert(&mut transaction).await?;
    }

    let paste_token = Token::new(paste.id, generate_token(paste.id)?);

    paste_token.insert(&mut transaction).await?;

    transaction.commit().await?;

    let response = ResponsePaste::from_paste(&paste, Some(paste_token), final_documents);

    Ok((StatusCode::OK, Json(response)).into_response())
}

/// Patch Paste.
///
/// Edit an existing paste.
///
/// **Requires authentication.**
async fn patch_paste(State(_app): State<App>, _token: Token) -> Result<Response, AppError> {
    Ok(StatusCode::NOT_IMPLEMENTED.into_response()) // FIXME: Make this actually work.
}

/// Delete Paste.
///
/// Delete an existing paste.
///
/// **Requires authentication.**
///
/// ## Path
///
/// - `content` - Whether to include the content or not.
///
/// ## Body
///
/// References: [`PostPasteBody`]
///
/// - `expiry` - The expiry of the paste.
///
/// ## Returns
///
/// - `401` - Invalid token and/or paste ID.
/// - `204` - Successful deletion of the paste.
async fn delete_paste(
    State(app): State<App>,
    Path(paste_id): Path<Snowflake>,
    token: Token,
) -> Result<Response, AppError> {
    if token.paste_id() != paste_id {
        return Err(AppError::Authentication(AuthError::ForbiddenPasteId));
    }

    Paste::delete_with_id(&app.database, paste_id).await?;

    Ok(StatusCode::NO_CONTENT.into_response())
}

// FIXME: This whole function needs rebuilding. I do not like the way its made.
// For example, the regex values. Can I have them as constants in any way? or are they super light when unwrapping?
// Any way to shrink the `.capture` call so that its not being called each time?
/// Contains Mime.
///
/// Checks if the mime is in the list of mimes.
///
/// If a mime in the mimes list ends with an asterisk "*",
/// at the end like `images/*` it will become a catch all,
/// making all mimes that start with `images` return true.
///
/// ## Arguments
///
/// - `mimes` - The array of mimes to check in.
/// - `value` - The value to look for.
///
/// ## Returns
///
/// True if mime was found, otherwise False.
fn contains_mime(mimes: &[&str], value: &str) -> bool {
    let match_all_mime =
        Regex::new(r"^(?P<left>[a-zA-Z0-9]+)/\*$").expect("Failed to build match all mime regex."); // checks if the mime ends with /* which indicates any of the mime type.
    let split_mime = Regex::new(r"^(?P<left>[a-zA-Z0-9]+)/(?P<right>[a-zA-Z0-9\*]+)$")
        .expect("Failed to build split mime regex."); // extracts the left and right parts of the mime.

    if let Some(split_mime_value) = split_mime.captures(value) {
        for mime in mimes {
            if mime == &value {
                return true;
            } else if let Some(capture) = match_all_mime.captures(mime) {
                if let (Some(mime_value_left), Some(capture_value_left)) =
                    (split_mime_value.name("left"), capture.name("left"))
                {
                    if mime_value_left.as_str() == capture_value_left.as_str() {
                        return true;
                    }
                }
            }
        }
    }

    false
}
