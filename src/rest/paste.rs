use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Multipart, Query, State},
    response::{IntoResponse, Response},
    routing::{delete, get, patch, post},
};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

use crate::{
    app::app::App,
    models::{document::{Document, DocumentType}, error::AppError, paste::Paste, snowflake::Snowflake},
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

    let mut documents = Vec::new();
    for document_id in paste.document_ids {
        let document = Document::fetch(&app.database, document_id).await?.ok_or_else(|| AppError::NotFound(format!("Could not find document with ID: {document_id}")))?;
        
        let content = {
            if query.request_content {
                let data = app.s3.fetch_document(document.id).await?;
                let d: &str = &String::from_utf8_lossy(&data);
                Some(d.to_string())
            } else {
                None
            }
        };

        let response_document = PostPasteResponseDocument {
            id: document.id,
            token: document.token,
            paste_id: document.paste_id,
            document_type: document.doc_type,
            content
        };

        documents.push(response_document)
    }

    let paste_response = PostPasteResponse {
        id: paste.id,
        token: paste.owner_token,
        documents
    };

    Ok((StatusCode::OK, Json(paste_response)).into_response())
}

#[derive(Deserialize, Serialize)]
pub struct GetPasteQuery {
    /// The ID for the paste to retrieve.
    paste_id: Snowflake,
    /// Whether to return the content of the documents.
    request_content: bool
}

async fn get_pastes(
    State(app): State<App>,
    Json(body): Json<GetPastesBody>,
    //_: Token
) -> Result<Response, AppError> {
    let mut pastes = Vec::new();

    for paste in body.pastes {
        let paste = Paste::fetch(&app.database, paste)
            .await?
            .ok_or_else(|| AppError::NotFound("Paste not found.".to_string()))?;
        
        pastes.push(paste)
    }

    Ok((StatusCode::OK, Json(pastes)).into_response())
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
    tracing::debug!("Here 1");
    let paste_id = Snowflake::generate(&app.database).await?;

    let mut documents: Vec<PostPasteResponseDocument> = Vec::new();
    while let Some(field) = multipart.next_field().await? {
        tracing::debug!("Here 2");
        if field
            .content_type()
            .is_some_and(|v| vec!["text/plain"].contains(&v)) {
                let name = field.name().unwrap_or("unknown").to_string();
                let headers = field.headers().clone();
                let data = field.bytes().await?;
                tracing::debug!("Here 3 ({})", name.clone());
                let document_id = Snowflake::generate(&app.database).await?;

                let document_type = match headers.get("") {
                    Some(h) => {
                        let val: &str = &String::from_utf8_lossy(h.as_bytes());
                        DocumentType::from(val.to_string())
                    }
                    None => {
                        let (_, document_type): (&str, &str) = name.rsplit_once(".").unwrap_or(("", "unknown"));
                        DocumentType::from_file_type(document_type.to_string())
                    }
                };

                tracing::debug!("Here 4 ({})", name.clone());

                app.s3.create_document(document_id, data.clone()).await?;
                
                tracing::debug!("Here 5 ({})", name.clone());

                let document = Document::new(
                    document_id,
                    String::new(), // FIXME: This should use the token that owns this paste.
                    paste_id,
                    document_type
                );

                document.update(&app.database).await?;

                tracing::debug!("Here 6 ({})", name.clone());
                
                let response_document = PostPasteResponseDocument{
                    id: document.id,
                    token: String::new(), // FIXME: This should use the token that owns this paste.
                    paste_id: paste_id,
                    document_type: document.doc_type,
                    content: {
                        if query.request_content {
                            let d: &str = &String::from_utf8_lossy(&data);
                            Some(d.to_string())
                        } else {
                            None
                        } 
                    }
                };

                tracing::debug!("Here 7 ({})", name.clone());

                documents.push(response_document);
            }
    }

    tracing::debug!("Here 8");

    let paste = Paste::new(
        paste_id,
        String::new(), // FIXME: This should use the token that owns this paste.
        documents.iter().map(|d|d.id).collect()
    );

    paste.update(&app.database).await?;

    tracing::debug!("Here 9");

    let response = PostPasteResponse {
        id: paste_id,
        token: String::new(), // FIXME: This should use the token that owns this paste.
        documents
    };

    Ok((StatusCode::OK, Json(response)).into_response())
}

#[derive(Deserialize, Serialize)]
pub struct PostPasteQuery {
    /// Whether to return the content.
    pub request_content: bool
}

#[derive(Deserialize, Serialize)]
pub struct PostPasteResponse {
    /// The ID for the paste.
    pub id: Snowflake,
    /// The token that created the paste.
    pub token: String,
    /// The documents attached to the paste.
    pub documents: Vec<PostPasteResponseDocument>
}

#[derive(Deserialize, Serialize)]
pub struct PostPasteResponseDocument {
    /// The ID for the document.
    pub id: Snowflake,
    /// The token that created the document.
    pub token: String,
    /// The paste ID the document is attached too.
    pub paste_id: Snowflake,
    /// The type of document.
    #[serde(rename = "type")]
    pub document_type: DocumentType,
    /// The content of the document.
    pub content: Option<String>
}

async fn patch_paste(
    State(app): State<App>,
    //_: Token,
) -> Result<Response, AppError> {
    todo!("Implement me!")
}

async fn delete_paste(
    State(app): State<App>,
    //_: Token
) -> Result<Response, AppError> {
    todo!("Implement me!")
}

async fn delete_pastes(
    State(app): State<App>,
    //_: Token
) -> Result<Response, AppError> {
    todo!("Implement me!")
}
