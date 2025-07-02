use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, State},
    response::{IntoResponse, Response},
    routing::{delete, get, patch, post},
};
use axum_extra::{
    TypedHeader,
    headers::{self, ContentType, Header},
};
use bytes::Bytes;
use http::{HeaderName, HeaderValue, StatusCode};

use crate::{
    app::{application::App, config::Config},
    models::{
        authentication::Token,
        document::{
            DEFAULT_MIME, Document, UNSUPPORTED_MIMES, contains_mime, document_limits,
            total_document_limits,
        },
        error::{AppError, AuthError},
        paste::{Paste, validate_paste},
        snowflake::Snowflake,
    },
};

pub fn generate_router(config: &Config) -> Router<App> {
    Router::new()
        .route(
            "/pastes/{paste_id}/documents/{document_id}",
            get(get_document),
        )
        .route("/pastes/{paste_id}/documents", post(post_document))
        .route(
            "/pastes/{paste_id}/documents/{document_id}",
            patch(patch_document),
        )
        .route(
            "/pastes/{paste_id}/documents/{document_id}",
            delete(delete_document),
        )
        .layer(DefaultBodyLimit::max(
            config.size_limits().maximum_total_document_size(),
        ))
}

/// Get Document.
///
/// Get an existing document.
///
/// ## Path
///
/// - `paste_id` - The pastes ID.
/// - `document_id` - The documents ID.
///
/// ## Returns
///
/// - `404` - The paste or document was not found.
/// - `200` - The [`ResponseDocument`] object.
async fn get_document(
    State(app): State<App>,
    Path((paste_id, document_id)): Path<(Snowflake, Snowflake)>,
) -> Result<Response, AppError> {
    let document = Document::fetch(app.database().pool(), &document_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Document not found.".to_string()))?;

    if document.paste_id() != &paste_id {
        return Err(AppError::BadRequest(
            "The document ID does not belong to that paste.".to_string(),
        ));
    }

    Paste::add_view(app.database().pool(), &paste_id).await?;

    Ok((StatusCode::OK, Json(document)).into_response())
}

/// Post Document.
///
/// Adds a document to an existing paste.
///
/// ## Body
///
/// The body of the document, will be the content of the document.
///
/// ## Returns
///
/// - `401` - Invalid token and/or paste ID.
/// - `400` - The body and/or documents are invalid.
/// - `200` - The [`ResponseDocument`].
/// - `204` - If content is set to false.
async fn post_document(
    State(app): State<App>,
    Path(paste_id): Path<Snowflake>,
    TypedHeader(content_disposition): TypedHeader<ContentDisposition>,
    content_type: Option<TypedHeader<ContentType>>,
    token: Token,
    body: Bytes,
) -> Result<Response, AppError> {
    let mut paste = validate_paste(app.database(), &paste_id, Some(token)).await?;

    let document_type = {
        if let Some(TypedHeader(content_type)) = content_type {
            if contains_mime(UNSUPPORTED_MIMES, &content_type.to_string()) {
                return Err(AppError::BadRequest(format!(
                    "Invalid mime type received: {content_type}"
                )));
            }

            content_type.to_string()
        } else {
            DEFAULT_MIME.to_string()
        }
    };

    let name = content_disposition.filename().unwrap_or("unknown");

    let document = Document::new(
        Snowflake::generate()?,
        *paste.id(),
        &document_type,
        name,
        body.len(),
    );

    document_limits(app.config(), &document)?;

    let mut transaction = app.database().pool().begin().await?;

    paste.set_edited();

    paste.update(transaction.as_mut()).await?;

    document.insert(transaction.as_mut()).await?;

    total_document_limits(&mut transaction, app.config(), &paste_id).await?;

    app.s3().create_document(&document, body).await?;

    transaction.commit().await?;

    Ok((StatusCode::OK, Json(document)).into_response())
}

/// Patch Document.
///
/// Adds a document to an existing paste.
///
/// ## Path
///
/// - `paste_id` - The paste ID of the document.
/// - `document_id` - The document ID to edit.
///
/// ## Body
///
/// The body of the document, will be the content of the document.
///
/// ## Returns
///
/// - `401` - Invalid token and/or paste ID.
/// - `400` - The body and/or documents are invalid.
/// - `200` - The [`ResponseDocument`].
/// - `204` - If content is set to false.
async fn patch_document(
    State(app): State<App>,
    Path((paste_id, document_id)): Path<(Snowflake, Snowflake)>,
    TypedHeader(content_disposition): TypedHeader<ContentDisposition>,
    content_type: Option<TypedHeader<ContentType>>,
    token: Token,
    body: Bytes,
) -> Result<Response, AppError> {
    let mut paste = validate_paste(app.database(), &paste_id, Some(token)).await?;

    let document_type = {
        if let Some(TypedHeader(content_type)) = content_type {
            if contains_mime(UNSUPPORTED_MIMES, &content_type.to_string()) {
                return Err(AppError::BadRequest(format!(
                    "Invalid mime type received: {content_type}"
                )));
            }

            content_type.to_string()
        } else {
            DEFAULT_MIME.to_string()
        }
    };

    let mut document = Document::fetch(app.database().pool(), &document_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Document not found.".to_string()))?;

    let mut transaction = app.database().pool().begin().await?;

    paste.set_edited();

    paste.update(transaction.as_mut()).await?;

    document.set_doc_type(&document_type);

    document.set_size(body.len());

    if let Some(filename) = content_disposition.filename() {
        document.set_name(filename);
    }

    document_limits(app.config(), &document)?;

    document.update(transaction.as_mut()).await?;

    total_document_limits(&mut transaction, app.config(), &paste_id).await?;

    app.s3().delete_document(document.generate_path()).await?;

    app.s3().create_document(&document, body).await?;

    transaction.commit().await?;

    Ok((StatusCode::OK, Json(document)).into_response())
}

/// Patch Document.
///
/// Adds a document to an existing paste.
///
/// ## Path
///
/// - `paste_id` - The paste ID of the document.
/// - `document_id` - The document ID to delete.
///
/// ## Returns
///
/// - `401` - Invalid token and/or paste ID.
/// - `400` - A paste must have at least one document.
/// - `204` - Successful deletion of the document.
async fn delete_document(
    State(app): State<App>,
    Path((paste_id, document_id)): Path<(Snowflake, Snowflake)>,
    token: Token,
) -> Result<Response, AppError> {
    if token.paste_id() != paste_id {
        return Err(AppError::Authentication(AuthError::ForbiddenPasteId));
    }

    let total_document_count =
        Document::fetch_total_document_count(app.database().pool(), &paste_id).await?;

    if total_document_count <= 1 {
        return Err(AppError::BadRequest(
            "A paste must have at least one document".to_string(),
        ));
    }

    if !Document::delete(app.database().pool(), &document_id).await? {
        return Err(AppError::NotFound(
            "The document was not found.".to_string(),
        ));
    }

    Ok(StatusCode::NO_CONTENT.into_response())
}

#[derive(Debug, Clone)]
struct ContentDisposition {
    disposition: String,
    filename: Option<String>,
}

impl ContentDisposition {
    #[allow(dead_code)]
    pub fn disposition(&self) -> &str {
        &self.disposition
    }

    pub fn filename(&self) -> Option<&str> {
        self.filename.as_deref()
    }
}

impl Header for ContentDisposition {
    fn name() -> &'static HeaderName {
        &axum::http::header::CONTENT_DISPOSITION
    }

    fn decode<'i, I>(values: &mut I) -> Result<Self, headers::Error>
    where
        I: Iterator<Item = &'i HeaderValue>,
    {
        let value = values.next().ok_or_else(headers::Error::invalid)?;

        let s = value.to_str().map_err(|_| headers::Error::invalid())?;

        let mut disposition = String::new();
        let mut filename = None;

        for (i, part) in s.split(';').enumerate() {
            let part = part.trim();
            if i == 0 {
                disposition = part.to_string();
            } else if let Some(rest) = part.strip_prefix("filename=") {
                filename = Some(rest.trim_matches('"').to_string());
            }
        }

        Ok(Self {
            disposition,
            filename,
        })
    }

    fn encode<E>(&self, values: &mut E)
    where
        E: Extend<HeaderValue>,
    {
        let mut parts = vec![self.disposition.clone()];

        if let Some(filename) = &self.filename {
            parts.push(format!("filename=\"{filename}\""));
        }

        let full = parts.join("; ");

        if let Ok(header_value) = HeaderValue::from_str(&full) {
            values.extend(std::iter::once(header_value));
        }
    }
}
