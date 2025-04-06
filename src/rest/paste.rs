use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Multipart, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, patch, post},
};
use regex::Regex;
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::{
    app::application::App,
    models::{
        authentication::{Token, generate_token},
        document::Document,
        error::{AppError, AuthError},
        paste::Paste,
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

pub fn generate_router() -> Router<App> {
    Router::new()
        .route("/pastes", get(get_pastes))
        .route("/pastes/{paste_id}", get(get_paste))
        .route("/pastes", post(post_paste))
        .route("/pastes/{paste_id}", patch(patch_paste))
        .route("/pastes/{paste_id}", delete(delete_paste))
        .layer(DefaultBodyLimit::disable())
}

async fn get_paste(
    State(app): State<App>,
    Path(paste_id): Path<Snowflake>,
    Query(query): Query<GetPasteQuery>,
) -> Result<Response, AppError> {
    let paste = Paste::fetch(&app.database, paste_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Paste not found.".to_string()))?;

    let mut response_documents = Vec::new();
    for document_id in &paste.document_ids {
        let document = Document::fetch(&app.database, *document_id)
            .await?
            .ok_or_else(|| {
                AppError::NotFound(format!("Could not find document with ID: {document_id}"))
            })?;

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

async fn get_pastes(
    State(app): State<App>,
    Json(body): Json<GetPastesBody>,
) -> Result<Response, AppError> {
    let mut response_pastes: Vec<ResponsePaste> = Vec::new();

    for paste_id in body.paste_ids {
        let paste = Paste::fetch(&app.database, paste_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Paste not found.".to_string()))?;

        let mut response_documents = Vec::new();
        for document_id in &paste.document_ids {
            let document = Document::fetch(&app.database, *document_id)
                .await?
                .ok_or_else(|| {
                    AppError::NotFound(format!("Could not find document with ID: {document_id}"))
                })?;

            let response_document = ResponseDocument::from_document(document, None);

            response_documents.push(response_document);
        }

        let response_paste = ResponsePaste::from_paste(&paste, None, response_documents);

        response_pastes.push(response_paste);
    }

    Ok((StatusCode::OK, Json(response_pastes)).into_response())
}

async fn post_paste(
    State(app): State<App>,
    Query(query): Query<PostPasteQuery>,
    mut multipart: Multipart,
) -> Result<Response, AppError> {
    let paste_id = Snowflake::generate()?;

    let mut response_documents: Vec<ResponseDocument> = Vec::new();
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

        let document_id = Snowflake::generate()?;

        let document = Document::new(document_id, paste_id, document_type, name);

        app.s3
            .create_document(document.generate_path(), data.clone())
            .await?;

        document.update(&app.database).await?;

        let content = {
            if query.include_content {
                let d: &str = &String::from_utf8_lossy(&data);
                Some(d.to_string())
            } else {
                None
            }
        };

        let response_document = ResponseDocument::from_document(document, content);

        response_documents.push(response_document);
    }

    if response_documents.is_empty() {
        return Err(AppError::BadRequest(
            "No documents were received.".to_string(),
        ));
    }

    let expiry = {
        if let Some(expiry) = query.expiry {
            tracing::trace!("Expiry found! {}", expiry);
            let time = OffsetDateTime::from_unix_timestamp(expiry as i64)?;
            let now = OffsetDateTime::now_utc();
            let difference = (time - now).whole_hours();

            if difference.is_negative() {
                return Err(AppError::NotFound("The value provided is not a valid timestamp.".to_string()));
            }

            if let Some(maximum_expiry_hours) = app.config.maximum_expiry_hours() {
                if difference as usize > maximum_expiry_hours {
                    return Err(AppError::NotFound( 
                        "The time provided is too large.".to_string(),
                    ));
                }
            }
            Some(time)
        } else {
            None
        }
    };

    let paste = Paste::new(
        paste_id,
        false,
        expiry,
        response_documents.iter().map(|d| d.id).collect(),
    );

    paste.update(&app.database).await?;

    let paste_token = Token::new(paste_id, generate_token(paste_id)?);

    paste_token.update(&app.database).await?;

    let response = ResponsePaste::from_paste(&paste, Some(paste_token), response_documents);

    Ok((StatusCode::OK, Json(response)).into_response())
}

async fn patch_paste(State(_app): State<App>, _token: Token) -> Result<Response, AppError> {
    Ok(StatusCode::NOT_IMPLEMENTED.into_response()) // FIXME: Make this actually work.
}

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

#[derive(Deserialize, Serialize)]
pub struct GetPasteQuery {
    /// Whether to return the content(s) of the documents.
    ///
    /// Defaults to False.
    #[serde(default, rename = "content")]
    pub include_content: bool,
}

#[derive(Deserialize, Serialize)]
pub struct GetPastesBody {
    /// The ID's to get.
    pub paste_ids: Vec<Snowflake>,
}

#[derive(Deserialize, Serialize)]
pub struct PostPasteQuery {
    /// Whether to return the content(s) of the documents.
    ///
    /// Defaults to false.
    #[serde(default, rename = "content")]
    pub include_content: bool,
    /// The expiry time for the paste.
    #[serde(default)]
    pub expiry: Option<usize>,
}

#[derive(Deserialize, Serialize)]
pub struct DeletePastesBody {
    /// The ID's to get.
    pub ids: Vec<Snowflake>,
}

#[derive(Deserialize, Serialize)]
pub struct ResponsePaste {
    /// The ID for the paste.
    pub id: Snowflake,
    /// The token attached to the paste.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    /// Whether the paste has been edited.
    pub edited: bool,
    /// The expiry time of the paste.
    pub expiry: Option<usize>,
    /// The documents attached to the paste.
    pub documents: Vec<ResponseDocument>,
}

impl ResponsePaste {
    pub const fn new(
        id: Snowflake,
        token: Option<String>,
        edited: bool,
        expiry: Option<usize>,
        documents: Vec<ResponseDocument>,
    ) -> Self {
        Self {
            id,
            token,
            edited,
            expiry,
            documents,
        }
    }

    pub fn from_paste(
        paste: &Paste,
        token: Option<Token>,
        documents: Vec<ResponseDocument>,
    ) -> Self {
        let token_value: Option<String> = { token.map(|t| t.token().expose_secret().to_string()) };

        let expiry = paste.expiry.map(|v| v.unix_timestamp() as usize);

        Self::new(paste.id, token_value, paste.edited, expiry, documents)
    }
}

#[derive(Deserialize, Serialize)]
pub struct ResponseDocument {
    /// The ID for the document.
    pub id: Snowflake,
    /// The paste ID the document is attached too.
    pub paste_id: Snowflake,
    /// The type of document.
    #[serde(rename = "type")]
    pub document_type: String,
    /// The name of the document.
    pub name: String,
    /// The content of the document.
    pub content: Option<String>,
}

impl ResponseDocument {
    pub const fn new(
        id: Snowflake,
        paste_id: Snowflake,
        document_type: String,
        name: String,
        content: Option<String>,
    ) -> Self {
        Self {
            id,
            paste_id,
            document_type,
            name,
            content,
        }
    }

    pub fn from_document(document: Document, content: Option<String>) -> Self {
        Self::new(
            document.id,
            document.paste_id,
            document.document_type,
            document.name,
            content,
        )
    }
}

// FIXME: This whole function needs rebuilding. I do not like the way its made.
// For example, the regex values. Can I have them as constants in any way? or are they super light when unwrapping?
// Any way to shrink the `.capture` call so that its not being called each time?
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
