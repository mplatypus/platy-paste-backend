use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Multipart, Query, State},
    response::{IntoResponse, Response},
    routing::{delete, get, patch, post},
};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

use crate::{
    app::application::App,
    models::{
        document::{Document, DocumentType},
        error::AppError,
        paste::Paste,
        snowflake::Snowflake,
    },
};

pub fn generate_router() -> Router<App> {
    Router::new()
        .route("/paste", get(get_paste))
        .route("/paste/all", get(get_pastes))
        .route("/paste", patch(patch_paste))
        .route("/paste", post(post_paste))
        .route("/paste", delete(delete_paste))
        .route("/paste/all", delete(delete_pastes))
        .layer(DefaultBodyLimit::disable())
}

async fn get_paste(
    State(app): State<App>,
    Query(query): Query<GetPasteQuery>,
    //_: Token
) -> Result<Response, AppError> {
    let paste = Paste::fetch(&app.database, query.paste_id)
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
            if query.request_content {
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

    let paste_response = ResponsePaste::from_paste(&paste, response_documents);

    Ok((StatusCode::OK, Json(paste_response)).into_response())
}

#[derive(Deserialize, Serialize)]
pub struct GetPasteQuery {
    /// The ID for the paste to retrieve.
    paste_id: Snowflake,
    /// Whether to return the content of the documents.
    request_content: bool,
}

async fn get_pastes(
    State(app): State<App>,
    Json(body): Json<GetPastesBody>,
    //_: Token
) -> Result<Response, AppError> {
    let mut response_pastes: Vec<ResponsePaste> = Vec::new();

    for paste_id in body.pastes {
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

        let response_paste = ResponsePaste::from_paste(&paste, response_documents);

        response_pastes.push(response_paste);
    }

    Ok((StatusCode::OK, Json(response_pastes)).into_response())
}

#[derive(Deserialize, Serialize)]
pub struct GetPastesBody {
    /// The ID's to get.
    pub pastes: Vec<Snowflake>,
}

async fn post_paste(
    State(app): State<App>,
    Query(query): Query<PostPasteQuery>,
    mut multipart: Multipart,
) -> Result<Response, AppError> {
    let paste_id = Snowflake::generate(&app.database)?;

    let mut response_documents: Vec<ResponseDocument> = Vec::new();
    while let Some(field) = multipart.next_field().await? {
        if field
            .content_type()
            .is_some_and(|v| ["text/plain"].contains(&v))
        {
            let name = field.name().unwrap_or("unknown").to_string();
            let headers = field.headers().clone();
            let data = field.bytes().await?;
            let document_id = Snowflake::generate(&app.database)?;

            let document_type = headers.get("").map_or_else(
                || {
                    let (_, document_type): (&str, &str) =
                        name.rsplit_once('.').unwrap_or(("", "unknown"));
                    DocumentType::from_file_type(document_type)
                },
                |h| {
                    let val: &str = &String::from_utf8_lossy(h.as_bytes());
                    DocumentType::from(val.to_string())
                },
            );

            let document = Document::new(document_id, paste_id, document_type);

            app.s3
                .create_document(document.generate_path(), data.clone())
                .await?;

            document.update(&app.database).await?;

            let content = {
                if query.request_content {
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
        None,
        None,
        response_documents.iter().map(|d| d.id).collect(),
    );

    paste.update(&app.database).await?;

    let response = ResponsePaste::from_paste(&paste, response_documents);

    Ok((StatusCode::OK, Json(response)).into_response())
}

#[derive(Deserialize, Serialize)]
pub struct PostPasteQuery {
    /// Whether to return the content.
    pub request_content: bool,
}

#[derive(Deserialize, Serialize)]
pub struct ResponsePaste {
    /// The ID for the paste.
    pub id: Snowflake,
    /// The user that created this paste.
    pub owner_id: Option<Snowflake>,
    /// The token that created the paste.
    pub owner_token: Option<String>,
    /// The documents attached to the paste.
    pub documents: Vec<ResponseDocument>,
}

impl ResponsePaste {
    pub const fn new(
        id: Snowflake,
        owner_id: Option<Snowflake>,
        owner_token: Option<String>,
        documents: Vec<ResponseDocument>,
    ) -> Self {
        Self {
            id,
            owner_id,
            owner_token,
            documents,
        }
    }

    pub fn from_paste(paste: &Paste, documents: Vec<ResponseDocument>) -> Self {
        let owner_id = {
            if paste.owner_token.is_some() {
                None
            } else {
                paste.owner_id
            }
        };

        Self::new(paste.id, owner_id, paste.owner_token.clone(), documents)
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
    pub document_type: DocumentType,
    /// The content of the document.
    pub content: Option<String>,
}

impl ResponseDocument {
    pub const fn new(
        id: Snowflake,
        paste_id: Snowflake,
        document_type: DocumentType,
        content: Option<String>,
    ) -> Self {
        Self {
            id,
            paste_id,
            document_type,
            content,
        }
    }

    pub fn from_document(document: Document, content: Option<String>) -> Self {
        Self::new(document.id, document.paste_id, document.doc_type, content)
    }
}

async fn patch_paste(
    State(_app): State<App>,
    //_: Token,
) -> Result<Response, AppError> {
    todo!("Implement me!")
}

async fn delete_paste(
    State(_app): State<App>,
    //_: Token
) -> Result<Response, AppError> {
    todo!("Implement me!")
}

async fn delete_pastes(
    State(_app): State<App>,
    //_: Token
) -> Result<Response, AppError> {
    todo!("Implement me!")
}
