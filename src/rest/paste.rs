use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Multipart, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, patch, post},
};
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};

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
        if field
            .content_type()
            .is_some_and(|v| ["text/plain"].contains(&v))
        {
            let name = field.name().unwrap_or("unknown").to_string();
            let headers = field.headers().clone();
            let data = field.bytes().await?;

            let document_id = Snowflake::generate()?;

            let document_type = headers.get("").map_or_else(
                || {
                    let (_, document_type): (&str, &str) =
                        name.rsplit_once('.').unwrap_or(("", "unknown"));
                    document_type.to_string()
                },
                |h| {
                    let val: &str = &String::from_utf8_lossy(h.as_bytes());
                    val.to_string()
                },
            );

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
    }

    let paste = Paste::new(
        paste_id,
        false,
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
        return Err(AppError::Authentication(AuthError::InvalidToken)); // FIXME: This might need changing.
    }

    Paste::delete_with_id(&app.database, paste_id).await?;

    Ok(StatusCode::NO_CONTENT.into_response())
}

const fn _const_false() -> bool {
    false
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
    /// The documents attached to the paste.
    pub documents: Vec<ResponseDocument>,
}

impl ResponsePaste {
    pub const fn new(
        id: Snowflake,
        token: Option<String>,
        edited: bool,
        documents: Vec<ResponseDocument>,
    ) -> Self {
        Self {
            id,
            token,
            edited,
            documents,
        }
    }

    pub fn from_paste(
        paste: &Paste,
        token: Option<Token>,
        documents: Vec<ResponseDocument>,
    ) -> Self {
        let token_value: Option<String> = { token.map(|t| t.token().expose_secret().to_string()) };

        Self::new(paste.id, token_value, paste.edited, documents)
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
